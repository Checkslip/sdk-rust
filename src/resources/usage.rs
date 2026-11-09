use crate::client::Client;
use crate::error::Result;
use crate::models::{Usage as UsageReport, UsageParams};

/// How many credits the account used, grouped over time. Get one from
/// [`Client::usage`].
#[derive(Debug, Clone)]
pub struct Usage {
    client: Client,
}

impl Usage {
    pub(crate) fn new(client: Client) -> Self {
        Usage { client }
    }

    /// Credit usage between two days, counted per interval.
    pub async fn get(&self, params: &UsageParams) -> Result<UsageReport> {
        self.client
            .get_json(&["api", "usage"], params.to_query())
            .await
    }
}
