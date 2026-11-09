//! The HTTP client, its builder, and the retry loop.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use reqwest::header::{
    HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, RETRY_AFTER, USER_AGENT,
};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use url::Url;

use crate::error::{Error, Problem, Result};
use crate::models::{SupportedBank, Usage, UsageParams};
use crate::query::Query;
use crate::resources::{Audit, Billing, Slips, Webhooks};

/// The production API.
pub const DEFAULT_BASE_URL: &str = "https://api.checkslip.com";

/// Per-request timeout used unless you set one.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Retries after the first attempt, unless you set a number.
pub const DEFAULT_MAX_RETRIES: u32 = 2;

const SDK_USER_AGENT: &str = concat!("checkslip-rust/", env!("CARGO_PKG_VERSION"));
const IDEMPOTENCY_KEY: &str = "idempotency-key";
const REQUEST_ID: &str = "x-request-id";

/// Async client for the Checkslip API.
///
/// Cloning is cheap: clones share one connection pool. Build one client and
/// reuse it.
///
/// ```no_run
/// # async fn run() -> checkslip::Result<()> {
/// use checkslip::{Client, LookupRequest};
///
/// let client = Client::new("csk_live_...")?;
/// let check = client
///     .slips()
///     .lookup(LookupRequest::qr("00330006000001010306902126302105340875102TH9104F86F"))
///     .await?;
/// println!("{} {}", check.request_id, check.outcome);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

struct Inner {
    http: reqwest::Client,
    base_url: Url,
    api_key: Option<HeaderValue>,
    user_agent: HeaderValue,
    timeout: Duration,
    retry: RetryPolicy,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RetryPolicy {
    pub(crate) max_retries: u32,
    pub(crate) base_delay: Duration,
    pub(crate) max_delay: Duration,
    pub(crate) max_retry_after: Duration,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.inner.base_url.as_str())
            .field("api_key", &self.inner.api_key.as_ref().map(|_| "csk_***"))
            .field("timeout", &self.inner.timeout)
            .field("max_retries", &self.inner.retry.max_retries)
            .finish()
    }
}

