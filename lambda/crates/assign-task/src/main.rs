use std::env;

use aws_lambda_events::event::ses::SimpleEmailEvent;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use log::LevelFilter;
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();
    let processor = service_fn(handler);
    lambda_runtime::run(processor).await?;
    Ok(())
}

async fn handler(event: LambdaEvent<SimpleEmailEvent>) -> Result<(), Error> {
    log::info!("Processing event...");
    let bucket_name = env::var("S3BUCKET")?;
    let key_prefix = env::var("KEY_PREFIX")?;
    let shared_config = aws_config::load_from_env().await;
    let s3 = aws_sdk_s3::Client::new(&shared_config);
    let record = &event.payload.records[0];
    if let Some(msg_id) = &record.ses.mail.message_id {
        let object_key = format!("{key_prefix}{msg_id}");
        log::info!("{object_key}");
        let email = s3
            .get_object()
            .bucket(&bucket_name)
            .key(&object_key)
            .response_content_type("text/plain")
            .send()
            .await?;
        let data = email.body.collect().await?;
        let message = String::from_utf8(data.into_bytes().to_vec())?;
        log::info!("{message}");
    }
    if let Some(subject) = &record.ses.mail.common_headers.subject {
        log::info!("message: {subject}");
    }
    Ok(())
}
