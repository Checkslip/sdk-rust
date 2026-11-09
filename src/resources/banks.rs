use serde::Deserialize;

use crate::client::Client;
use crate::error::Result;
use crate::models::SupportedBank;

/// Banks Checkslip can look up transfers with. Get one from
/// [`Client::banks`].
#[derive(Debug, Clone)]
pub struct Banks {
    client: Client,
}

#[derive(Deserialize)]
struct BankList {
    items: Vec<SupportedBank>,
}

impl Banks {
    pub(crate) fn new(client: Client) -> Self {
        Banks { client }
    }

    /// The banks Checkslip supports, with how far back each one lets us look.
    ///
    /// The API wraps the list in an object with nothing else in it, so the SDK
    /// hands back the banks themselves.
    pub async fn list(&self) -> Result<Vec<SupportedBank>> {
        let body: BankList = self.client.get_json(&["api", "banks"], Vec::new()).await?;
        Ok(body.items)
    }
}
