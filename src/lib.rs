//! Rust client for the [Checkslip](https://checkslip.com) API.
//!
//! Checkslip looks up Thai bank transfers. You send the QR text from a
//! transfer slip, or the slip image, and Checkslip asks the sending bank for
//! the transfer it holds. You get back what the bank has (amount, time,
//! sender, receiver) and decide for yourself whether it matches your order.
//!
//! ```no_run
//! use checkslip::{Client, LookupRequest, Outcome};
//!
//! # async fn run() -> checkslip::Result<()> {
//! let client = Client::new(std::env::var("CHECKSLIP_API_KEY").unwrap())?;
//!
//! let check = client
//!     .slips()
//!     .lookup(LookupRequest::qr("00330006000001010306902126302105340875102TH9104F86F")
//!         .client_reference("order-1042"))
//!     .await?;
//!
//! match check.outcome {
//!     Outcome::Found => {
//!         let amount = check.slip.amount().map(|m| m.satang);
//!         println!("bank has the transfer, amount in satang: {amount:?}");
//!     }
//!     Outcome::Duplicate => println!(
//!         "already checked this slip, first at {}",
//!         check.duplicate_of.expect("DUPLICATE carries duplicate_of").checked_at
//!     ),
//!     Outcome::NotFound => println!("the bank has no such transfer"),
//!     other => println!("outcome {other}"),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # What the client does for you
//!
//! - Sends `Authorization: Bearer csk_...` on every call.
//! - Sends an `Idempotency-Key` on every lookup (a random one unless you
//!   set your own), so a retried lookup is never charged twice.
//! - Retries 429 responses after `Retry-After`, and retries network errors
//!   and 5xx responses when that is safe: on `GET`, `PUT` and `DELETE`, and
//!   on `POST` calls that carry an `Idempotency-Key`. Waits grow
//!   exponentially with random jitter. See [`ClientBuilder::max_retries`].
//! - Turns `application/problem+json` errors into [`Error::Api`] with the
//!   `code` to match on.
//! - Follows `next_cursor` for you in the `list_all` methods, which return a
//!   [`Stream`](futures_core::Stream).
//!
//! # Features
//!
//! | feature      | default | what it adds |
//! |--------------|---------|--------------|
//! | `rustls-tls` | yes     | TLS through rustls |
//! | `native-tls` | no      | TLS through the platform library (OpenSSL, Schannel, Secure Transport) |
//! | `webhooks`   | no      | [`webhook`]: signature checks for incoming webhook messages |
//! | `blocking`   | no      | [`blocking::Client`] for code without an async runtime |
//!
//! # Minimum Rust version
//!
//! Rust 1.75.

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
mod macros;

mod client;
mod dates;
mod error;
pub mod models;
mod pagination;
mod query;
pub mod resources;

#[cfg(feature = "blocking")]
#[cfg_attr(docsrs, doc(cfg(feature = "blocking")))]
pub mod blocking;

#[cfg(feature = "webhooks")]
#[cfg_attr(docsrs, doc(cfg(feature = "webhooks")))]
pub mod webhook;

pub use client::{
    new_idempotency_key, Client, ClientBuilder, DEFAULT_BASE_URL, DEFAULT_MAX_RETRIES,
    DEFAULT_TIMEOUT,
};
pub use dates::day;
pub use error::{Error, FieldError, Problem, Result};
pub use models::*;
pub use pagination::ItemStream;
pub use resources::LookupRequest;

// Compile the code samples in the READMEs as doc tests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

#[cfg(doctest)]
#[doc = include_str!("../README.th.md")]
struct ReadmeThDoctests;

/// The `time` crate, re-exported so you can build dates without adding it.
pub use time;
/// The `uuid` crate, re-exported for request IDs.
pub use uuid;
