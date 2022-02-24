use std::env;

use aws_lambda_events::event::ses::SimpleEmailEvent;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use log::LevelFilter;
use mailparse::*;
use once_cell::sync::OnceCell;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::Deserialize;
use simple_logger::SimpleLogger;

static USERS: OnceCell<Vec<UserData>> = OnceCell::new();

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
    headers.insert("CONTENT_TYPE", "application/json".parse().unwrap());
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
        log::info!("Task from: {assign_addr}");
        let users = USERS.get().unwrap();
        if let Some(user) = users
            .iter()
            .find(|&i| i.person.as_ref().unwrap().email == assign_addr)
        {
            let assign_id = &user.id;
            log::info!("ID of user: {assign_id}");
        }
        // Using plain text version
        let body = email.subparts[0].get_body()?;
        log::info!("Note: {body}");
        let subject = email.headers.get_first_value("Subject").unwrap();
        log::info!("Task name: {subject}");
    }
    Ok(())
}

#[derive(Deserialize)]
struct UserData {
    id: String,
    person: Option<UserEmail>,
    name: String,
}

#[derive(Deserialize)]
struct UserEmail {
    email: String,
}

#[derive(Deserialize)]
struct NotionApiUserResponse {
    results: Vec<UserData>,
}
