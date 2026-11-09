//! Integration tests against a local mock server (wiremock).

use std::time::Duration;

use checkslip::time::macros::{date, datetime};
use checkslip::{
    Client, CreateEndpoint, Error, EventType, ListSlipsParams, LookupRequest, Outcome,
    ReceiverKind, SlipData, UpdateEndpoint, UsageInterval, UsageParams,
};
use futures_util::TryStreamExt;
use serde_json::{json, Value};
use wiremock::matchers::{body_json, header, header_exists, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const KEY: &str = "csk_test_0123456789";

fn fixture(name: &str) -> Value {
    let raw = std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    serde_json::from_str(&raw).unwrap()
}

fn client(server: &MockServer) -> Client {
    Client::builder()
        .api_key(KEY)
        .base_url(server.uri())
        .retry_base_delay(Duration::from_millis(1))
        .build()
        .unwrap()
}

fn problem(status: u16, code: &str, title: &str) -> ResponseTemplate {
    ResponseTemplate::new(status)
        .insert_header("content-type", "application/problem+json")
        .insert_header("x-request-id", "0192d4e0-0000-7000-8000-00000000abcd")
        .set_body_json(json!({
            "type": format!("https://checkslip.com/problems/{}", code.to_lowercase().replace('_', "-")),
            "title": title,
            "status": status,
            "code": code,
            "instance": "/api/slips/lookup",
        }))
}

fn idem_keys(reqs: &[Request]) -> Vec<String> {
    reqs.iter()
        .map(|r| {
            r.headers
                .get("idempotency-key")
                .expect("idempotency-key header")
                .to_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

const QR: &str = "00330006000001010306902126302105340875102TH9104F86F";

#[tokio::test]
async fn lookup_qr_sends_auth_body_and_a_generated_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/slips/lookup"))
        .and(header("authorization", format!("Bearer {KEY}").as_str()))
        .and(header_exists("idempotency-key"))
        .and(body_json(
            json!({ "qr": QR, "client_reference": "order-1042" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("lookup_found")))
        .expect(1)
        .mount(&server)
        .await;

    let check = client(&server)
        .slips()
        .lookup(LookupRequest::qr(QR).client_reference("order-1042"))
        .await
        .unwrap();

    assert_eq!(check.outcome, Outcome::Found);
    let slip = check.slip.as_full().expect("full slip on FOUND");
    assert_eq!(slip.trans_ref, "630210534087");
    assert_eq!(slip.amount.satang, 5000);
    assert_eq!(slip.sender.bank.code, "KKP");
    assert_eq!(slip.sender.account.as_deref(), Some("0439"));
    assert_eq!(slip.receiver.kind, ReceiverKind::BankAccount);
    assert_eq!(slip.transferred_at, datetime!(2026-10-29 10:33:30 +07:00));

    let reqs = server.received_requests().await.unwrap();
    let key = &idem_keys(&reqs)[0];
    assert!(checkslip::uuid::Uuid::parse_str(key).is_ok(), "{key}");
    let ua = reqs[0].headers.get("user-agent").unwrap().to_str().unwrap();
    assert!(ua.starts_with("checkslip-rust/"), "{ua}");
}

#[tokio::test]
async fn spec_examples_decode() {
    let bill: checkslip::SlipCheck =
        serde_json::from_value(fixture("lookup_found_bill_payment")).unwrap();
    let slip = bill.slip.as_full().unwrap();
    assert_eq!(slip.receiver.kind, ReceiverKind::BillPayment);
    assert!(slip.receiver.bank.is_none());
    assert_eq!(slip.amount.satang, 125050);

    let dup: checkslip::SlipCheck = serde_json::from_value(fixture("lookup_duplicate")).unwrap();
    assert_eq!(dup.outcome, Outcome::Duplicate);
    match &dup.slip {
        SlipData::Summary(s) => {
            assert_eq!(s.sender_bank.code, "KKP");
            assert_eq!(s.receiver_bank.as_ref().unwrap().code, "GSB");
        }
        other => panic!("expected summary, got {other:?}"),
    }
    assert_eq!(
        dup.duplicate_of.unwrap().request_id.to_string(),
        "0192d4e0-7c1a-7b3e-9f00-1a2b3c4d5e6f"
    );
}

#[tokio::test]
async fn caller_key_is_sent_unchanged() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/slips/lookup"))
        .and(header("idempotency-key", "order-1042-slip-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("lookup_found")))
        .expect(1)
        .mount(&server)
        .await;
    client(&server)
        .slips()
        .lookup(LookupRequest::qr(QR).idempotency_key("order-1042-slip-1"))
        .await
        .unwrap();
}

#[tokio::test]
async fn lookup_retries_5xx_with_the_same_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/slips/lookup"))
        .respond_with(problem(503, "UNAVAILABLE", "ธนาคารไม่ตอบ"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/slips/lookup"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("lookup_found")))
        .mount(&server)
        .await;

    let check = client(&server)
        .slips()
        .lookup(LookupRequest::qr(QR))
        .await
        .unwrap();
    assert_eq!(check.outcome, Outcome::Found);

    let keys = idem_keys(&server.received_requests().await.unwrap());
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0], keys[1], "retry must reuse the key");
}

#[tokio::test]
async fn lookup_retries_idempotency_in_flight() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(problem(409, "IDEMPOTENCY_IN_FLIGHT", "กำลังทำงาน"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("lookup_found")))
        .mount(&server)
        .await;
    client(&server)
        .slips()
        .lookup(LookupRequest::qr(QR))
        .await
        .unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn rate_limit_waits_for_retry_after_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/billing/balance"))
        .respond_with(
            problem(429, "RATE_LIMITED", "เรียกถี่เกินเพดาน").insert_header("retry-after", "1"),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/billing/balance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "account_id": "6f1c2a8e-2b4f-4d6a-9a52-3a1f0c8e9b10",
            "credits": 120,
            "low": false,
            "threshold": 10,
            "threshold_basis": "AVG_7_DAYS",
            "expires_at": "2027-11-01T00:00:00+07:00",
            "updated_at": "2026-11-09T09:00:00+07:00",
            "last_entry_id": null
        })))
        .mount(&server)
        .await;

    let started = std::time::Instant::now();
    let bal = client(&server).billing().balance().get().await.unwrap();
    assert!(
        started.elapsed() >= Duration::from_millis(950),
        "waited Retry-After"
    );
    assert_eq!(bal.credits, 120);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn rate_limit_error_after_retries_run_out() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            problem(429, "RATE_LIMITED", "เรียกถี่เกินเพดาน").insert_header("retry-after", "7"),
        )
        .mount(&server)
        .await;
    let c = Client::builder()
        .api_key(KEY)
        .base_url(server.uri())
        .max_retries(0)
        .build()
        .unwrap();
    let err = c.billing().balance().get().await.unwrap_err();
    match err {
        Error::RateLimited {
            retry_after,
            request_id,
        } => {
            assert_eq!(retry_after, Some(Duration::from_secs(7)));
            assert_eq!(
                request_id.as_deref(),
                Some("0192d4e0-0000-7000-8000-00000000abcd")
            );
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }
}

#[tokio::test]
async fn long_retry_after_is_not_waited_out() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(problem(429, "RATE_LIMITED", "x").insert_header("retry-after", "3600"))
        .mount(&server)
        .await;
    let err = client(&server).billing().balance().get().await.unwrap_err();
    assert!(matches!(err, Error::RateLimited { .. }));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn post_without_key_is_not_retried_on_5xx() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/webhooks/endpoints"))
        .respond_with(problem(500, "INTERNAL", "ระบบขัดข้อง"))
        .mount(&server)
        .await;
    let err = client(&server)
        .webhooks()
        .endpoints()
        .create(&CreateEndpoint::new(
            "https://shop.example/hooks/checkslip",
            [EventType::SlipChecked],
        ))
        .await
        .unwrap_err();
    assert_eq!(err.status(), Some(500));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn get_is_retried_on_5xx_up_to_max_retries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(problem(502, "INTERNAL", "bad gateway"))
        .mount(&server)
        .await;
    let err = client(&server).billing().balance().get().await.unwrap_err();
    assert_eq!(err.status(), Some(502));
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        1 + checkslip::DEFAULT_MAX_RETRIES as usize
    );
}

