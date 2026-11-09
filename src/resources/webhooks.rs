use reqwest::Method;

use crate::client::{Client, RequestSpec};
use crate::error::Result;
use crate::models::{
    CreateEndpoint, Delivery, DeliveryDetail, Endpoint, EndpointWithSecret, EventTypeInfo,
    ListDeliveriesParams, Page, RecoverRequest, RecoverResult, SecretRotation, UpdateEndpoint,
};
use crate::pagination::{paginate, ItemStream};

/// Webhook management. Get one from [`Client::webhooks`].
///
/// To check the signature on messages you receive, see the
/// [`webhook`](crate::webhook) module (feature `webhooks`).
#[derive(Debug, Clone)]
pub struct Webhooks {
    client: Client,
}

impl Webhooks {
    pub(crate) fn new(client: Client) -> Self {
        Webhooks { client }
    }

    /// Endpoints that receive messages.
    pub fn endpoints(&self) -> Endpoints {
        Endpoints {
            client: self.client.clone(),
        }
    }

    /// Messages sent to your endpoints.
    pub fn deliveries(&self) -> Deliveries {
        Deliveries {
            client: self.client.clone(),
        }
    }

    /// Event types you can subscribe to.
    pub async fn event_types(&self) -> Result<Vec<EventTypeInfo>> {
        #[derive(serde::Deserialize)]
        struct List {
            items: Vec<EventTypeInfo>,
        }
        let l: List = self
            .client
            .get_json(&["api", "webhooks", "event-types"], Vec::new())
            .await?;
        Ok(l.items)
    }
}

/// Webhook endpoints. Get one from [`Webhooks::endpoints`].
///
/// The calls that take `idempotency_key: Option<&str>` send an
/// `Idempotency-Key`. Pass `None` and the client makes one per call.
#[derive(Debug, Clone)]
pub struct Endpoints {
    client: Client,
}

const ENDPOINTS: [&str; 3] = ["api", "webhooks", "endpoints"];

impl Endpoints {
    /// Registers a URL. The response holds the signing secret, shown only
    /// this once.
    pub async fn create(&self, req: &CreateEndpoint) -> Result<EndpointWithSecret> {
        let spec = RequestSpec::new(Method::POST, &ENDPOINTS).json(req)?;
        self.client.send_json(spec).await
    }

    /// One page of endpoints.
    pub async fn list(&self, limit: Option<u32>, cursor: Option<String>) -> Result<Page<Endpoint>> {
        let mut q = Vec::new();
        crate::query::push_opt(&mut q, "limit", &limit);
        crate::query::push_opt(&mut q, "cursor", &cursor);
        self.client.get_json(&ENDPOINTS, q).await
    }

    /// Every endpoint.
    pub fn list_all(&self) -> ItemStream<Endpoint> {
        let this = self.clone();
        paginate(None, move |cursor| {
            let this = this.clone();
            async move { this.list(Some(100), cursor).await }
        })
    }

    /// One endpoint.
    pub async fn get(&self, endpoint_id: &str) -> Result<Endpoint> {
        self.client
            .get_json(&path(&[endpoint_id]), Vec::new())
            .await
    }

    /// Changes an endpoint. Only the fields you set change.
    pub async fn update(&self, endpoint_id: &str, patch: &UpdateEndpoint) -> Result<Endpoint> {
        let spec = RequestSpec::new(Method::PATCH, &path(&[endpoint_id]))
            .json_as(patch, "application/merge-patch+json")?;
        self.client.send_json(spec).await
    }

    /// Deletes an endpoint.
    pub async fn delete(&self, endpoint_id: &str) -> Result<()> {
        self.client
            .send_empty(RequestSpec::new(Method::DELETE, &path(&[endpoint_id])))
            .await
    }

    /// Resends failed messages from a time window.
    pub async fn recover(
        &self,
        endpoint_id: &str,
        req: &RecoverRequest,
        idempotency_key: Option<&str>,
    ) -> Result<RecoverResult> {
        let spec = RequestSpec::new(Method::POST, &path(&[endpoint_id, "recover"]))
            .idempotency_key(idempotency_key.map(str::to_owned))
            .json(req)?;
        self.client.send_json(spec).await
    }

    /// Issues a new signing secret. The old one keeps signing for 24 hours,
    /// so messages carry both signatures until then.
    pub async fn rotate_secret(
        &self,
        endpoint_id: &str,
        idempotency_key: Option<&str>,
    ) -> Result<SecretRotation> {
        let spec = RequestSpec::new(Method::POST, &path(&[endpoint_id, "secret", "rotate"]))
            .idempotency_key(idempotency_key.map(str::to_owned));
        self.client.send_json(spec).await
    }

    /// Sends a `webhook.test` message to the endpoint.
    pub async fn send_test(
        &self,
        endpoint_id: &str,
        idempotency_key: Option<&str>,
    ) -> Result<Delivery> {
        let spec = RequestSpec::new(Method::POST, &path(&[endpoint_id, "test"]))
            .idempotency_key(idempotency_key.map(str::to_owned));
        self.client.send_json(spec).await
    }
}

fn path<'a>(rest: &[&'a str]) -> Vec<&'a str> {
    let mut p: Vec<&str> = ENDPOINTS.to_vec();
    p.extend_from_slice(rest);
    p
}

/// Webhook deliveries. Get one from [`Webhooks::deliveries`].
#[derive(Debug, Clone)]
pub struct Deliveries {
    client: Client,
}

impl Deliveries {
    /// One page of deliveries, newest first.
    pub async fn list(&self, params: &ListDeliveriesParams) -> Result<Page<Delivery>> {
        self.client
            .get_json(&["api", "webhooks", "deliveries"], params.to_query())
            .await
    }

    /// Every delivery matching `params`.
    pub fn list_all(&self, params: ListDeliveriesParams) -> ItemStream<Delivery> {
        let this = self.clone();
        paginate(params.cursor.clone(), move |cursor| {
            let this = this.clone();
            let mut p = params.clone();
            p.cursor = cursor;
            async move { this.list(&p).await }
        })
    }

    /// One delivery with its payload and attempts.
    pub async fn get(&self, delivery_id: &str) -> Result<DeliveryDetail> {
        self.client
            .get_json(&["api", "webhooks", "deliveries", delivery_id], Vec::new())
            .await
    }

    /// Sends a delivery again with the same `webhook-id`. Deduplicate on
    /// `webhook-id` on your side, or you will record it twice.
    pub async fn redeliver(
        &self,
        delivery_id: &str,
        idempotency_key: Option<&str>,
    ) -> Result<Delivery> {
        let spec = RequestSpec::new(
            Method::POST,
            &["api", "webhooks", "deliveries", delivery_id, "redeliver"],
        )
        .idempotency_key(idempotency_key.map(str::to_owned));
        self.client.send_json(spec).await
    }
}
