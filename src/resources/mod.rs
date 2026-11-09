//! Groups of API calls. Reach them from [`Client`](crate::Client), for
//! example `client.slips()` or `client.webhooks().endpoints()`.

mod audit;
mod billing;
mod slips;
mod webhooks;

pub use audit::Audit;
pub use billing::Billing;
pub use slips::{LookupRequest, Slips};
pub use webhooks::{Deliveries, Endpoints, Webhooks};