#[tokio::test]
async fn problem_json_becomes_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(problem(402, "NO_CREDIT", "เครดิตไม่พอ"))
        .mount(&server)
        .await;
    let err = client(&server)
        .slips()
        .lookup(LookupRequest::qr(QR))
        .await
        .unwrap_err();
    assert_eq!(err.code(), Some("NO_CREDIT"));
    assert_eq!(
        err.request_id(),
        Some("0192d4e0-0000-7000-8000-00000000abcd")
    );
    match err {
        Error::Api {
            status,
            code,
            title,
            problem,
            ..
        } => {
            assert_eq!(status, 402);
            assert_eq!(code, "NO_CREDIT");
            assert_eq!(title, "เครดิตไม่พอ");
            assert_eq!(
                problem.type_uri.as_deref(),
                Some("https://checkslip.com/problems/no-credit")
            );
        }
        other => panic!("expected Api, got {other:?}"),
    }
    // 4xx is never retried
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn non_json_error_body_still_gives_status() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404).set_body_string("<html>nope</html>"))
        .mount(&server)
        .await;
    let err = client(&server)
        .slips()
        .get(checkslip::uuid::Uuid::nil())
        .await
        .unwrap_err();
    match err {
        Error::Api {
            status,
            code,
            title,
            ..
        } => {
            assert_eq!(status, 404);
            assert_eq!(code, "");
            assert_eq!(title, "Not Found");
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn bad_success_body_is_a_decode_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-request-id", "r-1")
                .set_body_json(json!({ "credits": "lots" })),
        )
        .mount(&server)
        .await;
    let err = client(&server).billing().balance().get().await.unwrap_err();
    assert!(matches!(err, Error::Decode { .. }), "{err:?}");
    assert_eq!(err.request_id(), Some("r-1"));
}

