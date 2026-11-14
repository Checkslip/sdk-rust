# checkslip

Official Rust client for the [Checkslip](https://checkslip.com) API.

[อ่านภาษาไทย (Thai)](README.th.md)

Checkslip looks up Thai bank transfers. You send the QR text from a transfer
slip, or the slip image, and Checkslip asks the sending bank for the transfer
it holds. The response carries what the bank has: amount, time, sender and
receiver. Whether that matches the order you are selling is your decision, so
compare the fields against your own records before you accept a payment.

API reference: <https://checkslip.com/docs/reference>

## Install

```sh
cargo add checkslip
cargo add tokio --features "macros rt-multi-thread"
```

The minimum supported Rust version is 1.80. It was 1.75 through 0.4.0 and
moved in 0.4.1, so that the `native-tls` feature can take a patched OpenSSL.
On Rust 1.75 you can stay on 0.4.0, which has the same API, but only with
the default `rustls-tls`: with `native-tls`, 0.4.0 keeps the OpenSSL this
release patches.

## Quick start

Create an API key in the Checkslip web app. Keys start with `csk_`. Keep the
key out of your source code and pass it in through the environment.

```rust,no_run
use checkslip::{Client, LookupRequest, Outcome};

#[tokio::main]
async fn main() -> checkslip::Result<()> {
    let client = Client::from_env()?; // reads CHECKSLIP_API_KEY

    let check = client
        .slips()
        .lookup(
            LookupRequest::qr("00330006000001010306902126302105340875102TH9104F86F")
                .client_reference("order-1042"),
        )
        .await?;

    match check.outcome {
        Outcome::Found => {
            let slip = check.slip.as_full().expect("FOUND carries the full record");
            println!("{} satang from {}", slip.amount.satang, slip.sender.bank.code);
        }
        Outcome::Duplicate => println!(
            "already checked this slip, first at {}",
            check.duplicate_of.expect("DUPLICATE carries duplicate_of").checked_at
        ),
        Outcome::NotFound => println!("the bank has no transfer with this reference"),
        other => println!("outcome {other}"),
    }
    Ok(())
}
```

Only `FOUND` uses a credit.

To look up from the image a customer uploaded (JPEG, PNG or WebP, up to
10 MB), pass the bytes. The client sends them as `multipart/form-data` and
works out the content type from the file header:

```rust,no_run
# async fn run(client: checkslip::Client, bytes: Vec<u8>) -> checkslip::Result<()> {
use checkslip::LookupRequest;

let check = client.slips().lookup(LookupRequest::image(bytes)).await?;
# Ok(())
# }
```

Checkslip reads the QR from the image and does not store the image.

### Other calls

```rust,no_run
# async fn run(client: checkslip::Client, check: checkslip::SlipCheck) -> checkslip::Result<()> {
use checkslip::{day, UsageParams};

// the stored record of an earlier lookup
let record = client.slips().get(check.request_id).await?;

let balance = client.billing().balance().get().await?;
println!("{} credits left", balance.credits);

let usage = client
    .usage().get(&UsageParams::new(day(2026, 11, 1)?, day(2026, 11, 30)?))
    .await?;

let banks = client.banks().list().await?; // needs no API key
# Ok(())
# }
```

| Group | Reached through | Calls |
|-------|-----------------|-------|
| Lookups and history | `client.slips()` | `lookup`, `get`, `list`, `list_all`, `counts`, and `exports()` with `create`, `get`, `download_part` |
| Banks | `client.banks()` | `list` |
| Billing | `client.billing()` | `balance().get`, `info()` with `get` and `update`, `packages().list`, `ledger()` with `list` and `list_all`, `orders()` with `list`, `list_all`, `get`, `sync`, `receipts()` with `list`, `list_all`, `get` |
| Usage | `client.usage()` | `get` |
| Audit | `client.audit()` | `events()` with `list`, `list_all`, `get`; `requests()` the same; `exports()` with `list`, `list_all`, `create`, `get`, `download` |
| Webhooks | `client.webhooks()` | `verify`, `event_types().list`, `endpoints()` with `list`, `list_all`, `create`, `get`, `update`, `delete`, `recover`, `rotate_secret`, `send_test`, and `deliveries()` with `list`, `list_all`, `get`, `redeliver` |

The same names, in the same shape, are in the JavaScript, Go and PHP clients.

## Configuration

```rust,no_run
use std::time::Duration;

let client = checkslip::Client::builder()
    .api_key(std::env::var("CHECKSLIP_API_KEY").unwrap())
    .timeout(Duration::from_secs(20)) // per attempt, default 30 s
    .max_retries(3)                   // default 2
    .user_agent("my-shop/1.4")        // sent before checkslip-rust/<version>
    .build()?;
# Ok::<(), checkslip::Error>(())
```

`base_url` points the client at another address, such as a mock server in
your tests. `http_client` takes your own `reqwest::Client` if you need a proxy
or custom TLS settings.

## Errors

The API answers errors as RFC 9457 problem documents
(`application/problem+json`). Each one has a `code` that stays stable, so
match on that. The `title` and `detail` are written for people and are often
in Thai.

```rust,no_run
# async fn run(client: checkslip::Client) {
use checkslip::{Error, LookupRequest};

match client.slips().lookup(LookupRequest::qr("...")).await {
    Ok(check) => println!("{}", check.outcome),
    Err(Error::Api { code, request_id, .. }) if code == "NO_CREDIT" => {
        println!("out of credit (request {request_id:?})");
    }
    Err(Error::Api { status: 422, code, .. }) => println!("slip problem: {code}"),
    Err(Error::RateLimited { retry_after, .. }) => println!("try again in {retry_after:?}"),
    Err(Error::Http(e)) => println!("network: {e}"),
    Err(e) => println!("{e}"),
}
# }
```

| Variant | When |
|---------|------|
| `Error::Api { status, code, title, detail, request_id, problem }` | The server answered with an error status. `problem` holds the whole body, including field errors. |
| `Error::RateLimited { retry_after, request_id }` | HTTP 429 after the client used up its retries. |
| `Error::Http(reqwest::Error)` | No response: connection, TLS or timeout. |
| `Error::Decode { source, request_id }` | A success response whose body did not match the expected type. |
| `Error::InvalidArgument(String)` | Something the client could not send, such as an API key containing a newline. |

Quote the `request_id` when you contact support.

## Retries

The client retries by itself. With the default `max_retries(2)` a call is
sent at most three times.

- HTTP 429 is retried after the number of seconds in `Retry-After`. If the
  server asks for more than 60 seconds (change this with `max_retry_after`),
  the client returns `Error::RateLimited` straight away so your code decides.
- Network errors, timeouts and 5xx responses are retried only when sending
  the request twice cannot do harm: `GET`, `PUT` and `DELETE` calls, and
  `POST` calls that carry an `Idempotency-Key`. A `POST` without one, such as
  creating a webhook endpoint, is never resent.
- A lookup that gets `409 IDEMPOTENCY_IN_FLIGHT` is retried, because the first
  request with that key is still running.
- Waits between attempts start at 500 ms and double each time up to 8 seconds.
  Each wait is a random value between half and all of that step, so many
  clients that failed at once do not retry in step.

Other 4xx errors are never retried.

## Idempotency

Every lookup sends an `Idempotency-Key` header. If you do not set one, the
client generates a random UUID for that call and reuses it on each retry, so a
retried lookup returns the first result and is charged at most once.

The generated key lives only as long as the call. If your process might crash
and send the same slip again later, set the key yourself from something
stable, such as your order ID:

```rust,no_run
# async fn run(client: checkslip::Client) -> checkslip::Result<()> {
use checkslip::LookupRequest;

let check = client
    .slips()
    .lookup(LookupRequest::qr("...").idempotency_key("order-1042-slip-1"))
    .await?;
# Ok(())
# }
```

The API refuses to reuse a key for a different request
(`422 IDEMPOTENCY_KEY_REUSED`).

The webhook calls that need a key (`recover`, `rotate_secret`, `send_test`,
`redeliver`) and `audit().exports().create` take an `Option<&str>`. Pass `None`
and the client generates one.

## Pagination

List endpoints return one page at a time with a `next_cursor`. Each has two
forms:

- `list(&params)` returns one `Page<T>` with `items` and `next_cursor`.
- `list_all(params)` returns a `Stream` of items and requests the next page
  as you read.

```rust,no_run
# async fn run(client: checkslip::Client) -> checkslip::Result<()> {
use futures_util::TryStreamExt;
use checkslip::{ListSlipsParams, Outcome};

let mut found = client
    .slips()
    .list_all(ListSlipsParams::new().outcome([Outcome::Found]).limit(100));

while let Some(record) = found.try_next().await? {
    println!("{} {:?}", record.request_id, record.client_reference);
}
# Ok(())
# }
```

`futures-util` (or `futures`) gives you `TryStreamExt`. If fetching a page
fails, the stream yields that error and ends.

## Webhooks

Checkslip sends a `slip.checked` message to your endpoint when a lookup
finishes. Messages are
signed the [Standard Webhooks](https://www.standardwebhooks.com/) way, and the
`webhooks` feature adds a checker:

```sh
cargo add checkslip --features webhooks
```

```rust,no_run
# #[cfg(feature = "webhooks")]
# mod example {
use checkslip::webhook::Webhook;
use checkslip::WebhookEvent;
# fn handle(headers: &reqwest::header::HeaderMap, raw_body: &[u8]) -> Result<(), Box<dyn std::error::Error>> {

// The whsec_... secret from endpoint creation, loaded from your config.
let hook = Webhook::new(&std::env::var("CHECKSLIP_WEBHOOK_SECRET")?)?;

// Use the raw body bytes, before any JSON parsing.
match hook.verify(headers, raw_body)? {
    WebhookEvent::SlipChecked { data, .. } => {
        println!("lookup {} finished: {}", data.request_id, data.outcome);
    }
    WebhookEvent::Test { .. } => println!("test message"),
    other => println!("newer event type: {}", other.event_type()),
}
# Ok(())
# }
# }
```

`verify` reads the `webhook-id`, `webhook-timestamp` and `webhook-signature`
headers. It refuses a message whose signature does not match or whose
timestamp is more than five minutes from your clock (change this with
`Webhook::tolerance`). During the 24 hours after you rotate a secret, messages
carry two signatures, and a match on either one passes.

The same message can arrive more than once, for example after a retry or a
`redeliver` call. It keeps its `webhook-id`, so store the IDs you have
handled and skip repeats.

`axum` and `actix-web` 4 both use the `http` 1.x `HeaderMap`, which is the
type `verify` takes. With another framework, read the three headers yourself
and call `verify_signature`.

## Features

| Feature | Default | Adds |
|---------|---------|------|
| `rustls-tls` | yes | TLS through rustls |
| `native-tls` | no | TLS through the platform library (OpenSSL, Schannel or Secure Transport) |
| `webhooks` | no | `checkslip::webhook` for webhook signature checks |
| `blocking` | no | `checkslip::blocking::Client` for programs without an async runtime |

To use the platform TLS library instead of rustls:

```sh
cargo add checkslip --no-default-features --features native-tls
```

The blocking client runs the async client on a private Tokio runtime. Do not
call it from async code.

```rust,no_run
# #[cfg(feature = "blocking")]
# fn run() -> checkslip::Result<()> {
let client = checkslip::blocking::Client::new(std::env::var("CHECKSLIP_API_KEY").unwrap())?;
let balance = client.balance()?;
let more = client.block_on(client.inner().billing().packages().list())?;
# Ok(())
# }
```

## Forward compatibility

Every enum in this crate is `#[non_exhaustive]` and has an `Unknown(String)`
variant. When the API adds a value, such as a new outcome, older versions of
the crate keep parsing responses and hand you the raw string. Include a
catch-all arm when you match.

## License

MIT. See [LICENSE](LICENSE). Report security problems as described in
[SECURITY.md](SECURITY.md).
