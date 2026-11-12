use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::common::{DuplicateOf, SlipData};
use super::slips::{LookupChannel, Outcome};
use crate::query::{push_opt, push_time, Query};

string_enum! {
    /// A webhook event type.
    pub enum EventType {
        /// A lookup finished, including a `PENDING` lookup that resolved later.
        SlipChecked = "slip.checked",
        /// A test message sent with
        /// [`Endpoints::send_test`](crate::resources::Endpoints::send_test).
        WebhookTest = "webhook.test",
    }
}

string_enum! {
    /// Why a webhook endpoint is off.
    ///
    /// MANUAL means you turned it off, GONE that your system answered 410, and FAILING
    /// that every send failed for 5 days in a row. STAFF means the Checkslip team turned
    /// it off, and you cannot turn it back on yourself, so open a case in the web app.
    /// The API can add a reason later, and an unknown one reaches you as the raw string,
    /// so an exhaustive match needs a catch-all.
    pub enum DisabledReason {
        /// Someone turned it off.
        Manual = "MANUAL",
        /// The endpoint answered 410 Gone.
        Gone = "GONE",
        /// Deliveries kept failing.
        Failing = "FAILING",
        /// The Checkslip team turned it off.
        Staff = "STAFF",
    }
}

/// A webhook signing secret (`whsec_...`).
///
/// `Debug` and `Display` print a redacted value so the secret does not end
/// up in logs. Call [`expose`](SigningSecret::expose) to read it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SigningSecret(String);

impl SigningSecret {
    /// Wraps a secret string.
    pub fn new(secret: impl Into<String>) -> Self {
        SigningSecret(secret.into())
    }

    /// The secret itself. Store it somewhere safe; the API shows it once.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SigningSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SigningSecret(whsec_***)")
    }
}

/// A URL that receives webhook messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Endpoint {
    /// Endpoint ID (`ep_...`).
    pub id: String,
    /// Where messages go.
    pub url: String,
    /// Your note about the endpoint.
    #[serde(default)]
    pub description: Option<String>,
    /// Event types it receives.
    pub event_types: Vec<EventType>,
    /// Whether messages are being sent.
    pub enabled: bool,
    /// Why the endpoint is off, or `None` while it is on.
    #[serde(default)]
    pub disabled_reason: Option<DisabledReason>,
    /// The last few characters of the secret, to tell secrets apart.
    pub secret_hint: String,
    /// While a rotated-out secret still signs messages, when it stops.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub previous_secret_expires_at: Option<OffsetDateTime>,
    /// When it was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// When it last changed.
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// A new endpoint, with its signing secret.
///
/// The API shows the secret only in this response. Store it now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EndpointWithSecret {
    /// The endpoint.
    #[serde(flatten)]
    pub endpoint: Endpoint,
    /// The signing secret.
    pub secret: SigningSecret,
}

/// Body for [`Endpoints::create`](crate::resources::Endpoints::create).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct CreateEndpoint {
    /// An `https` URL whose domain resolves only to public addresses.
    pub url: String,
    /// Event types to receive. At least one.
    pub event_types: Vec<EventType>,
    /// A note for yourself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl CreateEndpoint {
    /// An endpoint at `url` receiving `event_types`.
    pub fn new(url: impl Into<String>, event_types: impl IntoIterator<Item = EventType>) -> Self {
        CreateEndpoint {
            url: url.into(),
            event_types: event_types.into_iter().collect(),
            description: None,
        }
    }

    /// Adds a note.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Body for [`Endpoints::update`](crate::resources::Endpoints::update).
///
/// Fields left as `None` stay as they are (JSON merge patch). Set at least
/// one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct UpdateEndpoint {
    /// New URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// New event types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_types: Option<Vec<EventType>>,
    /// New note. `Some(None)` clears it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    /// Turn sending on or off.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

impl UpdateEndpoint {
    /// An empty patch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Changes the URL.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Changes the event types.
    pub fn event_types(mut self, types: impl IntoIterator<Item = EventType>) -> Self {
        self.event_types = Some(types.into_iter().collect());
        self
    }

    /// Changes the note, or clears it with `None`.
    pub fn description(mut self, description: Option<String>) -> Self {
        self.description = Some(description);
        self
    }

    /// Turns sending on or off.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }
}

/// Body for [`Endpoints::recover`](crate::resources::Endpoints::recover).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct RecoverRequest {
    /// Resend failed messages from this time.
    #[serde(with = "time::serde::rfc3339")]
    pub since: OffsetDateTime,
    /// Up to this time. The API uses now when left out.
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub until: Option<OffsetDateTime>,
}