fn record(id_suffix: u32, outcome: &str) -> Value {
    json!({
        "request_id": format!("0192d4e0-7c1a-7b3e-9f00-{id_suffix:012}"),
        "outcome": outcome,
        "checked_at": "2026-10-29T10:41:02+07:00",
        "channel": "API",
        "api_key_id": "AbCdEf123456",
        "status": 200,
        "latency_ms": 812,
        "checked_elsewhere": false,
        "charged": true,
        "charge_confirmed": true,
        "free_reason": null,
    })
}

#[tokio::test]
async fn list_all_follows_next_cursor() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/slips"))
        .and(query_param("cursor", "p2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [record(3, "FOUND")],
            "next_cursor": "p3"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/slips"))
        .and(query_param("cursor", "p3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [record(4, "SOMETHING_NEW")],
            "next_cursor": null
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/slips"))
        .and(query_param("outcome", "FOUND"))
        .and(query_param("limit", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [record(1, "FOUND"), record(2, "NOT_FOUND")],
            "next_cursor": "p2"
        })))
        .with_priority(10)
        .mount(&server)
        .await;

    let all: Vec<_> = client(&server)
        .slips()
        .list_all(ListSlipsParams::new().outcome([Outcome::Found]).limit(2))
        .try_collect()
        .await
        .unwrap();

    assert_eq!(all.len(), 4);
    assert_eq!(all[1].outcome, Outcome::NotFound);
    assert_eq!(all[3].outcome, Outcome::Unknown("SOMETHING_NEW".into()));
    assert_eq!(all[0].charged, Some(true));

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 3);
    // every page keeps the filters
    for r in &reqs {
        let q = r.url.query().unwrap_or("");
        assert!(q.contains("outcome=FOUND") && q.contains("limit=2"), "{q}");
    }
}

