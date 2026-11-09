use bytes::Bytes;
use reqwest::Method;
use uuid::Uuid;

use crate::client::{Client, RequestSpec};
use crate::error::Result;
use crate::models::{
    ApiRequest, ApiRequestsParams, AuditEvent, AuditEventsParams, CreateEvidenceExport,
    EvidenceExport, Page,
};
use crate::pagination::{paginate, ItemStream};

/// The audit log, the API request log and evidence exports. Get one from
/// [`Client::audit`]. Needs an API key with `audit:read`.
#[derive(Debug, Clone)]
pub struct Audit {
    client: Client,
}

impl Audit {
    pub(crate) fn new(client: Client) -> Self {
        Audit { client }
    }

    /// Account activity, such as an API key created or a member removed.
    pub fn events(&self) -> AuditEvents {
        AuditEvents {
            client: self.client.clone(),
        }
    }

    /// Requests this account sent.
    pub fn requests(&self) -> AuditRequests {
        AuditRequests {
            client: self.client.clone(),
        }
    }

    /// Evidence files for an auditor, a bank or a court.
    pub fn exports(&self) -> AuditExports {
        AuditExports {
            client: self.client.clone(),
        }
    }
}

/// Account activity.
#[derive(Debug, Clone)]
pub struct AuditEvents {
    client: Client,
}

impl AuditEvents {
    /// One page of audit events, newest first.
    pub async fn list(&self, params: &AuditEventsParams) -> Result<Page<AuditEvent>> {
        self.client
            .get_json(&["api", "audit", "events"], params.to_query())
            .await
    }
    /// Every audit event matching `params`.
    pub fn list_all(&self, params: AuditEventsParams) -> ItemStream<AuditEvent> {
        let this = self.clone();
        paginate(params.cursor.clone(), move |cursor| {
            let this = this.clone();
            let mut p = params.clone();
            p.cursor = cursor;
            async move { this.list(&p).await }
        })
    }
    /// One audit event.
    pub async fn get(&self, event_id: Uuid) -> Result<AuditEvent> {
        self.client
            .get_json(
                &["api", "audit", "events", &event_id.to_string()],
                Vec::new(),
            )
            .await
    }
}

/// Requests this account sent.
#[derive(Debug, Clone)]
pub struct AuditRequests {
    client: Client,
}

impl AuditRequests {
    /// One page of API requests this account made, newest first.
    pub async fn list(&self, params: &ApiRequestsParams) -> Result<Page<ApiRequest>> {
        self.client
            .get_json(&["api", "audit", "requests"], params.to_query())
            .await
    }
    /// Every API request matching `params`.
    pub fn list_all(&self, params: ApiRequestsParams) -> ItemStream<ApiRequest> {
        let this = self.clone();
        paginate(params.cursor.clone(), move |cursor| {
            let this = this.clone();
            let mut p = params.clone();
            p.cursor = cursor;
            async move { this.list(&p).await }
        })
    }
    /// One API request, by its `X-Request-Id`.
    pub async fn get(&self, request_id: Uuid) -> Result<ApiRequest> {
        self.client
            .get_json(
                &["api", "audit", "requests", &request_id.to_string()],
                Vec::new(),
            )
            .await
    }
}

/// Evidence files for an auditor, a bank or a court.
#[derive(Debug, Clone)]
pub struct AuditExports {
    client: Client,
}

impl AuditExports {
    /// Starts an evidence export. Poll [`export`](Self::export) until its
    /// status is `READY`, then [`download_export`](Self::download_export).
    pub async fn create(
        &self,
        req: &CreateEvidenceExport,
        idempotency_key: Option<&str>,
    ) -> Result<EvidenceExport> {
        let spec = RequestSpec::new(Method::POST, &["api", "audit", "exports"])
            .idempotency_key(idempotency_key.map(str::to_owned))
            .json(req)?;
        self.client.send_json(spec).await
    }
    /// One page of evidence exports.
    pub async fn list(
        &self,
        limit: Option<u32>,
        cursor: Option<String>,
    ) -> Result<Page<EvidenceExport>> {
        let mut q = Vec::new();
        crate::query::push_opt(&mut q, "limit", &limit);
        crate::query::push_opt(&mut q, "cursor", &cursor);
        self.client.get_json(&["api", "audit", "exports"], q).await
    }
    /// Every evidence export.
    pub fn list_all(&self) -> ItemStream<EvidenceExport> {
        let this = self.clone();
        paginate(None, move |cursor| {
            let this = this.clone();
            async move { this.list(Some(100), cursor).await }
        })
    }
    /// One evidence export.
    pub async fn get(&self, export_id: Uuid) -> Result<EvidenceExport> {
        self.client
            .get_json(
                &["api", "audit", "exports", &export_id.to_string()],
                Vec::new(),
            )
            .await
    }
    /// The export file, as NDJSON bytes.
    pub async fn download(&self, export_id: Uuid) -> Result<Bytes> {
        let spec = RequestSpec::new(
            Method::GET,
            &["api", "audit", "exports", &export_id.to_string(), "file"],
        );
        self.client.send_bytes(spec).await
    }
}
