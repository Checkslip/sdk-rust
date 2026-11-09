//! Check a webhook message the way your HTTP handler would.
//!
//! ```sh
//! cargo run --example verify_webhook --features webhooks
//! ```

use checkslip::webhook::Webhook;
use checkslip::WebhookEvent;
use reqwest::header::{HeaderMap, HeaderValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // In a real handler the secret comes from your config and the headers
    // and raw body from the incoming request.
    let secret = std::env::var("CHECKSLIP_WEBHOOK_SECRET")
        .unwrap_or_else(|_| "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw8xU3ajqMAKI=".to_owned());
    let hook = Webhook::new(&secret)?;

    let body = br#"{"type":"webhook.test","timestamp":"2026-11-09T10:00:00+07:00","data":{"endpoint_id":"ep_01JC0000000000000000000000"}}"#;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    let mut headers = HeaderMap::new();
    headers.insert(
        "webhook-id",
        HeaderValue::from_static("msg_01JC0000000000000000000000"),
    );
    headers.insert(
        "webhook-timestamp",
        HeaderValue::from_str(&now.to_string())?,
    );
    let sig = hook.sign("msg_01JC0000000000000000000000", now, body);
    headers.insert("webhook-signature", HeaderValue::from_str(&sig)?);

    match hook.verify(&headers, body)? {
        WebhookEvent::SlipChecked { data, .. } => {
            println!("lookup {} finished: {}", data.request_id, data.outcome)
        }
        WebhookEvent::Test { data, .. } => println!("test message for {}", data.endpoint_id),
        other => println!("event type {} (newer than this crate)", other.event_type()),
    }
    Ok(())
}
