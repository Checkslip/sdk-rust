use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use super::common::{date_format, Channel};
use crate::query::{push_opt, push_time, Query};

string_enum! {
    /// Who did something.
    pub enum ActorType {
        /// A signed-in user.
        User = "USER",
        /// An API key.
        ApiKey = "API_KEY",
        /// The Checkslip assistant.
        Assistant = "ASSISTANT",
        /// Checkslip staff.
        Staff = "STAFF",
        /// Checkslip itself.
        System = "SYSTEM",
    }
}

string_enum! {
    /// Kind of audit event.
    pub enum AuditCategory {
        /// Credits or payments.
        Money = "MONEY",
        /// Sign-in and access.
        Access = "ACCESS",
        /// A change to account data.
        DataChange = "DATA_CHANGE",
        /// A read of sensitive data.
        SensitiveRead = "SENSITIVE_READ",
        /// A security event.
        Security = "SECURITY",
    }
}

string_enum! {
    /// How an audited action ended.
    pub enum AuditOutcome {
        /// It worked.
        Success = "SUCCESS",
        /// It failed.
        Failure = "FAILURE",
        /// It was not allowed.
        Denied = "DENIED",
    }
}

/// Who did an audited action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Actor {
    /// Kind of actor.
    #[serde(rename = "type")]
    pub actor_type: ActorType,
    /// Actor ID.
    #[serde(default)]
    pub id: Option<Uuid>,
    /// The user an assistant or staff member acted for.
    #[serde(default)]
    pub on_behalf_of_user_id: Option<Uuid>,
    /// How strongly the actor was authenticated.
    #[serde(default)]
    pub assurance: Option<String>,
}

/// What an audited action touched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Target {
    /// Kind of thing, for example `api_key`.
    #[serde(rename = "type")]
    pub target_type: String,
    /// Its ID.
    pub id: String,
}

/// A field removed from a record, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Redaction {
    /// Path of the field.
    pub path: String,
    /// Why it was removed.
    pub reason: String,
}

/// One entry in the account's audit log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AuditEvent {
    /// Event ID.
    pub event_id: Uuid,
    /// Kind of event.
    pub category: AuditCategory,
    /// How it ended.
    pub outcome: AuditOutcome,
    /// When it happened.
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
    /// The event that caused this one.
    #[serde(default)]
    pub causation_id: Option<Uuid>,
    /// The API request that caused it.
    #[serde(default)]
    pub request_id: Option<Uuid>,
    /// Who did it.
    pub actor: Actor,
    /// Where it came from.
    pub channel: Channel,
    /// Dotted action name, for example `api_key.created`.
    pub action: String,
    /// What it touched.
    pub target: Target,
    /// State before, when recorded.
    #[serde(default)]
    pub before: Option<serde_json::Value>,
    /// State after, when recorded.
    #[serde(default)]
    pub after: Option<serde_json::Value>,
    /// Reason given.
    #[serde(default)]
    pub reason: Option<String>,
    /// The event this one undid.
    #[serde(default)]
    pub undo_of_event_id: Option<Uuid>,
    /// Fields removed from this record.
    #[serde(default)]
    pub redactions: Option<Vec<Redaction>>,
}

/// Filters for [`Audit::events`](crate::resources::Audit::events).
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct AuditEventsParams {
    /// Start of the range.
    pub from: Option<OffsetDateTime>,
    /// End of the range.
    pub to: Option<OffsetDateTime>,
    /// Action, or a prefix ending in `.*`.
    pub action: Option<String>,
    /// Kind of actor.
    pub actor_type: Option<ActorType>,
    /// Actor ID.
    pub actor_id: Option<Uuid>,
    /// Channel.
    pub channel: Option<Channel>,
    /// Kind of target.
    pub target_type: Option<String>,
    /// Target ID.
    pub target_id: Option<String>,
    /// Page size, 1 to 100.
    pub limit: Option<u32>,
    /// Cursor from a previous page.
    pub cursor: Option<String>,
}

impl AuditEventsParams {
    pub(crate) fn to_query(&self) -> Query {
        let mut q = Vec::new();
        push_time(&mut q, "from", self.from);
        push_time(&mut q, "to", self.to);
        push_opt(&mut q, "action", &self.action);
        push_opt(&mut q, "actor_type", &self.actor_type);
        push_opt(&mut q, "actor_id", &self.actor_id);
        push_opt(&mut q, "channel", &self.channel);
        push_opt(&mut q, "target_type", &self.target_type);
        push_opt(&mut q, "target_id", &self.target_id);
        push_opt(&mut q, "limit", &self.limit);
        push_opt(&mut q, "cursor", &self.cursor);
        q
    }
}

/// Request fingerprint: what the request was about, without personal data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Fingerprint {
    /// SHA-256 of the QR text.
    #[serde(default)]
    pub qr_sha256: Option<String>,
    /// Transaction reference.
    #[serde(default)]
    pub trans_ref: Option<String>,
}

