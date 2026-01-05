use color_print::cprintln;
use std::error::Error;
use tonic::Request;
pub mod message {
    tonic::include_proto!("message");
}
use message::message_service_client::MessageServiceClient;
use message::{SendMessageRequest, LClock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    let args: Vec<String> = args.into_iter().filter(|arg| arg != "--release").collect();

    let address = if args.len() > 1 {
        if args[1].starts_with("http://") {
            args[1].clone()
        } else {
            format!("http://{}", args[1])
        }
    } else {
        "http://127.0.0.1:50051".to_string()
    };
    let mut client = MessageServiceClient::connect(address).await?;
    cprintln!("<blue>*attack*</blue> sending normal message with ts=10");
    let msg = SendMessageRequest {
        clock: Some(LClock { sender: 99, timestamp: 10 }),
        content: "hello, i am evil!".to_string(),
    };
    match client.send_message(Request::new(msg)).await {
        Ok(_) => cprintln!("<green>*GOOD*</green> ok"),
        Err(e) => cprintln!("<red>*BAD*</red> error: {}", e),
    }
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    cprintln!("\n<blue>*attack*</blue> trying to reuse ts=10");
    let msg = SendMessageRequest {
        clock: Some(LClock { sender: 99, timestamp: 10 }),
        content: "rewrite history".to_string(),
    };
    match client.send_message(Request::new(msg)).await {
        Ok(_) => cprintln!("<red>*BAD*</red> accepted reused timestamp"),
        Err(_) => cprintln!("<green>*GOOD*</green> rejected"),
    }
    cprintln!("\n<blue>*attack*</blue> trying smaller ts=5");
    let msg = SendMessageRequest {
        clock: Some(LClock { sender: 99, timestamp: 8 }),
        content: "go back in time".to_string(),
    };
    match client.send_message(Request::new(msg)).await {
        Ok(_) => cprintln!("<red>*BAD*</red> accepted old timestamp"),
        Err(_) => cprintln!("<green>*GOOD*</green> rejected"),
    }
    cprintln!("\n<blue>*attack*</blue> trying future ts=1000");
    let msg = SendMessageRequest {
        clock: Some(LClock { sender: 99, timestamp: 1000 }),
        content: "from the future".to_string(),
    };
    match client.send_message(Request::new(msg)).await {
        Ok(_) => cprintln!("<red>*BAD*</red> accepted future timestamp"),
        Err(_) => cprintln!("<green>*GOOD*</green> rejected"),
    }
    cprintln!("\n<blue>*attack*</blue> sending valid next ts=12");
    let msg = SendMessageRequest {
        clock: Some(LClock { sender: 99, timestamp: 12 }),
        content: "valid next".to_string(),
    };
    match client.send_message(Request::new(msg)).await {
        Ok(_) => cprintln!("<green>*GOOD*</green> ok"),
        Err(e) => cprintln!("<red>*BAD*</red> error: {}", e),
    }
    Ok(())
}