impl Client {
    /// A client for the production API with default settings.
    ///
    /// Fails only if the key cannot be sent as a header (for example it
    /// contains a newline) or the TLS backend cannot start.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Client::builder().api_key(api_key).build()
    }

    /// A client using the `CHECKSLIP_API_KEY` environment variable.
    pub fn from_env() -> Result<Self> {
        let key = std::env::var("CHECKSLIP_API_KEY")
            .map_err(|_| Error::InvalidArgument("CHECKSLIP_API_KEY is not set".to_owned()))?;
        Client::new(key)
    }

    /// Starts a [`ClientBuilder`].
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Slip lookups and lookup history.
    pub fn slips(&self) -> Slips {
        Slips::new(self.clone())
    }

    /// Credits, orders, packages and receipts.
    pub fn billing(&self) -> Billing {
        Billing::new(self.clone())
    }

    /// Webhook endpoints, deliveries and event types.
    pub fn webhooks(&self) -> Webhooks {
        Webhooks::new(self.clone())
    }

    /// Audit log, API request log and evidence exports.
    pub fn audit(&self) -> Audit {
        Audit::new(self.clone())
    }

    /// Banks Checkslip can look up transfers from. Needs no API key.
    pub async fn banks(&self) -> Result<Vec<SupportedBank>> {
        #[derive(serde::Deserialize)]
        struct List {
            items: Vec<SupportedBank>,
        }
        let list: List = self.get_json(&["api", "banks"], Vec::new()).await?;
        Ok(list.items)
    }

    /// Lookup and credit counts per day or month, in Bangkok time.
    pub async fn usage(&self, params: &UsageParams) -> Result<Usage> {
        self.get_json(&["api", "usage"], params.to_query()).await
    }

    // ----- plumbing used by the resource modules -----

    pub(crate) async fn get_json<T: DeserializeOwned>(
        &self,
        path: &[&str],
        query: Query,
    ) -> Result<T> {
        self.send_json(RequestSpec::new(Method::GET, path).query(query))
            .await
    }

    pub(crate) async fn send_json<T: DeserializeOwned>(&self, spec: RequestSpec) -> Result<T> {
        let resp = self.execute(spec).await?;
        resp.json()
    }

    pub(crate) async fn send_empty(&self, spec: RequestSpec) -> Result<()> {
        self.execute(spec).await.map(|_| ())
    }

    pub(crate) async fn send_bytes(&self, spec: RequestSpec) -> Result<Bytes> {
        self.execute(spec).await.map(|r| r.body)
    }

    fn url(&self, path: &[&str], query: &Query) -> Result<Url> {
        let mut url = self.inner.base_url.clone();
        {
            let mut segs = url
                .path_segments_mut()
                .map_err(|_| Error::InvalidArgument("base URL cannot have a path".to_owned()))?;
            segs.pop_if_empty();
            segs.extend(path);
        }
        if !query.is_empty() {
            url.query_pairs_mut()
                .extend_pairs(query.iter().map(|(k, v)| (*k, v.as_str())));
        }
        Ok(url)
    }

    pub(crate) async fn execute(&self, spec: RequestSpec) -> Result<RawResponse> {
        let url = self.url(
            &spec.path.iter().map(String::as_str).collect::<Vec<_>>(),
            &spec.query,
        )?;
        let policy = self.inner.retry;
        let replay_safe = spec.replay_safe();
        let mut attempt: u32 = 0;

        loop {
            let req = self.build(&spec, url.clone())?;
            let can_retry = attempt < policy.max_retries;

            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    if can_retry && replay_safe && is_transient(&e) {
                        tokio::time::sleep(policy.backoff(attempt)).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(Error::Http(e));
                }
            };

            let status = resp.status();
            let headers = resp.headers().clone();
            let request_id = header_str(&headers, REQUEST_ID);
            let body = resp.bytes().await.map_err(Error::Http)?;

            if status.is_success() {
                return Ok(RawResponse { body, request_id });
            }

            let retry_after = parse_retry_after(&headers);
            let problem: Problem = serde_json::from_slice(&body).unwrap_or_default();

            if status == StatusCode::TOO_MANY_REQUESTS {
                let wait = retry_after.unwrap_or_else(|| policy.backoff(attempt));
                if can_retry && wait <= policy.max_retry_after {
                    tokio::time::sleep(wait).await;
                    attempt += 1;
                    continue;
                }
                return Err(Error::RateLimited {
                    retry_after,
                    request_id,
                });
            }

            let retryable_status = status.is_server_error()
                || (status == StatusCode::CONFLICT
                    && spec.idempotency_key.is_some()
                    && problem.code.as_deref() == Some("IDEMPOTENCY_IN_FLIGHT"));
            if can_retry && replay_safe && retryable_status {
                let wait = retry_after
                    .filter(|d| *d <= policy.max_retry_after)
                    .unwrap_or_else(|| policy.backoff(attempt));
                tokio::time::sleep(wait).await;
                attempt += 1;
                continue;
            }

            return Err(api_error(status, problem, request_id));
        }
    }

    fn build(&self, spec: &RequestSpec, url: Url) -> Result<reqwest::RequestBuilder> {
        let inner = &self.inner;
        let mut req = inner
            .http
            .request(spec.method.clone(), url)
            .timeout(inner.timeout)
            .header(USER_AGENT, inner.user_agent.clone())
            .header("accept", "application/json, application/problem+json");
        if let Some(key) = &inner.api_key {
            req = req.header(AUTHORIZATION, key.clone());
        }
        if let Some(key) = &spec.idempotency_key {
            let v = HeaderValue::from_str(key).map_err(|_| {
                Error::InvalidArgument(
                    "Idempotency-Key must be 1 to 255 printable ASCII characters".to_owned(),
                )
            })?;
            req = req.header(IDEMPOTENCY_KEY, v);
        }
        req = match &spec.body {
            Body::Empty => req,
            Body::Json {
                bytes,
                content_type,
            } => req.header(CONTENT_TYPE, *content_type).body(bytes.clone()),
            Body::Multipart(m) => req.multipart(m.form()?),
        };
        Ok(req)
    }
}

fn api_error(status: StatusCode, problem: Problem, request_id: Option<String>) -> Error {
    Error::Api {
        status: status.as_u16(),
        code: problem.code.clone().unwrap_or_default(),
        title: problem
            .title
            .clone()
            .unwrap_or_else(|| status.canonical_reason().unwrap_or("error").to_owned()),
        detail: problem.detail.clone(),
        request_id,
        problem: Box::new(problem),
    }
}

fn is_transient(e: &reqwest::Error) -> bool {
    e.is_connect() || e.is_timeout() || e.is_request() || e.is_body()
}