impl RecoverRequest {
    /// Recover everything since `since`.
    pub fn since(since: OffsetDateTime) -> Self {
        RecoverRequest { since, until: None }
    }

    /// Stops at `until`.
    pub fn until(mut self, until: OffsetDateTime) -> Self {
        self.until = Some(until);
        self
    }
}

/// What a recover call queued.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RecoverResult {
    /// The endpoint.
    pub endpoint_id: String,
    /// Start of the window.
    #[serde(with = "time::serde::rfc3339")]
    pub since: OffsetDateTime,
    /// End of the window.
    #[serde(with = "time::serde::rfc3339")]
    pub until: OffsetDateTime,
}

/// A new signing secret after rotation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SecretRotation {
    /// The new secret. Shown only once.
    pub secret: SigningSecret,
    /// The old secret keeps signing until this time.
    #[serde(with = "time::serde::rfc3339")]
    pub previous_secret_expires_at: OffsetDateTime,
}

/// An event type and whether you can subscribe to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EventTypeInfo {
    /// The event type.
    #[serde(rename = "type")]
    pub event_type: EventType,
    /// What it means.
    pub description: String,
    /// Whether an endpoint can subscribe to it.
    pub subscribable: bool,
}

string_enum! {
    /// State of a delivery.
    pub enum DeliveryStatus {
        /// Not sent yet.
        Pending = "PENDING",
        /// Failed at least once; more attempts are scheduled.
        Retrying = "RETRYING",
        /// Your endpoint answered 2xx.
        Succeeded = "SUCCEEDED",
        /// All attempts failed.
        Failed = "FAILED",
        /// Will not be sent.
        Cancelled = "CANCELLED",
    }
}

/// One message to one endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Delivery {
    /// Delivery ID (`dlv_...`).
    pub id: String,
    /// Message ID (`msg_...`), sent as the `webhook-id` header.
    pub message_id: String,
    /// The endpoint.
    pub endpoint_id: String,
    /// Event type.
    pub event_type: EventType,
    /// Current state.
    pub status: DeliveryStatus,
    /// Attempts so far.
    pub attempt_count: u32,
    /// Next scheduled attempt.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub next_attempt_at: Option<OffsetDateTime>,
    /// Last attempt.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_attempt_at: Option<OffsetDateTime>,
    /// HTTP status of the last attempt.
    #[serde(default)]
    pub last_http_status: Option<u16>,
    /// When it was queued.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

string_enum! {
    /// What started a delivery attempt.
    pub enum AttemptTrigger {
        /// The normal retry schedule.
        Schedule = "SCHEDULE",
        /// A redeliver call.
        Redeliver = "REDELIVER",
        /// A recover call.
        Recover = "RECOVER",
    }
}

string_enum! {
    /// How a delivery attempt ended.
    pub enum AttemptOutcome {
        /// Your endpoint answered 2xx.
        Success = "SUCCESS",
        /// Your endpoint answered an error status.
        HttpError = "HTTP_ERROR",
        /// No answer in time.
        Timeout = "TIMEOUT",
        /// Could not connect.
        ConnectError = "CONNECT_ERROR",
        /// TLS failed.
        TlsError = "TLS_ERROR",
        /// Your endpoint redirected; redirects are not followed.
        RedirectNotFollowed = "REDIRECT_NOT_FOLLOWED",
        /// The address is not public.
        DestinationBlocked = "DESTINATION_BLOCKED",
    }
}

/// One attempt to send a delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DeliveryAttempt {
    /// Attempt number, from 1.
    pub attempt_no: u32,
    /// What started it.
    #[serde(default)]
    pub trigger: Option<AttemptTrigger>,
    /// When it ran.
    #[serde(with = "time::serde::rfc3339")]
    pub attempted_at: OffsetDateTime,
    /// How it ended.
    pub outcome: AttemptOutcome,
    /// Status your endpoint answered.
    #[serde(default)]
    pub http_status: Option<u16>,
    /// How long it took.
    pub duration_ms: u64,
    /// Start of your endpoint's response body.
    #[serde(default)]
    pub response_excerpt: Option<String>,
}

/// A delivery with its payload and attempts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DeliveryDetail {
    /// The delivery.
    #[serde(flatten)]
    pub delivery: Delivery,
    /// The message, until it expires.
    #[serde(default)]
    pub payload: Option<WebhookEvent>,
    /// When the payload will be deleted.
    #[serde(with = "time::serde::rfc3339")]
    pub payload_expires_at: OffsetDateTime,
    /// Attempts so far.
    #[serde(default)]
    pub attempts: Vec<DeliveryAttempt>,
}

