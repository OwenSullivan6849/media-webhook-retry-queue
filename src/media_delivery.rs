use crate::webhook_queue::{message_ids, new_delivery_id, send_webhook, InfraiClient};
use std::env;

pub fn enqueue_media_event(client: &InfraiClient, event_json: &str) -> Result<(), String> {
    let delivery_id = new_delivery_id();
    client.queue_publish(event_json, &delivery_id)?;
    println!("queued media delivery {delivery_id}");
    Ok(())
}

pub fn deliver_one_batch(client: &InfraiClient) -> Result<(), String> {
    let webhook_url = env::var("WEBHOOK_URL").map_err(|_| "set WEBHOOK_URL".to_string())?;
    let batch = client.queue_consume(10, 30)?;
    for message_id in message_ids(&batch) {
        // Acknowledge only after the receiver accepts the delivery.
        send_webhook(&webhook_url, &batch)?;
        client.queue_ack(&message_id)?;
        println!("delivered {message_id}");
    }
    Ok(())
}
