use std::env;

use aws_lambda_events::event::ses::SimpleEmailEvent;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use log::LevelFilter;
use mailparse::*;
use once_cell::sync::OnceCell;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, StatusCode,
};
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;

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
        let email = parse_mail(&bytes)?;
        let assign_addr: String;
        match &addrparse_header(email.headers.get_first_header("From").unwrap())?[0] {
            MailAddr::Single(info) => {
                assign_addr = info.addr.to_string();
            }
            _ => panic!(),
        }
        let users = USERS.get().unwrap();
        let assign_id: String;
        match users
            .iter()
            .find(|&i| i.person.as_ref().unwrap().email == assign_addr)
        {
            Some(user) => {
                assign_id = String::from(&user.id);
            }
            None => panic!("{assign_addr} is not in our Notion Workspace!"),
        }
        // Using plain text version
        let body = email.subparts[0].get_body()?;
        let subject = email.headers.get_first_value("Subject").unwrap();
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
                    person: UserEmail { email: assign_addr },
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

#[derive(Deserialize)]
struct UserData {
    id: String,
    person: Option<UserEmail>,
}

#[derive(Deserialize, Serialize)]
struct UserEmail {
    email: String,
}

#[derive(Deserialize)]
struct NotionApiUserResponse {
    results: Vec<UserData>,
}

#[derive(Serialize)]
struct PersonData {
    id: String,
    #[serde(rename = "type")]
    data_type: String,
    person: UserEmail,
}

#[derive(Serialize)]
struct TextContent {
    content: String,
}

#[derive(Serialize)]
struct TypedData {
    #[serde(rename = "type")]
    data_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    database_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    people: Option<Vec<PersonData>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<TextContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<Vec<TypedData>>,
}

#[derive(Serialize)]
struct Properties {
    #[serde(rename = "Name")]
    name: TypedData,
    #[serde(rename = "Assign")]
    assign: TypedData,
}

#[derive(Serialize)]
struct ParagraphContent {
    text: Vec<TypedData>,
}

#[derive(Serialize)]
struct BlockData {
    object: String,
    #[serde(rename = "type")]
    data_type: String,
    paragraph: ParagraphContent,
}

impl BlockData {
    fn new(text_content: String) -> Self {
        BlockData {
            object: String::from("block"),
            data_type: String::from("paragraph"),
            paragraph: ParagraphContent {
                text: vec![TypedData::text(TextContent {
                    content: text_content,
                })],
            },
        }
    }
}

#[derive(Serialize)]
struct TaskData {
    parent: TypedData,
    properties: Properties,
    children: Vec<BlockData>,
}

impl TypedData {
    fn new() -> Self {
        TypedData {
            data_type: String::from("none"),
            database_id: None,
            people: None,
            text: None,
            title: None,
        }
    }

    fn database(id: String) -> Self {
        let mut data = TypedData::new();
        data.data_type = String::from("database_id");
        data.database_id = Some(id);
        data
    }

    fn people(people: PersonData) -> Self {
        let mut data = TypedData::new();
        data.data_type = String::from("people");
        data.people = Some(vec![people]);
        data
    }

    fn text(text: TextContent) -> Self {
        let mut data = TypedData::new();
        data.data_type = String::from("text");
        data.text = Some(text);
        data
    }

    fn title(title: TypedData) -> Self {
        let mut data = TypedData::new();
        data.data_type = String::from("title");
        data.title = Some(vec![title]);
        data
    }
}
