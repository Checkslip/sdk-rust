//! Guards the surface the four Checkslip SDKs share, so the clients cannot
//! drift apart.
//!
//! Rust has no runtime reflection, so the naming check is a compile-time one:
//! `canonical_surface` names every method the table lists, and this file stops
//! compiling if one is missing, renamed or given a different shape. The
//! "nothing extra is exposed" half of the check lives in the JavaScript, Go and
//! PHP suites, which can enumerate their own surface.

use std::time::Duration;

use checkslip::time::macros::{date, datetime};
use checkslip::{
    ApiRequestsParams, Channel, Client, CountBucket, CountSlipsParams, CreateEndpoint,
    CreateEvidenceExport, FreeReason, LedgerParams, ListDeliveriesParams, ListSlipsParams,
    LookupRequest, OrdersParams, UpdateEndpoint, UsageBucket, UsageCounts, UsageParams,
};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const KEY: &str = "csk_test_0123456789";

fn client(server: &MockServer) -> Client {
    Client::builder()
        .api_key(KEY)
        .base_url(server.uri())
        .retry_base_delay(Duration::from_millis(1))
        .build()
        .unwrap()
}

/// Every canonical `resource.method` from the table, spelled the Rust way:
/// 39 operations, 9 `list_all` streams and the local `webhooks().verify`.
///
/// This function is never called. It exists so the compiler checks that each
/// name is there, takes what the table says it takes, and returns something
/// awaitable where the table says so.
#[allow(dead_code, unused_must_use, clippy::no_effect_underscore_binding)]
async fn canonical_surface(c: &Client) {
    let uuid = uuid::Uuid::nil();

    // verify: slips (5 + 3 on exports), banks (1)
    c.slips().lookup(LookupRequest::qr("x"));
    c.slips().get(uuid);
    c.slips().list(&ListSlipsParams::default());
    c.slips().list_all(ListSlipsParams::default());
    c.slips().counts(&CountSlipsParams::new(
        datetime!(2026-11-01 00:00 +07:00),
        datetime!(2026-11-09 00:00 +07:00),
        CountBucket::Day,
    ));
    c.slips().exports().create(&Default::default());
    c.slips().exports().get(uuid);
    c.banks().list();

    // billing (13)
    c.billing().balance().get();
    c.billing().info().get();
    c.billing().packages().list();
    c.billing().ledger().list(&LedgerParams::default());
    c.billing().ledger().list_all(LedgerParams::default());
    c.billing().orders().list(&OrdersParams::default());
    c.billing().orders().list_all(OrdersParams::default());
    c.billing().orders().get(uuid);
    c.billing().orders().sync(uuid);
    c.billing().receipts().list(None, None);
    c.billing().receipts().list_all();
    c.billing().receipts().get(uuid);

    // usage (1)
    c.usage().get(&UsageParams::new(
        date!(2026 - 11 - 01),
        date!(2026 - 11 - 09),
    ));

    // audit (11)
    c.audit().events().list(&Default::default());
    c.audit().events().list_all(Default::default());
    c.audit().events().get(uuid);
    c.audit().requests().list(&Default::default());
    c.audit().requests().list_all(Default::default());
    c.audit().requests().get(uuid);
    c.audit().exports().create(
        &CreateEvidenceExport::new(date!(2026 - 11 - 01), date!(2026 - 11 - 02)),
        None,
    );
    c.audit().exports().list(None, None);
    c.audit().exports().list_all();
    c.audit().exports().get(uuid);
    c.audit().exports().download(uuid);

    // webhook (15)
    c.webhooks().endpoints().list(None, None);
    c.webhooks().endpoints().list_all();
    c.webhooks()
        .endpoints()
        .create(&CreateEndpoint::new("https://example.com/h", []));
    c.webhooks().endpoints().get("ep_1");
    c.webhooks()
        .endpoints()
        .update("ep_1", &UpdateEndpoint::default());
    c.webhooks().endpoints().delete("ep_1");
    c.webhooks().endpoints().rotate_secret("ep_1", None);
    c.webhooks().endpoints().send_test("ep_1", None);
    c.webhooks()
        .deliveries()
        .list(&ListDeliveriesParams::default());
    c.webhooks()
        .deliveries()
        .list_all(ListDeliveriesParams::default());
    c.webhooks().deliveries().get("dl_1");
    c.webhooks().deliveries().redeliver("dl_1", None);
    c.webhooks().event_types().list();
}

#[tokio::test]
async fn bare_items_are_unwrapped() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/banks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [{
                "code": "KKP", "bot_code": "069", "name_th": "a", "name_en": "b",
                "supported_since": "2026-01-01", "landing_page": true, "lookback_days": 90
            }]
        })))
        .mount(&server)
        .await;

    let banks = client(&server).banks().list().await.unwrap();
    assert_eq!(banks.len(), 1);
    assert_eq!(banks[0].code, "KKP");
}

#[tokio::test]
async fn event_types_are_unwrapped() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/webhooks/event-types"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [{ "type": "slip.checked", "description": "d", "subscribable": true }]
        })))
        .mount(&server)
        .await;

    let types = client(&server)
        .webhooks()
        .event_types()
        .list()
        .await
        .unwrap();
    assert_eq!(types.len(), 1);
}

#[tokio::test]
async fn the_package_list_keeps_its_envelope() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/billing/packages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [],
            "prices_include_vat": true,
            "checkout_open": false
        })))
        .mount(&server)
        .await;

    let packages = client(&server).billing().packages().list().await.unwrap();
    assert!(packages.prices_include_vat);
    assert!(!packages.checkout_open);
}

