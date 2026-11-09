//! Errors returned by the client.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Shorthand for `Result<T, checkslip::Error>`.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Everything that can go wrong when calling the API.
///
/// The API answers errors as RFC 9457 `application/problem+json`. Those
/// become [`Error::Api`], or [`Error::RateLimited`] for HTTP 429. The
/// `code` field is the stable thing to match on; `title` and `detail` are
/// written for people and may change.
///
/// ```
/// # async fn run(client: checkslip::Client) {
/// use checkslip::{Error, LookupRequest};
///
/// match client.slips().lookup(LookupRequest::qr("00330006000001010306902126302105340875102TH9104F86F")).await {
///     Ok(check) => println!("outcome: {}", check.outcome),
///     Err(Error::Api { code, .. }) if code == "NO_CREDIT" => println!("top up first"),
///     Err(Error::RateLimited { retry_after, .. }) => println!("slow down: {retry_after:?}"),
///     Err(e) => println!("failed: {e}"),
/// }
/// # }
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The API answered with an error status and a problem body.
    #[error("Checkslip API error {status} {code}: {title}")]
    Api {
        /// HTTP status code, for example 402 or 422.
        status: u16,
        /// Machine-readable error code, for example `NO_CREDIT` or
        /// `INVALID_QR`. Empty when the server sent no problem body.
        code: String,
        /// Short summary of the problem. The API often writes it in Thai.
        title: String,
        /// Longer explanation, when the server sent one.
        detail: Option<String>,
        /// The `X-Request-Id` response header. Quote it when you contact
        /// support.
        request_id: Option<String>,
        /// The whole problem body, including field errors.
        problem: Box<Problem>,
    },

    /// HTTP 429: too many requests. The client already retried up to
    /// `max_retries` times before returning this.
    #[error("rate limited by the Checkslip API (retry after {retry_after:?})")]
    RateLimited {
        /// How long the server asked us to wait, from `Retry-After`.
        retry_after: Option<Duration>,
        /// The `X-Request-Id` response header.
        request_id: Option<String>,
    },

    /// The request never got a response: connection, TLS or timeout
    /// failure.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// The response arrived but its body did not match the expected shape.
    #[error("could not decode the response body: {source}")]
    Decode {
        /// The JSON error.
        #[source]
        source: serde_json::Error,
        /// The `X-Request-Id` response header.
        request_id: Option<String>,
    },

    /// A value given to the client cannot be sent, such as an API key with
    /// a newline in it or a base URL that does not parse.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

impl Error {
    /// The HTTP status, when the server answered.
    pub fn status(&self) -> Option<u16> {
        match self {
            Error::Api { status, .. } => Some(*status),
            Error::RateLimited { .. } => Some(429),
            Error::Http(e) => e.status().map(|s| s.as_u16()),
            _ => None,
        }
    }

    /// The problem `code`, such as `NO_CREDIT`, when there is one.
    pub fn code(&self) -> Option<&str> {
        match self {
            Error::Api { code, .. } if !code.is_empty() => Some(code),
            Error::RateLimited { .. } => Some("RATE_LIMITED"),
            _ => None,
        }
    }

    /// The `X-Request-Id` of the failed response, when there is one.
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Error::Api { request_id, .. }
            | Error::RateLimited { request_id, .. }
            | Error::Decode { request_id, .. } => request_id.as_deref(),
            _ => None,
        }
    }
}

/// An RFC 9457 problem body as the Checkslip API sends it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Problem {
    /// A URI naming the problem type, for example
    /// `https://checkslip.com/problems/no-credit`.
    #[serde(rename = "type", default)]
    pub type_uri: Option<String>,
    /// Short summary.
    #[serde(default)]
    pub title: Option<String>,
    /// HTTP status.
    #[serde(default)]
    pub status: Option<u16>,
    /// Machine-readable code, for example `NO_CREDIT`.
    #[serde(default)]
    pub code: Option<String>,
    /// Longer explanation.
    #[serde(default)]
    pub detail: Option<String>,
    /// Identifies this occurrence of the problem.
    #[serde(default)]
    pub instance: Option<String>,
    /// Field-level errors for validation failures.
    #[serde(default)]
    pub errors: Vec<FieldError>,
    /// Members this crate has no field for, such as `order_id`.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// One field-level error inside a [`Problem`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FieldError {
    /// JSON pointer to the field, for example `/qr`.
    pub pointer: String,
    /// What is wrong with it.
    pub code: String,
    /// Human-readable explanation.
    #[serde(default)]
    pub detail: Option<String>,
}