fn header_str(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

/// Reads `Retry-After` as whole seconds. The API sends seconds; an HTTP
/// date is ignored and the client falls back to its own backoff.
pub(crate) fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    headers
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

impl RetryPolicy {
    /// Exponential backoff with jitter: a random wait between half and all
    /// of `base * 2^attempt`, capped at `max_delay`.
    pub(crate) fn backoff(&self, attempt: u32) -> Duration {
        let exp = self
            .base_delay
            .saturating_mul(2u32.saturating_pow(attempt.min(16)));
        let cap = exp.min(self.max_delay);
        let half = cap / 2;
        let jitter_ms = fastrand::u64(0..=half.as_millis() as u64);
        half + Duration::from_millis(jitter_ms)
    }
}

pub(crate) struct RawResponse {
    pub(crate) body: Bytes,
    pub(crate) request_id: Option<String>,
}

impl RawResponse {
    pub(crate) fn json<T: DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_slice(&self.body).map_err(|source| Error::Decode {
            source,
            request_id: self.request_id.clone(),
        })
    }
}

pub(crate) enum Body {
    Empty,
    Json {
        bytes: Bytes,
        content_type: &'static str,
    },
    Multipart(MultipartImage),
}

/// A multipart body kept as bytes so it can be rebuilt for each retry.
pub(crate) struct MultipartImage {
    pub(crate) image: Bytes,
    pub(crate) file_name: String,
    pub(crate) content_type: String,
    pub(crate) client_reference: Option<String>,
}

impl MultipartImage {
    fn form(&self) -> Result<reqwest::multipart::Form> {
        let part = reqwest::multipart::Part::stream(self.image.clone())
            .file_name(self.file_name.clone())
            .mime_str(&self.content_type)
            .map_err(|_| {
                Error::InvalidArgument(format!("bad image content type {:?}", self.content_type))
            })?;
        let mut form = reqwest::multipart::Form::new().part("image", part);
        if let Some(r) = &self.client_reference {
            form = form.text("client_reference", r.clone());
        }
        Ok(form)
    }
}

pub(crate) struct RequestSpec {
    method: Method,
    path: Vec<String>,
    query: Query,
    body: Body,
    idempotency_key: Option<String>,
}

impl RequestSpec {
    pub(crate) fn new(method: Method, path: &[&str]) -> Self {
        RequestSpec {
            method,
            path: path.iter().map(|s| (*s).to_owned()).collect(),
            query: Vec::new(),
            body: Body::Empty,
            idempotency_key: None,
        }
    }

    pub(crate) fn query(mut self, query: Query) -> Self {
        self.query = query;
        self
    }

    pub(crate) fn json<B: serde::Serialize>(self, body: &B) -> Result<Self> {
        self.json_as(body, "application/json")
    }

    pub(crate) fn json_as<B: serde::Serialize>(
        mut self,
        body: &B,
        content_type: &'static str,
    ) -> Result<Self> {
        let bytes = serde_json::to_vec(body)
            .map_err(|e| Error::InvalidArgument(format!("could not encode body: {e}")))?;
        self.body = Body::Json {
            bytes: Bytes::from(bytes),
            content_type,
        };
        Ok(self)
    }

    pub(crate) fn multipart(mut self, m: MultipartImage) -> Self {
        self.body = Body::Multipart(m);
        self
    }

    /// Sets the key, or generates a random one when `key` is `None`.
    pub(crate) fn idempotency_key(mut self, key: Option<String>) -> Self {
        self.idempotency_key = Some(key.unwrap_or_else(new_idempotency_key));
        self
    }

    /// Whether sending this request twice is harmless.
    fn replay_safe(&self) -> bool {
        self.idempotency_key.is_some()
            || matches!(
                self.method,
                Method::GET | Method::HEAD | Method::PUT | Method::DELETE | Method::OPTIONS
            )
    }
}

/// A new random idempotency key (a UUID v4).
pub fn new_idempotency_key() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Builds a [`Client`].
///
/// ```
/// use std::time::Duration;
///
/// let client = checkslip::Client::builder()
///     .api_key("csk_test_...")
///     .timeout(Duration::from_secs(20))
///     .max_retries(3)
///     .user_agent("my-shop/1.4")
///     .build()
///     .unwrap();
/// # let _ = client;
/// ```
#[derive(Default)]
pub struct ClientBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    max_retries: Option<u32>,
    user_agent: Option<String>,
    http: Option<reqwest::Client>,
    retry_base_delay: Option<Duration>,
    max_retry_after: Option<Duration>,
}

impl fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("api_key", &self.api_key.as_ref().map(|_| "csk_***"))
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .field("user_agent", &self.user_agent)
            .finish_non_exhaustive()
    }
}

