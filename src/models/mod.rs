//! Request and response types.
//!
//! Field names match the API's JSON. Fields the API may leave out or send
//! as `null` are `Option`. Enums are `#[non_exhaustive]` and keep values
//! this version of the crate does not know in an `Unknown(String)` variant,
//! so a new value on the server does not break your build or your parsing.

mod audit;
mod billing;
pub(crate) mod common;
mod slips;
mod webhooks;

pub use audit::*;
pub use billing::*;
pub use common::{
    Bank, Channel, Currency, DuplicateOf, Money, Page, Party, Receiver, ReceiverKind, Slip,
    SlipData, SlipSummary,
};
pub use slips::*;
pub use webhooks::*;
