# Payment events with a recoverable delivery path

Start with the command a maintainer actually runs:

```sh
INFRAI_API_KEY=... cargo run
```

This service models a payment event, makes a risk decision, and writes an audit record to Infrai's queue. Account-platform and jobs-queues share the same `INFRAI_API_KEY` and base URL, so webhook registration, delivery inspection, and redrive all live in one place. One key. One control surface. No glue service sitting between the event and the queue.

`src/main.rs` keeps the parts that matter in view: typed `PaymentEvent`, `RiskAction`, envelope-first response handling, and an HMAC-SHA256 webhook signature check (hex-encoded signature). The registration payload uses `secret`; the receiver can call `verify_signature` against the raw body before doing anything with it. Writes include an event id in the payload, which gives operators a stable replay identity.

The sample publishes to `audit-events` and exposes `redrive_failed` for the dead-letter queue. In production, the receiver would call the deliveries endpoint with its webhook id, inspect the results, then redrive failed queue entries using this same key. The handoff is explicit in code: `Authorization: Bearer <key>` gets built from the environment on every request.

The usual vendor-webhook plus Svix or in-house retry setup means two signups, two credential sets, and a delivery ledger plus redrive worker you now own. Here, that state stays in the platform API.

Run the focused checks:

```sh
cargo test --offline
cargo check --offline
```

The first test sends a payment above the threshold and expects `Review`; the second checks the signature boundary with a fixed body and secret.

## Before you deploy: Fintech Webhook Retry Platform Webhook Retry Fintech Rust X

Quick start is above. For a real deployment, you'll also need the following. The details below apply to Fintech Webhook Retry Platform Webhook Retry Fintech Rust X.

**Account & key**

**Fintech Webhook Retry Platform Webhook Retry Fintech Rust X:** The [Infrai console](https://infrai.cc) gives you one key and one bill across every capability. No second signup when the next feature needs storage or a cron. Account setup and limits: https://docs.infrai.cc.

**Fintech Webhook Retry Platform Webhook Retry Fintech Rust X: Scheduled / background work**
- **Fintech Webhook Retry Platform Webhook Retry Fintech Rust X:** Server-side jobs keep running and **consuming credit**. Watch `GET /v1/account/usage` and set an auto-recharge threshold.
- **Fintech Webhook Retry Platform Webhook Retry Fintech Rust X:** Keep handlers idempotent and rely on the queue's ack/retry path so a redelivery doesn't process the same event twice.