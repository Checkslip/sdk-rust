use reqwest::Method;
use uuid::Uuid;

use crate::client::{Client, RequestSpec};
use crate::error::Result;
use crate::models::{
    Balance, BillingInfo, CreditEntry, LedgerParams, Order, OrdersParams, PackageList, Page,
    Receipt,
};
use crate::pagination::{paginate, ItemStream};

/// Credits, orders, packages and receipts. Get one from
/// [`Client::billing`].
#[derive(Debug, Clone)]
pub struct Billing {
    client: Client,
}

impl Billing {
    pub(crate) fn new(client: Client) -> Self {
        Billing { client }
    }

    /// The credit left on this account.
    pub fn balance(&self) -> BillingBalance {
        BillingBalance {
            client: self.client.clone(),
        }
    }

    /// The details printed on receipts.
    pub fn info(&self) -> BillingDetails {
        BillingDetails {
            client: self.client.clone(),
        }
    }

    /// The credit packages on sale.
    pub fn packages(&self) -> BillingPackages {
        BillingPackages {
            client: self.client.clone(),
        }
    }

    /// Every top-up, charge and expiry.
    pub fn ledger(&self) -> BillingLedger {
        BillingLedger {
            client: self.client.clone(),
        }
    }

    /// Orders for credit packages.
    pub fn orders(&self) -> BillingOrders {
        BillingOrders {
            client: self.client.clone(),
        }
    }

    /// Receipts for paid orders.
    pub fn receipts(&self) -> BillingReceipts {
        BillingReceipts {
            client: self.client.clone(),
        }
    }
}

/// The credit left on this account.
#[derive(Debug, Clone)]
pub struct BillingBalance {
    client: Client,
}

impl BillingBalance {
    /// Credits left and when they expire.
    pub async fn get(&self) -> Result<Balance> {
        self.client
            .get_json(&["api", "billing", "balance"], Vec::new())
            .await
    }
}

/// The details printed on receipts.
#[derive(Debug, Clone)]
pub struct BillingDetails {
    client: Client,
}

impl BillingDetails {
    /// The details printed on receipts.
    pub async fn get(&self) -> Result<BillingInfo> {
        self.client
            .get_json(&["api", "billing", "billing-info"], Vec::new())
            .await
    }

    /// Replaces the details printed on receipts. Receipts already issued keep
    /// the details they were made with.
    pub async fn update(&self, info: &BillingInfo) -> Result<BillingInfo> {
        let spec = RequestSpec::new(Method::PUT, &["api", "billing", "billing-info"]).json(info)?;
        self.client.send_json(spec).await
    }
}

/// The credit packages on sale.
#[derive(Debug, Clone)]
pub struct BillingPackages {
    client: Client,
}

impl BillingPackages {
    /// Packages on sale and their prices.
    ///
    /// The answer is kept whole rather than unwrapped to its items, because it
    /// also says whether prices include VAT and whether checkout is open.
    pub async fn list(&self) -> Result<PackageList> {
        self.client
            .get_json(&["api", "billing", "packages"], Vec::new())
            .await
    }
}

/// Every change to the credit balance.
#[derive(Debug, Clone)]
pub struct BillingLedger {
    client: Client,
}

impl BillingLedger {
    /// One page of credit changes, newest first.
    pub async fn list(&self, params: &LedgerParams) -> Result<Page<CreditEntry>> {
        self.client
            .get_json(&["api", "billing", "ledger"], params.to_query())
            .await
    }

    /// Every credit change matching `params`, fetching each page as it goes.
    pub fn list_all(&self, params: LedgerParams) -> ItemStream<CreditEntry> {
        let this = self.clone();
        paginate(params.cursor.clone(), move |cursor| {
            let this = this.clone();
            let mut p = params.clone();
            p.cursor = cursor;
            async move { this.list(&p).await }
        })
    }
}

/// Orders for credit packages.
#[derive(Debug, Clone)]
pub struct BillingOrders {
    client: Client,
}

impl BillingOrders {
    /// One page of orders.
    pub async fn list(&self, params: &OrdersParams) -> Result<Page<Order>> {
        self.client
            .get_json(&["api", "billing", "orders"], params.to_query())
            .await
    }

    /// Every order matching `params`.
    pub fn list_all(&self, params: OrdersParams) -> ItemStream<Order> {
        let this = self.clone();
        paginate(params.cursor.clone(), move |cursor| {
            let this = this.clone();
            let mut p = params.clone();
            p.cursor = cursor;
            async move { this.list(&p).await }
        })
    }

    /// One order.
    pub async fn get(&self, order_id: Uuid) -> Result<Order> {
        self.client
            .get_json(
                &["api", "billing", "orders", &order_id.to_string()],
                Vec::new(),
            )
            .await
    }

    /// Asks the payment provider for the latest state of an order.
    pub async fn sync(&self, order_id: Uuid) -> Result<Order> {
        let spec = RequestSpec::new(
            Method::POST,
            &["api", "billing", "orders", &order_id.to_string(), "sync"],
        );
        self.client.send_json(spec).await
    }
}

/// Receipts for paid orders.
#[derive(Debug, Clone)]
pub struct BillingReceipts {
    client: Client,
}

impl BillingReceipts {
    /// One page of receipts.
    pub async fn list(&self, limit: Option<u32>, cursor: Option<String>) -> Result<Page<Receipt>> {
        let mut q = Vec::new();
        crate::query::push_opt(&mut q, "limit", &limit);
        crate::query::push_opt(&mut q, "cursor", &cursor);
        self.client
            .get_json(&["api", "billing", "receipts"], q)
            .await
    }

    /// Every receipt.
    pub fn list_all(&self) -> ItemStream<Receipt> {
        let this = self.clone();
        paginate(None, move |cursor| {
            let this = this.clone();
            async move { this.list(Some(100), cursor).await }
        })
    }

    /// One receipt.
    pub async fn get(&self, receipt_id: Uuid) -> Result<Receipt> {
        self.client
            .get_json(
                &["api", "billing", "receipts", &receipt_id.to_string()],
                Vec::new(),
            )
            .await
    }
}