/// Filters for [`Deliveries::list`](crate::resources::Deliveries::list).
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ListDeliveriesParams {
    /// Only this endpoint.
    pub endpoint_id: Option<String>,
    /// Only this status.
    pub status: Option<DeliveryStatus>,
    /// Only this event type.
    pub event_type: Option<EventType>,
    /// Queued at or after.
    pub since: Option<OffsetDateTime>,
    /// Queued before.
    pub until: Option<OffsetDateTime>,
    /// Page size, 1 to 100.
    pub limit: Option<u32>,
    /// Cursor from a previous page.
    pub cursor: Option<String>,
}

impl ListDeliveriesParams {
    /// No filters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Only deliveries to this endpoint.
    pub fn endpoint_id(mut self, id: impl Into<String>) -> Self {
        self.endpoint_id = Some(id.into());
        self
    }

    /// Only deliveries in this state.
    pub fn status(mut self, status: DeliveryStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub(crate) fn to_query(&self) -> Query {
        let mut q = Vec::new();
        push_opt(&mut q, "endpoint_id", &self.endpoint_id);
        push_opt(&mut q, "status", &self.status);
        push_opt(&mut q, "event_type", &self.event_type);
        push_time(&mut q, "since", self.since);
        push_time(&mut q, "until", self.until);
        push_opt(&mut q, "limit", &self.limit);
        push_opt(&mut q, "cursor", &self.cursor);
        q
    }
}

/// The `data` of a `slip.checked` message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SlipCheckedData {
    /// The lookup's ID, the same as in the lookup response.
    pub request_id: Uuid,
    /// What the bank said.
    pub outcome: Outcome,
    /// When Checkslip asked the bank.
    #[serde(with = "time::serde::rfc3339")]
    pub checked_at: OffsetDateTime,
    /// Where the lookup came from.
    pub channel: LookupChannel,
    /// Your own reference from the lookup request.
    #[serde(default)]
    pub client_reference: Option<String>,
    /// Transfer data.
    #[serde(default)]
    pub slip: Option<SlipData>,
    /// The check that first found this transfer, for duplicates.
    #[serde(default)]
    pub duplicate_of: Option<DuplicateOf>,
    /// `true` when this lookup was `PENDING` first and has now resolved.
    #[serde(default)]
    pub resolved_after_pending: bool,
}

/// The `data` of a `webhook.test` message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TestData {
    /// The endpoint the test was sent to.
    pub endpoint_id: String,
}

/// A webhook message body.
///
/// Use [`webhook::Webhook::verify`](crate::webhook::Webhook::verify) (feature
/// `webhooks`) to check the signature and parse the body in one step.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum WebhookEvent {
    /// `slip.checked`
    #[serde(rename = "slip.checked")]
    SlipChecked {
        /// When the event happened.
        #[serde(with = "time::serde::rfc3339")]
        timestamp: OffsetDateTime,
        /// The lookup result.
        data: Box<SlipCheckedData>,
    },
    /// `webhook.test`
    #[serde(rename = "webhook.test")]
    Test {
        /// When the event happened.
        #[serde(with = "time::serde::rfc3339")]
        timestamp: OffsetDateTime,
        /// Which endpoint it was sent to.
        data: TestData,
    },
    /// An event type added after this version of the crate. The whole
    /// message is kept as JSON.
    #[serde(untagged)]
    Unknown(serde_json::Value),
}

impl WebhookEvent {
    /// The event type.
    pub fn event_type(&self) -> EventType {
        match self {
            WebhookEvent::SlipChecked { .. } => EventType::SlipChecked,
            WebhookEvent::Test { .. } => EventType::WebhookTest,
            WebhookEvent::Unknown(v) => {
                EventType::from(v.get("type").and_then(|t| t.as_str()).unwrap_or(""))
            }
        }
    }
}

impl<'de> Deserialize<'de> for WebhookEvent {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(rename = "type")]
            event_type: EventType,
            #[serde(with = "time::serde::rfc3339")]
            timestamp: OffsetDateTime,
            data: serde_json::Value,
        }

        let value = serde_json::Value::deserialize(d)?;
        let raw = Raw::deserialize(&value).map_err(serde::de::Error::custom)?;
        match raw.event_type {
            EventType::SlipChecked => Ok(WebhookEvent::SlipChecked {
                timestamp: raw.timestamp,
                data: Box::new(
                    SlipCheckedData::deserialize(&raw.data).map_err(serde::de::Error::custom)?,
                ),
            }),
            EventType::WebhookTest => Ok(WebhookEvent::Test {
                timestamp: raw.timestamp,
                data: TestData::deserialize(&raw.data).map_err(serde::de::Error::custom)?,
            }),
            EventType::Unknown(_) => Ok(WebhookEvent::Unknown(value)),
        }
    }
}
