use std::env;

use aws_lambda_events::event::ses::SimpleEmailEvent;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use log::LevelFilter;
use once_cell::sync::OnceCell;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, StatusCode,
};
use simple_logger::SimpleLogger;

use email_notion::email::parse_email;
use email_notion::notion::*;

static USERS: OnceCell<Vec<UserData>> = OnceCell::new();
static CLIENT: OnceCell<Client> = OnceCell::new();

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();
    let mut headers = HeaderMap::new();
    let token = format!("Bearer {}", env::var("NOTION_TOKEN").unwrap());
    let mut token_value = HeaderValue::from_str(&token).unwrap();
    token_value.set_sensitive(true);
    headers.insert("AUTHORIZATION", token_value);
    headers.insert("CONTENT-TYPE", "application/json".parse().unwrap());
    headers.insert("Notion-Version", "2021-08-16".parse().unwrap());
    let builder = Client::builder().default_headers(headers);
    let client = builder.build().unwrap();
    let resp = client
        .get("https://api.notion.com/v1/users")
        .send()
        .await?
        .text()
        .await?;
    let users = serde_json::from_str::<NotionApiUserResponse>(&resp)?.results;
    USERS.get_or_init(|| users);
    CLIENT.get_or_init(|| client);
    let processor = service_fn(handler);
    lambda_runtime::run(processor).await?;
    Ok(())
}

async fn handler(event: LambdaEvent<SimpleEmailEvent>) -> Result<(), Error> {
    let bucket_name = env::var("S3BUCKET")?;
    let key_prefix = env::var("KEY_PREFIX")?;
    let shared_config = aws_config::load_from_env().await;
    let s3 = aws_sdk_s3::Client::new(&shared_config);
    let record = &event.payload.records[0];
    if let Some(msg_id) = &record.ses.mail.message_id {
        let object_key = format!("{key_prefix}{msg_id}");
        let saved_email = s3
            .get_object()
            .bucket(&bucket_name)
            .key(&object_key)
            .response_content_type("text/plain")
            .send()
            .await?;
        let data = saved_email.body.collect().await?;
        let bytes = data.into_bytes().to_vec();
        let email = parse_email(&bytes)?;
        let users = USERS.get().unwrap();
        let assign_id: String;
        match users
            .iter()
            .find(|&i| i.person.as_ref().unwrap().email == email.from)
        {
            Some(user) => {
                assign_id = String::from(&user.id);
            }
            None => panic!("{} is not in our Notion Workspace!", email.from),
        }
        // Using plain text version
        let body = email.body;
        let subject = email.subject;
        let paragraphs = body
            .split('\n')
            .collect::<Vec<&str>>()
            .iter()
            .filter_map(|substr| {
                if !substr.is_empty() {
                    Some(BlockData::new(substr.to_string()))
                } else {
                    None
                }
            })
            .collect::<Vec<BlockData>>();
        // In case you didn't know, Notion's data model is verbose.
        let task_data = TaskData {
            parent: TypedData::database(env::var("DATABASE_ID").unwrap()),
            properties: Properties {
                name: TypedData::title(TypedData::text(TextContent { content: subject })),
                assign: TypedData::people(PersonData {
                    id: assign_id,
                    data_type: String::from("person"),
                    person: UserEmail { email: email.from },
                }),
            },
            children: paragraphs,
        };
        let req_body = serde_json::to_string(&task_data).unwrap();
        let client = CLIENT.get().unwrap();
        let response = client
            .post("https://api.notion.com/v1/pages")
            .body(req_body)
            .send()
            .await?;
        if response.status().eq(&StatusCode::OK) {
            log::info!("Task created successfully!")
        } else {
            let error_message = response.text().await?;
            log::warn!("{error_message}");
        }
    }
    Ok(())
}
