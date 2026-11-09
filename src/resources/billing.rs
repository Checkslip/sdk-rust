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

    /// Credits left and when they expire.
    pub async fn balance(&self) -> Result<Balance> {
        self.client
            .get_json(&["api", "billing", "balance"], Vec::new())
            .await
    }

    /// One page of credit changes, newest first.
    pub async fn ledger(&self, params: &LedgerParams) -> Result<Page<CreditEntry>> {
        self.client
            .get_json(&["api", "billing", "ledger"], params.to_query())
            .await
    }

    /// Every credit change matching `params`.
    pub fn ledger_all(&self, params: LedgerParams) -> ItemStream<CreditEntry> {
        let this = self.clone();
        paginate(params.cursor.clone(), move |cursor| {
            let this = this.clone();
            let mut p = params.clone();
            p.cursor = cursor;
            async move { this.ledger(&p).await }
        })
    }

    /// Packages on sale and their prices.
    pub async fn packages(&self) -> Result<PackageList> {
        self.client
            .get_json(&["api", "billing", "packages"], Vec::new())
            .await
    }

    /// One page of orders.
    pub async fn orders(&self, params: &OrdersParams) -> Result<Page<Order>> {
        self.client
            .get_json(&["api", "billing", "orders"], params.to_query())
            .await
    }

    /// Every order matching `params`.
    pub fn orders_all(&self, params: OrdersParams) -> ItemStream<Order> {
        let this = self.clone();
        paginate(params.cursor.clone(), move |cursor| {
            let this = this.clone();
            let mut p = params.clone();
            p.cursor = cursor;
            async move { this.orders(&p).await }
        })
    }

    /// One order.
    pub async fn order(&self, order_id: Uuid) -> Result<Order> {
        self.client
            .get_json(
                &["api", "billing", "orders", &order_id.to_string()],
                Vec::new(),
            )
            .await
    }

    /// Asks the payment provider for the latest state of an order.
    pub async fn sync_order(&self, order_id: Uuid) -> Result<Order> {
        let spec = RequestSpec::new(
            Method::POST,
            &["api", "billing", "orders", &order_id.to_string(), "sync"],
        );
        self.client.send_json(spec).await
    }

    /// One page of receipts.
    pub async fn receipts(
        &self,
        limit: Option<u32>,
        cursor: Option<String>,
    ) -> Result<Page<Receipt>> {
        let mut q = Vec::new();
        crate::query::push_opt(&mut q, "limit", &limit);
        crate::query::push_opt(&mut q, "cursor", &cursor);
        self.client
            .get_json(&["api", "billing", "receipts"], q)
            .await
    }

    /// Every receipt.
    pub fn receipts_all(&self) -> ItemStream<Receipt> {
        let this = self.clone();
        paginate(None, move |cursor| {
            let this = this.clone();
            async move { this.receipts(Some(100), cursor).await }
        })
    }

    /// One receipt.
    pub async fn receipt(&self, receipt_id: Uuid) -> Result<Receipt> {
        self.client
            .get_json(
                &["api", "billing", "receipts", &receipt_id.to_string()],
                Vec::new(),
            )
            .await
    }

    /// The details printed on receipts.
    pub async fn billing_info(&self) -> Result<BillingInfo> {
        self.client
            .get_json(&["api", "billing", "billing-info"], Vec::new())
            .await
    }

    /// Replaces the details printed on receipts.
    pub async fn set_billing_info(&self, info: &BillingInfo) -> Result<BillingInfo> {
        let spec = RequestSpec::new(Method::PUT, &["api", "billing", "billing-info"]).json(info)?;
        self.client.send_json(spec).await
    }
}