/// One API request the account made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ApiRequest {
    /// Request ID, the same as the `X-Request-Id` header.
    pub request_id: Uuid,
    /// When it arrived.
    #[serde(with = "time::serde::rfc3339")]
    pub received_at: OffsetDateTime,
    /// Where it came from.
    #[serde(default)]
    pub channel: Option<Channel>,
    /// HTTP method.
    pub method: String,
    /// Route template.
    pub route: String,
    /// Status it got.
    pub http_status: u16,
    /// API key used.
    #[serde(default)]
    pub api_key_id: Option<String>,
    /// User, for web app requests.
    #[serde(default)]
    pub user_id: Option<Uuid>,
    /// Caller network range (/24 or /48).
    #[serde(default)]
    pub client_ip_prefix: Option<String>,
    /// Keyed hash of the full caller address.
    #[serde(default)]
    pub client_ip_hmac: Option<String>,
    /// Caller user agent.
    #[serde(default)]
    pub user_agent: Option<String>,
    /// Result code, for example `FOUND` or `NO_CREDIT`.
    #[serde(default)]
    pub outcome_code: Option<String>,
    /// What the request was about.
    #[serde(default)]
    pub fingerprint: Option<Fingerprint>,
    /// Fields removed from this record.
    #[serde(default)]
    pub redactions: Option<Vec<Redaction>>,
}

/// Filters for [`Audit::requests`](crate::resources::Audit::requests).
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ApiRequestsParams {
    /// Start of the range. The API defaults to the last 24 hours.
    pub from: Option<OffsetDateTime>,
    /// End of the range. At most 31 days after `from`.
    pub to: Option<OffsetDateTime>,
    /// Only this status.
    pub http_status: Option<u16>,
    /// Only this result code.
    pub outcome_code: Option<String>,
    /// Only this route.
    pub route: Option<String>,
    /// Only this channel.
    pub channel: Option<Channel>,
    /// Only this API key.
    pub api_key_id: Option<String>,
    /// Only requests about this transaction reference.
    pub trans_ref: Option<String>,
    /// Only requests with this QR hash.
    pub qr_sha256: Option<String>,
    /// Page size, 1 to 100.
    pub limit: Option<u32>,
    /// Cursor from a previous page.
    pub cursor: Option<String>,
}

impl ApiRequestsParams {
    pub(crate) fn to_query(&self) -> Query {
        let mut q = Vec::new();
        push_time(&mut q, "from", self.from);
        push_time(&mut q, "to", self.to);
        push_opt(&mut q, "http_status", &self.http_status);
        push_opt(&mut q, "outcome_code", &self.outcome_code);
        push_opt(&mut q, "route", &self.route);
        push_opt(&mut q, "channel", &self.channel);
        push_opt(&mut q, "api_key_id", &self.api_key_id);
        push_opt(&mut q, "trans_ref", &self.trans_ref);
        push_opt(&mut q, "qr_sha256", &self.qr_sha256);
        push_opt(&mut q, "limit", &self.limit);
        push_opt(&mut q, "cursor", &self.cursor);
        q
    }
}

string_enum! {
    /// A record stream in an evidence export.
    pub enum AuditStream {
        /// API requests.
        ApiRequests = "API_REQUESTS",
        /// Audit events.
        AuditEvents = "AUDIT_EVENTS",
    }
}

string_enum! {
    /// State of an evidence export.
    pub enum EvidenceExportStatus {
        /// Waiting to start.
        Queued = "QUEUED",
        /// Being built.
        Running = "RUNNING",
        /// Ready to download.
        Ready = "READY",
        /// Failed.
        Failed = "FAILED",
        /// The file was deleted.
        Expired = "EXPIRED",
    }
}

/// Body for [`AuditExports::create`](crate::resources::AuditExports::create).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct CreateEvidenceExport {
    /// First day.
    #[serde(with = "date_format")]
    pub from_day: Date,
    /// Last day.
    #[serde(with = "date_format")]
    pub to_day: Date,
    /// Streams to include. The API includes both when left empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub streams: Vec<AuditStream>,
    /// Proof of a recent step-up sign-in, when the API asks for one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_up_proof: Option<String>,
}

impl CreateEvidenceExport {
    /// Export days `from_day` to `to_day`, inclusive.
    pub fn new(from_day: Date, to_day: Date) -> Self {
        CreateEvidenceExport {
            from_day,
            to_day,
            streams: Vec::new(),
            step_up_proof: None,
        }
    }
}

/// Who asked for an export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExportCreator {
    /// Kind of actor.
    #[serde(rename = "type")]
    pub actor_type: ActorType,
    /// Actor ID.
    #[serde(default)]
    pub id: Option<Uuid>,
}

/// Summary of an evidence export file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExportManifest {
    /// File format name.
    pub format: String,
    /// Hash algorithm.
    pub hash_algorithm: String,
    /// JSON canonicalization used before hashing.
    pub canonicalization: String,
    /// SHA-256 of the file, hex.
    pub sha256: String,
    /// File size in bytes.
    pub size_bytes: u64,
}

/// An audit evidence export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EvidenceExport {
    /// Export ID.
    pub export_id: Uuid,
    /// Current state.
    pub status: EvidenceExportStatus,
    /// First day.
    #[serde(with = "date_format")]
    pub from_day: Date,
    /// Last day.
    #[serde(with = "date_format")]
    pub to_day: Date,
    /// Streams included.
    pub streams: Vec<AuditStream>,
    /// When it was requested.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// Who asked for it.
    #[serde(default)]
    pub created_by: Option<ExportCreator>,
    /// When it became ready.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub ready_at: Option<OffsetDateTime>,
    /// When the file will be deleted.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub expires_at: Option<OffsetDateTime>,
    /// File summary, once ready.
    #[serde(default)]
    pub manifest: Option<ExportManifest>,
}
