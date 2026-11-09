//! Checking the signature on webhook messages (feature `webhooks`).
//!
//! Checkslip signs messages the [Standard Webhooks] way. Each request to
//! your endpoint carries three headers:
//!
//! - `webhook-id`: the message ID (`msg_...`). It stays the same on
//!   retries and redeliveries, so deduplicate on it.
//! - `webhook-timestamp`: Unix seconds when this attempt was signed.
//! - `webhook-signature`: one or more `v1,<base64>` values separated by
//!   spaces. There are two while a rotated-out secret is still active.
//!
//! The signature is HMAC-SHA256 over `{webhook-id}.{webhook-timestamp}.{body}`
//! keyed with the base64-decoded part of your `whsec_...` secret. The check
//! has to run on the raw request body, before any JSON parsing, and messages
//! whose timestamp is more than five minutes from your clock should be
//! refused so an old message cannot be replayed.
//!
//! [Standard Webhooks]: https://www.standardwebhooks.com/
//!
//! ```
//! use checkslip::webhook::Webhook;
//! use checkslip::WebhookEvent;
//! # use reqwest::header::HeaderMap;
//! # fn handler(headers: &HeaderMap, raw_body: &[u8]) -> Result<(), checkslip::webhook::WebhookError> {
//! let hook = Webhook::new("whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw8xU3ajqMAKI=")?;
//! match hook.verify(headers, raw_body)? {
//!     WebhookEvent::SlipChecked { data, .. } => println!("{} {}", data.request_id, data.outcome),
//!     WebhookEvent::Test { .. } => println!("test message"),
//!     _ => {} // event types added later
//! }
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use hmac::{Hmac, Mac};
use reqwest::header::HeaderMap;
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::models::WebhookEvent;

/// How far a message timestamp may be from your clock: five minutes.
pub const DEFAULT_TOLERANCE: Duration = Duration::from_secs(5 * 60);

/// Why a webhook message was refused.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WebhookError {
    /// The secret is not `whsec_` followed by base64.
    #[error("the webhook secret is not in whsec_<base64> form")]
    InvalidSecret,
    /// A required header is missing or not valid text.
    #[error("missing or unreadable {0} header")]
    MissingHeader(&'static str),
    /// `webhook-timestamp` is not a whole number of seconds.
    #[error("webhook-timestamp is not a number")]
    InvalidTimestamp,
    /// The timestamp is too far from the current time.
    #[error("webhook-timestamp is outside the allowed window")]
    TimestampOutOfRange,
    /// No signature in the header matches.
    #[error("no webhook signature matches")]
    NoMatchingSignature,
    /// The signature matched but the body is not a message this crate can
    /// read.
    #[error("webhook body is not valid JSON: {0}")]
    Payload(#[source] serde_json::Error),
}

/// Checks webhook signatures with one signing secret.
#[derive(Clone)]
pub struct Webhook {
    key: Vec<u8>,
    tolerance: Duration,
}

impl fmt::Debug for Webhook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Webhook")
            .field("key", &"***")
            .field("tolerance", &self.tolerance)
            .finish()
    }
}

impl Webhook {
    /// Uses `secret`, the `whsec_...` value from endpoint creation or
    /// rotation.
    pub fn new(secret: &str) -> Result<Self, WebhookError> {
        let raw = secret.trim();
        let raw = raw.strip_prefix("whsec_").unwrap_or(raw);
        let key = STANDARD
            .decode(raw)
            .map_err(|_| WebhookError::InvalidSecret)?;
        if key.is_empty() {
            return Err(WebhookError::InvalidSecret);
        }
        Ok(Webhook {
            key,
            tolerance: DEFAULT_TOLERANCE,
        })
    }