impl ClientBuilder {
    /// Your API key (`csk_...`). Without one, only endpoints that need no
    /// sign-in work, such as [`Client::banks`].
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Another API address, for example a mock server in tests.
    /// Defaults to [`DEFAULT_BASE_URL`].
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Timeout for each attempt. Defaults to [`DEFAULT_TIMEOUT`].
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// How many times to retry after the first attempt. `0` turns retries
    /// off. Defaults to [`DEFAULT_MAX_RETRIES`].
    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = Some(n);
        self
    }

    /// Your app's name and version. It is sent before the SDK's own
    /// `checkslip-rust/<version>` in the `User-Agent` header.
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Uses your own `reqwest::Client`, for proxies or custom TLS. The
    /// timeout from this builder still applies to each request.
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http = Some(client);
        self
    }

    /// First backoff step between retries. Defaults to 500 ms; it doubles
    /// on each retry up to 8 seconds.
    pub fn retry_base_delay(mut self, delay: Duration) -> Self {
        self.retry_base_delay = Some(delay);
        self
    }

    /// The longest `Retry-After` the client will wait out by itself.
    /// A longer one comes back as [`Error::RateLimited`] right away.
    /// Defaults to 60 seconds.
    pub fn max_retry_after(mut self, max: Duration) -> Self {
        self.max_retry_after = Some(max);
        self
    }

    /// Builds the client.
    pub fn build(self) -> Result<Client> {
        let base = self.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL);
        let base_url = Url::parse(base)
            .map_err(|e| Error::InvalidArgument(format!("bad base URL {base:?}: {e}")))?;
        if base_url.cannot_be_a_base() {
            return Err(Error::InvalidArgument(format!("bad base URL {base:?}")));
        }

        let api_key = match self.api_key {
            Some(k) => {
                let k = k.trim();
                if k.is_empty() {
                    return Err(Error::InvalidArgument("API key is empty".to_owned()));
                }
                let mut v = HeaderValue::from_str(&format!("Bearer {k}")).map_err(|_| {
                    Error::InvalidArgument("API key has characters a header cannot hold".to_owned())
                })?;
                v.set_sensitive(true);
                Some(v)
            }
            None => None,
        };

        let ua = match &self.user_agent {
            Some(app) => format!("{app} {SDK_USER_AGENT}"),
            None => SDK_USER_AGENT.to_owned(),
        };
        let user_agent = HeaderValue::from_str(&ua)
            .map_err(|_| Error::InvalidArgument("bad user agent".to_owned()))?;

        let http = match self.http {
            Some(c) => c,
            None => reqwest::Client::builder().build().map_err(Error::Http)?,
        };

        Ok(Client {
            inner: Arc::new(Inner {
                http,
                base_url,
                api_key,
                user_agent,
                timeout: self.timeout.unwrap_or(DEFAULT_TIMEOUT),
                retry: RetryPolicy {
                    max_retries: self.max_retries.unwrap_or(DEFAULT_MAX_RETRIES),
                    base_delay: self.retry_base_delay.unwrap_or(Duration::from_millis(500)),
                    max_delay: Duration::from_secs(8),
                    max_retry_after: self.max_retry_after.unwrap_or(Duration::from_secs(60)),
                },
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_stays_in_range() {
        let p = RetryPolicy {
            max_retries: 5,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(8),
            max_retry_after: Duration::from_secs(60),
        };
        for attempt in 0..10 {
            let cap = (Duration::from_millis(500) * 2u32.pow(attempt.min(16)))
                .min(Duration::from_secs(8));
            for _ in 0..50 {
                let d = p.backoff(attempt);
                assert!(d >= cap / 2 && d <= cap, "attempt {attempt}: {d:?}");
            }
        }
    }

    #[test]
    fn debug_hides_the_key() {
        let c = Client::new("csk_live_supersecret").unwrap();
        let s = format!("{c:?}");
        assert!(!s.contains("supersecret"), "{s}");
    }

    #[test]
    fn rejects_bad_keys() {
        assert!(matches!(
            Client::new("csk_bad\nkey"),
            Err(Error::InvalidArgument(_))
        ));
        assert!(matches!(Client::new("  "), Err(Error::InvalidArgument(_))));
    }

    #[test]
    fn url_encodes_path_segments() {
        let c = Client::builder()
            .base_url("http://localhost:1234/prefix/")
            .build()
            .unwrap();
        let u = c
            .url(&["api", "slips", "a/b c"], &vec![("x", "1 2".into())])
            .unwrap();
        assert_eq!(
            u.as_str(),
            "http://localhost:1234/prefix/api/slips/a%2Fb%20c?x=1+2"
        );
    }
}
