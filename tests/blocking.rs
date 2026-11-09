//! The blocking wrapper, driven from a plain thread.
#![cfg(feature = "blocking")]

use checkslip::LookupRequest;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn blocking_lookup_works_without_a_runtime() {
    // The mock server needs its own runtime; the blocking client must run
    // outside of it, so it goes on a separate thread.
    let rt = tokio::runtime::Runtime::new().unwrap();
    let server = rt.block_on(async {
        let server = MockServer::start().await;
        let body = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/lookup_found.json"
        ))
        .unwrap();
        Mock::given(method("POST"))
            .and(path("/api/slips/lookup"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
            .mount(&server)
            .await;
        server
    });
    let uri = server.uri();

    let outcome = std::thread::spawn(move || {
        let client = checkslip::blocking::Client::from_builder(
            checkslip::Client::builder()
                .api_key("csk_test_x")
                .base_url(uri),
        )
        .unwrap();
        client
            .lookup(LookupRequest::qr(
                "00330006000001010306902126302105340875102TH9104F86F",
            ))
            .unwrap()
            .outcome
    })
    .join()
    .unwrap();
    assert_eq!(outcome, checkslip::Outcome::Found);
    drop(server);
}