#[tokio::test]
async fn image_lookup_sends_multipart() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/slips/lookup"))
        .and(header_exists("idempotency-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("lookup_found")))
        .mount(&server)
        .await;

    let png = b"\x89PNG\r\n\x1a\nfake image bytes".to_vec();
    client(&server)
        .slips()
        .lookup(LookupRequest::image(png).client_reference("order-7"))
        .await
        .unwrap();

    let reqs = server.received_requests().await.unwrap();
    let ct = reqs[0]
        .headers
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.starts_with("multipart/form-data; boundary="), "{ct}");
    let body = String::from_utf8_lossy(&reqs[0].body);
    assert!(
        body.contains("name=\"image\"; filename=\"slip.png\""),
        "{body}"
    );
    assert!(body.contains("Content-Type: image/png"), "{body}");
    assert!(body.contains("fake image bytes"));
    assert!(body.contains("name=\"client_reference\""));
    assert!(body.contains("order-7"));
}

#[tokio::test]
async fn image_lookup_retry_resends_the_whole_image() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(problem(500, "INTERNAL", "x"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("lookup_found")))
        .mount(&server)
        .await;
    let jpeg = [&[0xFFu8, 0xD8, 0xFF, 0xE0][..], b"jpeg payload"].concat();
    client(&server)
        .slips()
        .lookup(LookupRequest::image(jpeg))
        .await
        .unwrap();
    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 2);
    for r in &reqs {
        assert!(String::from_utf8_lossy(&r.body).contains("jpeg payload"));
    }
    let keys = idem_keys(&reqs);
    assert_eq!(keys[0], keys[1]);
}