    /// Changes the allowed clock difference. Defaults to
    /// [`DEFAULT_TOLERANCE`].
    pub fn tolerance(mut self, tolerance: Duration) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Checks the signature headers against the raw body and, if they
    /// match, parses the body.
    pub fn verify(&self, headers: &HeaderMap, body: &[u8]) -> Result<WebhookEvent, WebhookError> {
        let get = |name: &'static str| {
            headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .ok_or(WebhookError::MissingHeader(name))
        };
        let id = get("webhook-id")?;
        let ts = get("webhook-timestamp")?;
        let sig = get("webhook-signature")?;
        self.verify_signature(id, ts, sig, body)?;
        serde_json::from_slice(body).map_err(WebhookError::Payload)
    }

    /// Checks a signature from header values you read yourself. Does not
    /// parse the body.
    pub fn verify_signature(
        &self,
        webhook_id: &str,
        webhook_timestamp: &str,
        webhook_signature: &str,
        body: &[u8],
    ) -> Result<(), WebhookError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.verify_signature_at(webhook_id, webhook_timestamp, webhook_signature, body, now)
    }

    /// Like [`verify_signature`](Self::verify_signature) with a fixed "now"
    /// in Unix seconds. Useful in tests.
    pub fn verify_signature_at(
        &self,
        webhook_id: &str,
        webhook_timestamp: &str,
        webhook_signature: &str,
        body: &[u8],
        now_unix: i64,
    ) -> Result<(), WebhookError> {
        let ts: i64 = webhook_timestamp
            .trim()
            .parse()
            .map_err(|_| WebhookError::InvalidTimestamp)?;
        let tol = self.tolerance.as_secs() as i64;
        if (now_unix - ts).abs() > tol {
            return Err(WebhookError::TimestampOutOfRange);
        }

        let expected = self.mac(webhook_id, ts, body);
        let matched = webhook_signature
            .split_whitespace()
            .filter_map(|s| s.strip_prefix("v1,"))
            .filter_map(|b64| STANDARD.decode(b64).ok())
            .fold(false, |acc, got| {
                acc | bool::from(got.as_slice().ct_eq(expected.as_slice()))
            });
        if matched {
            Ok(())
        } else {
            Err(WebhookError::NoMatchingSignature)
        }
    }

    /// Makes a `v1,<base64>` signature, the same way Checkslip does. Handy
    /// for testing your webhook handler.
    pub fn sign(&self, webhook_id: &str, timestamp: i64, body: &[u8]) -> String {
        format!(
            "v1,{}",
            STANDARD.encode(self.mac(webhook_id, timestamp, body))
        )
    }

    fn mac(&self, id: &str, ts: i64, body: &[u8]) -> Vec<u8> {
        let mut mac =
            <Hmac<Sha256> as Mac>::new_from_slice(&self.key).expect("HMAC accepts any key length");
        mac.update(id.as_bytes());
        mac.update(b".");
        mac.update(ts.to_string().as_bytes());
        mac.update(b".");
        mac.update(body);
        mac.finalize().into_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected signature computed separately with Python's hmac module:
    // base64(hmac.new(b64decode(key), b"{id}.{ts}.{body}", sha256).digest())
    const SECRET: &str = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw8xU3ajqMAKI=";
    const ID: &str = "msg_p5jXN8AQM9LWM0D4loKWxJek";
    const TS: i64 = 1614265330;
    const BODY: &[u8] = br#"{"test": 2432232314}"#;
    const SIG: &str = "v1,sFUQ7ZQ/JupRnxDQ+9BoErr5rwitcY1Mv6IYf+B4/LE=";

    #[test]
    fn matches_an_independent_hmac() {
        let w = Webhook::new(SECRET).unwrap();
        assert_eq!(w.sign(ID, TS, BODY), SIG);
        w.verify_signature_at(ID, &TS.to_string(), SIG, BODY, TS)
            .unwrap();
    }

    #[test]
    fn accepts_any_of_several_signatures() {
        let w = Webhook::new(SECRET).unwrap();
        let header = format!("v1,bm90LXRoaXMtb25l {SIG}");
        w.verify_signature_at(ID, &TS.to_string(), &header, BODY, TS)
            .unwrap();
    }

    #[test]
    fn refuses_changed_body_old_timestamp_and_other_versions() {
        let w = Webhook::new(SECRET).unwrap();
        let ts = TS.to_string();
        assert!(matches!(
            w.verify_signature_at(ID, &ts, SIG, br#"{"test": 1}"#, TS),
            Err(WebhookError::NoMatchingSignature)
        ));
        assert!(matches!(
            w.verify_signature_at(ID, &ts, SIG, BODY, TS + 301),
            Err(WebhookError::TimestampOutOfRange)
        ));
        assert!(matches!(
            w.verify_signature_at(ID, &ts, SIG, BODY, TS - 301),
            Err(WebhookError::TimestampOutOfRange)
        ));
        let v2 = SIG.replace("v1,", "v2,");
        assert!(matches!(
            w.verify_signature_at(ID, &ts, &v2, BODY, TS),
            Err(WebhookError::NoMatchingSignature)
        ));
        assert!(matches!(
            w.verify_signature_at(ID, "soon", SIG, BODY, TS),
            Err(WebhookError::InvalidTimestamp)
        ));
    }

    #[test]
    fn rejects_bad_secrets() {
        assert!(Webhook::new("whsec_!!!").is_err());
        assert!(Webhook::new("whsec_").is_err());
    }

    #[test]
    fn debug_hides_the_key() {
        let w = Webhook::new(SECRET).unwrap();
        assert!(!format!("{w:?}").contains("MfKQ"));
    }
}
