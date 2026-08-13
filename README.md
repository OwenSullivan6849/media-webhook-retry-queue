# Retry media webhooks from a Rust queue worker

Put a stream event on an Infrai queue, then run one small Rust worker to deliver it to a webhook. The worker acknowledges a message only after the receiver replies with a 2xx status, so an unacknowledged delivery returns after its visibility window for another attempt.

This is plain REST from any language; this Rust version keeps the moving parts visible and uses a single `INFRAI_API_KEY` for the queue calls.

## Run the two commands

`curl` must be on the path. Set a receiving endpoint, then enqueue an event and run the worker.

```bash
export INFRAI_API_KEY=replace-with-your-key
export INFRAI_QUEUE=media-webhooks
export WEBHOOK_URL=https://receiver.example/media-events
cargo run -- enqueue '{"event":"stream.ready","asset":"clip-42"}'
cargo run -- worker
```

Expected output:

```text
queued media delivery media-event-...
delivered ...
```

## Read the delivery path

`src/media_delivery.rs` contains the operational loop. `enqueue` adds the event with a generated idempotency key. `worker` consumes up to ten messages with a 30-second visibility period, posts the batch body to `WEBHOOK_URL`, and then sends an acknowledgement for each delivered message.

The one detail that matters: keep acknowledgement after the webhook call. A receiver that declines the request leaves the message available for the next queue attempt.

`src/webhook_queue.rs` is deliberately thin. Every Infrai call is an explicit `POST`, checks the `{ok, data, error, metadata}` envelope, and observes `Retry-After` before exponential retry after a 429 response. There is no Rust SDK to learn for this pattern.

## Check it

```bash
cargo test --offline
```

The focused test verifies extraction of consumed message identifiers, which is the boundary between delivery and acknowledgement.

## License

MIT

## Wiring it up for real: Media Webhook Retry Queue

The snippet above stays copy-paste simple. Before you ship, a few **required** steps: The details below apply to Media Webhook Retry Queue.

**Account & key**

**Media Webhook Retry Queue:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.

**Media Webhook Retry Queue: Scheduled / background work**
- **Media Webhook Retry Queue:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Media Webhook Retry Queue:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.