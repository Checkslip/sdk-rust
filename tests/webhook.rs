//! Webhook signature checks on a full `slip.checked` message.
#![cfg(feature = "webhooks")]

use checkslip::webhook::{Webhook, WebhookError};
use checkslip::{EventType, Outcome, WebhookEvent};
use reqwest::header::{HeaderMap, HeaderValue};

const SECRET: &str = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw8xU3ajqMAKI=";
const MSG_ID: &str = "msg_01JC0000000000000000000000";

const BODY: &str = r#"{"type":"slip.checked","timestamp":"2026-10-29T10:41:03+07:00","data":{"request_id":"0192d4e0-7c1a-7b3e-9f00-1a2b3c4d5e6f","outcome":"FOUND","checked_at":"2026-10-29T10:41:02+07:00","channel":"API","client_reference":"order-1042","checked_elsewhere":false,"resolved_after_pending":true,"slip":{"trans_ref":"630210534087","transferred_at":"2026-10-29T10:33:30+07:00","amount":{"satang":5000,"currency":"THB"},"sender":{"bank":{"code":"KKP","bot_code":"069","name_th":null,"name_en":"Kiatnakin Phatra Bank"},"name_th":null,"name_en":"MR. SOMCHAI J","account":"0439"},"receiver":{"kind":"BANK_ACCOUNT","bank":{"code":"GSB","bot_code":"030","name_th":null,"name_en":null},"name_th":null,"name_en":null,"account":"9681"}}}}"#;

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn headers(id: &str, ts: i64, sig: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert("webhook-id", HeaderValue::from_str(id).unwrap());
    h.insert(
        "webhook-timestamp",
        HeaderValue::from_str(&ts.to_string()).unwrap(),
    );
    h.insert("webhook-signature", HeaderValue::from_str(sig).unwrap());
    h
}

#[test]
fn verifies_and_parses_slip_checked() {
    let hook = Webhook::new(SECRET).unwrap();
    let ts = now();
    let sig = hook.sign(MSG_ID, ts, BODY.as_bytes());
    let event = hook
        .verify(&headers(MSG_ID, ts, &sig), BODY.as_bytes())
        .unwrap();
    assert_eq!(event.event_type(), EventType::SlipChecked);
    match event {
        WebhookEvent::SlipChecked { data, .. } => {
            assert_eq!(data.outcome, Outcome::Found);
            assert!(data.resolved_after_pending);
            assert_eq!(data.client_reference.as_deref(), Some("order-1042"));
            assert_eq!(data.slip.unwrap().amount().unwrap().satang, 5000);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn rotation_header_with_two_signatures_passes_for_either_secret() {
    let old = Webhook::new(SECRET).unwrap();
    let new = Webhook::new("whsec_c2Vjb25kLXNlY3JldC1mb3ItdGVzdHMtb25seS0xMjM=").unwrap();
    let ts = now();
    let header = format!(
        "{} {}",
        new.sign(MSG_ID, ts, BODY.as_bytes()),
        old.sign(MSG_ID, ts, BODY.as_bytes())
    );
    for hook in [&old, &new] {
        hook.verify(&headers(MSG_ID, ts, &header), BODY.as_bytes())
            .unwrap();
    }
}

#[test]
fn refuses_tampering_wrong_id_and_missing_headers() {
    let hook = Webhook::new(SECRET).unwrap();
    let ts = now();
    let sig = hook.sign(MSG_ID, ts, BODY.as_bytes());

    let changed = BODY.replace("5000", "500000");
    assert!(matches!(
        hook.verify(&headers(MSG_ID, ts, &sig), changed.as_bytes()),
        Err(WebhookError::NoMatchingSignature)
    ));
    assert!(matches!(
        hook.verify(&headers("msg_other", ts, &sig), BODY.as_bytes()),
        Err(WebhookError::NoMatchingSignature)
    ));
    let mut h = headers(MSG_ID, ts, &sig);
    h.remove("webhook-signature");
    assert!(matches!(
        hook.verify(&h, BODY.as_bytes()),
        Err(WebhookError::MissingHeader("webhook-signature"))
    ));
    assert!(matches!(
        hook.verify(
            &headers(
                MSG_ID,
                ts - 3600,
                &hook.sign(MSG_ID, ts - 3600, BODY.as_bytes())
            ),
            BODY.as_bytes()
        ),
        Err(WebhookError::TimestampOutOfRange)
    ));
}

#[test]
fn unknown_event_type_is_kept() {
    let hook = Webhook::new(SECRET).unwrap();
    let body =
        br#"{"type":"slip.refunded","timestamp":"2026-11-09T10:00:00+07:00","data":{"x":1}}"#;
    let ts = now();
    let ev = hook
        .verify(&headers(MSG_ID, ts, &hook.sign(MSG_ID, ts, body)), body)
        .unwrap();
    assert_eq!(ev.event_type(), EventType::Unknown("slip.refunded".into()));
    assert!(matches!(ev, WebhookEvent::Unknown(_)));
}
