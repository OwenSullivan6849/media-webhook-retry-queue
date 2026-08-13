mod media_delivery;
mod webhook_queue;

use media_delivery::{deliver_one_batch, enqueue_media_event};
use std::env;
use webhook_queue::InfraiClient;

fn main() -> Result<(), String> {
    let client = InfraiClient::from_env()?;
    match env::args().nth(1).as_deref() {
        Some("enqueue") => {
            let event = env::args().nth(2).ok_or("pass a media event as JSON")?;
            enqueue_media_event(&client, &event)
        }
        Some("worker") => deliver_one_batch(&client),
        _ => Err("run: enqueue '<json>' or worker".to_string()),
    }
}
