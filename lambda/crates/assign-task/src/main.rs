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
    log::info!("Environment: bucket = {bucket_name}, prefix = {key_prefix}");
    for record in event.payload.records {
        let all_recipients = record.ses.receipt.recipients;
        log::info!("recipients: {all_recipients:?}");
        if let Some(subject) = record.ses.mail.common_headers.subject {
            log::info!("message: {subject}");
        }
        if let Some(msg_id) = record.ses.mail.message_id {
            log::info!("message ID: {msg_id}");
        }
        if let Some(action_type) = record.ses.receipt.action.type_ {
            log::info!("Action type: {action_type}");
        }
    }
    Ok(())
}