#[tokio::test]
async fn accepted_is_not_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/slips/lookup"))
        .respond_with(
            ResponseTemplate::new(202)
                .insert_header("retry-after", "5")
                .set_body_json(json!({
                    "request_id": "0192d4e0-0000-7000-8000-0000000000a1",
                    "outcome": "PENDING",
                    "checked_at": "2026-10-29T10:41:02+07:00",
                    "slip": {
                "trans_ref": "630210534087",
                "sender_bank": { "code": "KKP", "bot_code": "069", "name_th": null, "name_en": null }
            }
                })),
        )
        .mount(&server)
        .await;

    let check = client(&server)
        .slips()
        .lookup(LookupRequest::qr("x"))
        .await
        .expect("202 must not be an error");
    assert_eq!(check.outcome, checkslip::Outcome::Pending);
}

#[tokio::test]
async fn a_payload_with_unmodelled_fields_still_parses() {
    // A response may carry fields the SDK does not model, including ones that
    // left the customer API. Parsing must not depend on the exact set.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/slips/lookup"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "request_id": "0192d4e0-0000-7000-8000-0000000000a1",
            "outcome": "NOT_FOUND",
            "checked_at": "2026-10-29T10:41:02+07:00",
            "slip": {
                "trans_ref": "630210534087",
                "sender_bank": { "code": "KKP", "bot_code": "069", "name_th": null, "name_en": null }
            },
            "trace_id": "t1",
            "recorded_by": "gw",
            "chain": {},
            "build": "2026.11.9",
            "latency_ms": 812,
            "upstream_ms": 640,
            "checked_elsewhere": false
        })))
        .mount(&server)
        .await;

    let check = client(&server)
        .slips()
        .lookup(LookupRequest::qr("x"))
        .await
        .expect("unmodelled fields must not break parsing");
    assert_eq!(check.outcome, checkslip::Outcome::NotFound);
}

#[test]
fn usage_counts_parse_with_or_without_the_deprecated_fields() {
    // The API added free_other, stopped counting free_no_credit and
    // free_hold_expired, and will drop those two fields. The crate and the
    // API are deployed separately, so a response from either side of that
    // change has to parse.
    let full = json!({
        "checks": 12, "charged": 7, "free_not_found": 1, "free_duplicate": 0,
        "free_unavailable": 0, "free_invalid_qr": 0, "free_invalid_file": 0,
        "free_qr_tampered": 0, "free_too_old": 0, "free_no_credit": 3,
        "free_hold_expired": 2, "free_system_error": 0
    });
    let before: UsageCounts =
        serde_json::from_value(full).expect("the shape with the deprecated fields must parse");
    assert_eq!(before.free_no_credit, 3);
    assert_eq!(before.free_hold_expired, 2);
    assert_eq!(
        before.free_other, 0,
        "a server that predates the field sends no count"
    );

    let trimmed = json!({
        "checks": 12, "charged": 7, "free_not_found": 1, "free_duplicate": 0,
        "free_unavailable": 0, "free_invalid_qr": 0, "free_invalid_file": 0,
        "free_qr_tampered": 0, "free_too_old": 0, "free_system_error": 0,
        "free_other": 5
    });
    let after: UsageCounts = serde_json::from_value(trimmed.clone())
        .expect("the shape without the deprecated fields must parse");
    assert_eq!(after.free_other, 5);
    assert_eq!(after.free_no_credit, 0);
    assert_eq!(after.free_hold_expired, 0);

    // UsageBucket flattens the counts, so the defaults have to survive that too.
    let mut bucket = trimmed;
    bucket["start"] = json!("2026-11-01");
    let bucket: UsageBucket =
        serde_json::from_value(bucket).expect("a flattened bucket must parse as well");
    assert_eq!(bucket.counts.free_other, 5);
    assert_eq!(bucket.counts.free_no_credit, 0);

    assert_eq!(FreeReason::Other.as_str(), "OTHER");
    assert_eq!(
        serde_json::from_value::<FreeReason>(json!("OTHER")).unwrap(),
        FreeReason::Other
    );
    // A reason added after this version still arrives whole.
    assert_eq!(
        serde_json::from_value::<FreeReason>(json!("SOMETHING_NEW")).unwrap(),
        FreeReason::Unknown("SOMETHING_NEW".to_string())
    );
}

#[tokio::test]
async fn audit_request_filters_repeat_the_key_for_each_value() {
    // The spec declares these four as style=form with explode=true, so each
    // value gets its own key instead of being joined into one.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/audit/requests"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "items": [], "next_cursor": null })),
        )
        .mount(&server)
        .await;

    let params = ApiRequestsParams::new()
        .http_status([200, 429])
        .outcome_code(["FOUND", "NOT_FOUND"])
        .route(["/api/slips/lookup", "/api/usage"])
        .channel([Channel::Api, Channel::Dashboard]);
    client(&server)
        .audit()
        .requests()
        .list(&params)
        .await
        .expect("the multi-value filters must be accepted");

    let got = &server.received_requests().await.unwrap()[0];
    let pairs: Vec<(String, String)> = got
        .url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    for expected in [
        ("http_status", "200"),
        ("http_status", "429"),
        ("outcome_code", "FOUND"),
        ("outcome_code", "NOT_FOUND"),
        ("route", "/api/slips/lookup"),
        ("route", "/api/usage"),
        ("channel", "API"),
        ("channel", "DASHBOARD"),
    ] {
        assert!(
            pairs
                .iter()
                .any(|(k, v)| k == expected.0 && v == expected.1),
            "missing {expected:?} in {pairs:?}"
        );
    }
}
