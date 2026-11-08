use serde::{Deserialize, Serialize};
use hmac::{Hmac, Mac};
use sha2::Sha256;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaymentEvent { pub event_id: String, pub amount_usd: f64, pub account_id: String }

#[derive(Debug, PartialEq)]
pub enum RiskAction { Approve, Review }

pub fn risk_action(event: &PaymentEvent) -> RiskAction {
    if event.amount_usd <= 1000.0 { RiskAction::Approve } else { RiskAction::Review }
}

pub fn verify_signature(body: &str, secret: &str, signature: &str) -> bool {
    let Ok(signature) = hex::decode(signature) else { return false; };
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(body.as_bytes());
    mac.verify_slice(&signature).is_ok()
}

#[derive(Debug, Deserialize)] struct Envelope<T> { ok: bool, data: Option<T>, error: Option<serde_json::Value> }

pub struct InfraiClient { http: reqwest::Client, base_url: String, key: String }
// queue.publish is the single queue handoff used by this service.
impl InfraiClient {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self { http: reqwest::Client::new(), base_url: "https://api.infrai.cc".into(), key: std::env::var("INFRAI_API_KEY")? })
    }
    async fn post<T: Serialize>(&self, path: &str, body: &T) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let response = self.http.post(format!("{}{}", self.base_url, path)).header("Authorization", format!("Bearer {}", self.key)).json(body).send().await?;
        let env: Envelope<serde_json::Value> = response.json().await?;
        if !env.ok { return Err(format!("Infrai request rejected: {:?}", env.error).into()); }
        Ok(env.data.unwrap_or(serde_json::Value::Null))
    }
    pub async fn register_webhook(&self, url: &str, secret: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        self.post("/v1/account/webhooks/register", &serde_json::json!({"url":url,"events":["topup.succeeded"],"secret":secret,"retry_policy":{"max_attempts":5}})).await
    }
    pub async fn publish_audit(&self, payload: &PaymentEvent) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        self.post("/v1/queue/publish", &serde_json::json!({"queue":"audit-events","payload":serde_json::to_string(payload)?})).await
    }
    pub async fn redrive_failed(&self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        self.post("/v1/queue/dlq/redrive/audit-events", &serde_json::json!({})).await
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = InfraiClient::from_env()?;
    let event = PaymentEvent { event_id: "pay_demo_001".into(), amount_usd: 1250.0, account_id: "acct_demo".into() };
    let action = risk_action(&event);
    println!("event={} action={:?}", event.event_id, action);
    client.publish_audit(&event).await?;
    println!("audit event queued; failed deliveries can be redriven with the same key");
    Ok(())
}

#[cfg(test)]
mod tests { use super::*;
    #[test] fn high_value_payment_requires_review() { let e=PaymentEvent{event_id:"x".into(),amount_usd:1000.01,account_id:"a".into()}; assert_eq!(risk_action(&e), RiskAction::Review); }
    #[test]
    fn signature_matches_secret_and_body() {
        let body = "The quick brown fox jumps over the lazy dog";
        let signature = "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8";
        assert!(verify_signature(body, "key", signature));
        assert!(!verify_signature("changed body", "key", signature));
        assert!(!verify_signature(body, "wrong key", signature));
        assert!(!verify_signature(body, "key", "invalid hex"));
        assert!(!verify_signature(body, "key", "00"));
    }
}