#[tokio::test]
async fn endpoint_create_and_patch() {
    let server = MockServer::start().await;
    let endpoint = json!({
        "id": "ep_01JC0000000000000000000000",
        "url": "https://shop.example/hooks/checkslip",
        "description": null,
        "event_types": ["slip.checked"],
        "enabled": true,
        "disabled_reason": null,
        "secret_hint": "…AKI=",
        "created_at": "2026-11-09T10:00:00+07:00",
        "updated_at": "2026-11-09T10:00:00+07:00"
    });
    let mut with_secret = endpoint.clone();
    with_secret["secret"] = json!("whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw8xU3ajqMAKI=");
    Mock::given(method("POST"))
        .and(path("/api/webhooks/endpoints"))
        .and(body_json(json!({
            "url": "https://shop.example/hooks/checkslip",
            "event_types": ["slip.checked"]
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(with_secret))
        .mount(&server)
        .await;
    let mut disabled = endpoint.clone();
    disabled["enabled"] = json!(false);
    disabled["disabled_reason"] = json!("MANUAL");
    Mock::given(method("PATCH"))
        .and(path(
            "/api/webhooks/endpoints/ep_01JC0000000000000000000000",
        ))
        .and(header("content-type", "application/merge-patch+json"))
        .and(body_json(json!({ "enabled": false })))
        .respond_with(ResponseTemplate::new(200).set_body_json(disabled))
        .mount(&server)
        .await;

    let c = client(&server);
    let created = c
        .webhooks()
        .endpoints()
        .create(&CreateEndpoint::new(
            "https://shop.example/hooks/checkslip",
            [EventType::SlipChecked],
        ))
        .await
        .unwrap();
    assert!(created.secret.expose().starts_with("whsec_"));
    assert!(
        !format!("{created:?}").contains("MfKQ"),
        "Debug must hide the secret"
    );

    let updated = c
        .webhooks()
        .endpoints()
        .update(&created.endpoint.id, &UpdateEndpoint::new().enabled(false))
        .await
        .unwrap();
    assert!(!updated.enabled);
    assert_eq!(
        updated.disabled_reason,
        Some(checkslip::DisabledReason::Manual)
    );
}

#[tokio::test]
async fn keyed_webhook_posts_get_a_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/api/webhooks/deliveries/dlv_01JC0000000000000000000000/redeliver",
        ))
        .and(header("idempotency-key", "redo-1"))
        .respond_with(ResponseTemplate::new(202).set_body_json(json!({
            "id": "dlv_01JC0000000000000000000000",
            "message_id": "msg_01JC0000000000000000000000",
            "endpoint_id": "ep_01JC0000000000000000000000",
            "event_type": "slip.checked",
            "status": "PENDING",
            "attempt_count": 0,
            "next_attempt_at": "2026-11-09T10:00:05+07:00",
            "last_attempt_at": null,
            "last_http_status": null,
            "created_at": "2026-11-09T10:00:00+07:00"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(
            "/api/webhooks/endpoints/ep_01JC0000000000000000000000/secret/rotate",
        ))
        .and(header_exists("idempotency-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "secret": "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw8xU3ajqMAKI=",
            "previous_secret_expires_at": "2026-11-10T10:00:00+07:00"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let c = client(&server);
    let d = c
        .webhooks()
        .deliveries()
        .redeliver("dlv_01JC0000000000000000000000", Some("redo-1"))
        .await
        .unwrap();
    assert_eq!(d.status, checkslip::DeliveryStatus::Pending);
    c.webhooks()
        .endpoints()
        .rotate_secret("ep_01JC0000000000000000000000", None)
        .await
        .unwrap();
}

#[tokio::test]
async fn banks_need_no_key_and_usage_sends_dates() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/banks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [{
                "code": "KBANK", "bot_code": "004",
                "name_th": "ธนาคารกสิกรไทย", "name_en": "Kasikornbank",
                "supported_since": "2026-10-01", "landing_page": true, "lookback_days": 90
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/usage"))
        .and(query_param("from", "2026-11-01"))
        .and(query_param("to", "2026-11-09"))
        .and(query_param("interval", "DAY"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "interval": "DAY",
            "from": "2026-11-01",
            "to": "2026-11-09",
            "timezone": "Asia/Bangkok",
            "as_of": "2026-11-09T10:00:00+07:00",
            "buckets": [{
                "start": "2026-11-01",
                "checks": 3, "charged": 2, "free_not_found": 1, "free_duplicate": 0,
                "free_unavailable": 0, "free_invalid_qr": 0, "free_invalid_file": 0,
                "free_qr_tampered": 0, "free_too_old": 0, "free_no_credit": 0,
                "free_hold_expired": 0, "free_system_error": 0
            }],
            "totals": {
                "checks": 3, "charged": 2, "free_not_found": 1, "free_duplicate": 0,
                "free_unavailable": 0, "free_invalid_qr": 0, "free_invalid_file": 0,
                "free_qr_tampered": 0, "free_too_old": 0, "free_no_credit": 0,
                "free_hold_expired": 0, "free_system_error": 0
            }
        })))
        .mount(&server)
        .await;

    let anon = Client::builder().base_url(server.uri()).build().unwrap();
    let banks = anon.banks().list().await.unwrap();
    assert_eq!(banks[0].code, "KBANK");
    assert_eq!(banks[0].supported_since, date!(2026 - 10 - 01));

    let usage = client(&server)
        .usage()
        .get(
            &UsageParams::new(date!(2026 - 11 - 01), date!(2026 - 11 - 09))
                .interval(UsageInterval::Day),
        )
        .await
        .unwrap();
    assert_eq!(usage.totals.charged, 2);
    assert_eq!(usage.buckets[0].counts.free_not_found, 1);

    let reqs = server.received_requests().await.unwrap();
    assert!(reqs[0].headers.get("authorization").is_none());
    assert!(reqs[1].headers.get("authorization").is_some());
}

#[tokio::test]
async fn network_errors_retry_on_get_then_surface() {
    // Bind and drop a listener so the port is very likely closed.
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let c = Client::builder()
        .api_key(KEY)
        .base_url(format!("http://127.0.0.1:{port}"))
        .retry_base_delay(Duration::from_millis(1))
        .build()
        .unwrap();
    let err = c.billing().balance().get().await.unwrap_err();
    assert!(matches!(err, Error::Http(_)), "{err:?}");
}
