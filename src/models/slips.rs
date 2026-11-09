use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::common::{DuplicateOf, SlipData};
use crate::query::{push_list, push_opt, push_time};

string_enum! {
    /// The result of looking up a transfer with the sending bank.
    ///
    /// A lookup returns `FOUND`, `NOT_FOUND`, `DUPLICATE` or `PENDING`. The
    /// other values appear in history records for requests the API answered
    /// with an error, such as an unreadable QR.
    pub enum Outcome {
        /// The bank returned the transfer. This is the only outcome that
        /// uses a credit.
        Found = "FOUND",
        /// The bank has no transfer with this reference.
        NotFound = "NOT_FOUND",
        /// This account already looked up this transfer; see `duplicate_of`.
        Duplicate = "DUPLICATE",
        /// The bank has not posted the transfer yet. Checkslip keeps asking
        /// and sends a `slip.checked` webhook when it knows.
        Pending = "PENDING",
        /// The bank could not be reached.
        Unavailable = "UNAVAILABLE",
        /// The QR text is not a bank slip QR.
        InvalidQr = "INVALID_QR",
        /// The image could not be read.
        InvalidFile = "INVALID_FILE",
        /// The QR checksum does not match its contents.
        QrTampered = "QR_TAMPERED",
        /// The transfer is older than the bank lets us look up.
        TooOld = "TOO_OLD",
    }
}

string_enum! {
    /// Why a lookup used no credit.
    pub enum FreeReason {
        /// Outcome was `NOT_FOUND`.
        NotFound = "NOT_FOUND",
        /// Outcome was `DUPLICATE`.
        Duplicate = "DUPLICATE",
        /// Outcome was `PENDING`.
        Pending = "PENDING",
        /// The bank could not be reached.
        Unavailable = "UNAVAILABLE",
        /// The QR could not be read.
        InvalidQr = "INVALID_QR",
        /// The image could not be read.
        InvalidFile = "INVALID_FILE",
        /// The QR checksum did not match.
        QrTampered = "QR_TAMPERED",
        /// The transfer was too old.
        TooOld = "TOO_OLD",
        /// The account had no credit.
        NoCredit = "NO_CREDIT",
        /// The credit hold ran out before the bank answered.
        HoldExpired = "HOLD_EXPIRED",
        /// Checkslip failed on its side.
        SystemError = "SYSTEM_ERROR",
    }
}

/// The answer to a lookup.
///
/// Checkslip returns what the sending bank has. Whether to accept the
/// payment is your decision: compare `slip` against your own order (amount,
/// receiver, time).
/// latency_ms, upstream_ms and checked_elsewhere are deliberately absent:
/// they are being withdrawn from the Checkslip API, so your code will not
/// break when it stops sending them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SlipCheck {
    /// ID of this lookup. Use it with [`Slips::get`](crate::resources::Slips::get).
    pub request_id: Uuid,
    /// What the bank said.
    pub outcome: Outcome,
    /// When Checkslip asked the bank.
    #[serde(with = "time::serde::rfc3339")]
    pub checked_at: OffsetDateTime,
    /// The transfer data.
    pub slip: SlipData,
    /// The earlier lookup, when `outcome` is `DUPLICATE`.
    #[serde(default)]
    pub duplicate_of: Option<DuplicateOf>,
    /// Your own reference, echoed back.
    #[serde(default)]
    pub client_reference: Option<String>,
}

string_enum! {
    /// Where a lookup was made from.
    pub enum LookupChannel {
        /// An API key.
        Api = "API",
        /// The web dashboard.
        Dashboard = "DASHBOARD",
        /// The Checkslip assistant.
        Assistant = "ASSISTANT",
    }
}

/// One lookup in the account's history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SlipCheckRecord {
    /// ID of the lookup.
    pub request_id: Uuid,
    /// What the bank said, or why the request failed.
    pub outcome: Outcome,
    /// When it ran.
    #[serde(with = "time::serde::rfc3339")]
    pub checked_at: OffsetDateTime,
    /// Where it came from.
    pub channel: LookupChannel,
    /// The API key used, when it came through one.
    #[serde(default)]
    pub api_key_id: Option<String>,
    /// HTTP status the request got.
    #[serde(default)]
    pub status: Option<u16>,
    /// Transfer data, when there is any.
    #[serde(default)]
    pub slip: Option<SlipData>,
    /// The earlier lookup, for duplicates.
    #[serde(default)]
    pub duplicate_of: Option<DuplicateOf>,
    /// Your own reference.
    #[serde(default)]
    pub client_reference: Option<String>,
    /// Whether billing has confirmed the charge. `None` until settled.
    #[serde(default)]
    pub charge_confirmed: Option<bool>,
    /// Whether a credit was used. `None` until settled.
    #[serde(default)]
    pub charged: Option<bool>,
    /// Why no credit was used.
    #[serde(default)]
    pub free_reason: Option<FreeReason>,
}

string_enum! {
    /// Filter value for whether a lookup used a credit.
    pub enum ChargedFilter {
        /// Used a credit.
        True = "true",
        /// Used no credit.
        False = "false",
        /// Billing has not settled yet.
        Unsettled = "unsettled",
    }
}

/// Filters for [`Slips::list`](crate::resources::Slips::list).
///
/// Every field is optional. The API's defaults apply to anything left out.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ListSlipsParams {
    /// Free-text search.
    pub q: Option<String>,
    /// Exact transaction reference.
    pub trans_ref: Option<String>,
    /// Only these outcomes.
    pub outcome: Vec<Outcome>,
    /// Leave out these outcomes.
    pub outcome_not: Vec<Outcome>,
    /// Sending bank, as a three-digit Bank of Thailand code.
    pub sender_bank: Option<String>,
    /// API key ID (12 characters).
    pub api_key: Option<String>,
    /// Only lookups newer than this request ID.
    pub after: Option<Uuid>,
    /// Filter by charge state.
    pub charged: Vec<ChargedFilter>,
    /// Only these free reasons.
    pub free_reason: Vec<FreeReason>,
    /// Start of the time range.
    pub from: Option<OffsetDateTime>,
    /// End of the time range.
    pub to: Option<OffsetDateTime>,
    /// Page size, 1 to 100.
    pub limit: Option<u32>,
    /// Cursor from a previous page.
    pub cursor: Option<String>,
}

impl ListSlipsParams {
    /// No filters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the page size (1 to 100).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Only lookups with these outcomes.
    pub fn outcome(mut self, outcomes: impl IntoIterator<Item = Outcome>) -> Self {
        self.outcome = outcomes.into_iter().collect();
        self
    }

    /// Only lookups of this transaction reference.
    pub fn trans_ref(mut self, trans_ref: impl Into<String>) -> Self {
        self.trans_ref = Some(trans_ref.into());
        self
    }

    /// Only lookups in this time range.
    pub fn range(mut self, from: OffsetDateTime, to: OffsetDateTime) -> Self {
        self.from = Some(from);
        self.to = Some(to);
        self
    }

    pub(crate) fn to_query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "q", &self.q);
        push_opt(&mut q, "trans_ref", &self.trans_ref);
        push_list(&mut q, "outcome", &self.outcome);
        push_list(&mut q, "outcome_not", &self.outcome_not);
        push_opt(&mut q, "sender_bank", &self.sender_bank);
        push_opt(&mut q, "api_key", &self.api_key);
        push_opt(&mut q, "after", &self.after);
        push_list(&mut q, "charged", &self.charged);
        push_list(&mut q, "free_reason", &self.free_reason);
        push_time(&mut q, "from", self.from);
        push_time(&mut q, "to", self.to);
        push_opt(&mut q, "limit", &self.limit);
        push_opt(&mut q, "cursor", &self.cursor);
        q
    }
}

string_enum! {
    /// Bucket size for [`Slips::counts`](crate::resources::Slips::counts).
    pub enum CountBucket {
        /// One minute.
        Minute = "MINUTE",
        /// Five minutes.
        FiveMinute = "FIVE_MINUTE",
        /// Fifteen minutes.
        FifteenMinute = "FIFTEEN_MINUTE",
        /// One hour.
        Hour = "HOUR",
        /// Three hours.
        ThreeHour = "THREE_HOUR",
        /// Six hours.
        SixHour = "SIX_HOUR",
        /// Twelve hours.
        TwelveHour = "TWELVE_HOUR",
        /// One day.
        Day = "DAY",
    }
}

/// Parameters for [`Slips::counts`](crate::resources::Slips::counts).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CountSlipsParams {
    /// Start of the range.
    pub from: OffsetDateTime,
    /// End of the range.
    pub to: OffsetDateTime,
    /// Bucket size.
    pub bucket: CountBucket,
    /// Only these outcomes.
    pub outcome: Vec<Outcome>,
    /// Leave out these outcomes.
    pub outcome_not: Vec<Outcome>,
}

impl CountSlipsParams {
    /// Counts between `from` and `to` in buckets of `bucket`.
    pub fn new(from: OffsetDateTime, to: OffsetDateTime, bucket: CountBucket) -> Self {
        CountSlipsParams {
            from,
            to,
            bucket,
            outcome: Vec::new(),
            outcome_not: Vec::new(),
        }
    }

    pub(crate) fn to_query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_time(&mut q, "from", Some(self.from));
        push_time(&mut q, "to", Some(self.to));
        q.push(("bucket", self.bucket.to_string()));
        push_list(&mut q, "outcome", &self.outcome);
        push_list(&mut q, "outcome_not", &self.outcome_not);
        q
    }
}

/// Lookup counts over time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SlipCounts {
    /// Bucket size used.
    pub bucket: CountBucket,
    /// When the counts were taken.
    #[serde(with = "time::serde::rfc3339")]
    pub as_of: OffsetDateTime,
    /// Filters the server applied.
    #[serde(default)]
    pub filters_applied: Vec<String>,
    /// Filters the server ignored for counts.
    #[serde(default)]
    pub filters_ignored: Vec<String>,
    /// One entry per bucket.
    pub series: Vec<CountPoint>,
}

/// Counts for one bucket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CountPoint {
    /// Start of the bucket.
    #[serde(with = "time::serde::rfc3339")]
    pub start: OffsetDateTime,
    /// Count per outcome, keyed by the outcome string.
    pub counts: BTreeMap<String, u64>,
}

string_enum! {
    /// A column in a slip CSV export.
    pub enum ExportColumn {
        /// `request_id`
        RequestId = "request_id",
        /// `outcome`
        Outcome = "outcome",
        /// `status`
        Status = "status",
        /// `checked_at`
        CheckedAt = "checked_at",
        /// `channel`
        Channel = "channel",
        /// `api_key_id`
        ApiKeyId = "api_key_id",
        /// `client_reference`
        ClientReference = "client_reference",
        /// `duplicate_of.request_id`
        DuplicateOfRequestId = "duplicate_of.request_id",
        /// `charge_confirmed`
        ChargeConfirmed = "charge_confirmed",
        /// `charged`
        Charged = "charged",
        /// `free_reason`
        FreeReason = "free_reason",
        /// `slip.trans_ref`
        TransRef = "slip.trans_ref",
        /// `slip.transferred_at`
        TransferredAt = "slip.transferred_at",
        /// `slip.amount.satang`
        AmountSatang = "slip.amount.satang",
        /// `slip.amount.currency`
        AmountCurrency = "slip.amount.currency",
        /// `slip.sender.bank.code`
        SenderBankCode = "slip.sender.bank.code",
        /// `slip.sender.bank.bot_code`
        SenderBankBotCode = "slip.sender.bank.bot_code",
        /// `slip.sender.name_th`
        SenderNameTh = "slip.sender.name_th",
        /// `slip.sender.name_en`
        SenderNameEn = "slip.sender.name_en",
        /// `slip.sender.account`
        SenderAccount = "slip.sender.account",
        /// `slip.receiver.bank.code`
        ReceiverBankCode = "slip.receiver.bank.code",
        /// `slip.receiver.bank.bot_code`
        ReceiverBankBotCode = "slip.receiver.bank.bot_code",
        /// `slip.receiver.name_th`
        ReceiverNameTh = "slip.receiver.name_th",
        /// `slip.receiver.name_en`
        ReceiverNameEn = "slip.receiver.name_en",
        /// `slip.receiver.account`
        ReceiverAccount = "slip.receiver.account",
    }
}

/// What to put in a slip CSV export.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SlipExportRequest {
    /// Columns, in order. At least one.
    pub columns: Vec<ExportColumn>,
    /// Free-text search.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    /// Exact transaction reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trans_ref: Option<String>,
    /// Only these outcomes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outcome: Vec<Outcome>,
    /// Leave out these outcomes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outcome_not: Vec<Outcome>,
    /// Sending bank, three-digit Bank of Thailand code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_bank: Option<String>,
    /// API key ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Filter by charge state.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub charged: Vec<ChargedFilter>,
    /// Only these free reasons.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub free_reason: Vec<FreeReason>,
    /// Start of the range.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub from: Option<OffsetDateTime>,
    /// End of the range.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub to: Option<OffsetDateTime>,
    /// `continue_cursor` from a truncated export, to carry on from there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl SlipExportRequest {
    /// An export with these columns and no filters.
    pub fn new(columns: impl IntoIterator<Item = ExportColumn>) -> Self {
        SlipExportRequest {
            columns: columns.into_iter().collect(),
            ..Default::default()
        }
    }
}

string_enum! {
    /// State of an export job.
    pub enum ExportState {
        /// Waiting to start.
        Queued = "QUEUED",
        /// Being built.
        Running = "RUNNING",
        /// Ready to download.
        Ready = "READY",
        /// Failed.
        Failed = "FAILED",
        /// The files were deleted.
        Expired = "EXPIRED",
    }
}

string_enum! {
    /// How the server built an export.
    pub enum ExportMode {
        /// Built during the request.
        Direct = "DIRECT",
        /// Built by a background job.
        Job = "JOB",
    }
}

/// A slip CSV export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SlipExport {
    /// Export ID.
    pub export_id: Uuid,
    /// Current state.
    pub state: ExportState,
    /// How it was built.
    #[serde(default)]
    pub mode: Option<ExportMode>,
    /// When it was requested.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// The request as the server stored it.
    pub request: SlipExportRequest,
    /// Number of rows, once known.
    #[serde(default)]
    pub rows: Option<u64>,
    /// `true` when the export hit the row limit.
    #[serde(default)]
    pub truncated: bool,
    /// Pass this as `cursor` in a new request to export the rest.
    #[serde(default)]
    pub continue_cursor: Option<String>,
    /// When the files will be deleted.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub expires_at: Option<OffsetDateTime>,
    /// Download links, one per CSV part.
    #[serde(default)]
    pub parts: Vec<ExportPart>,
}

/// One downloadable CSV part of a [`SlipExport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExportPart {
    /// Part number, from 1.
    pub part: u32,
    /// Rows in this part, once known.
    #[serde(default)]
    pub rows: Option<u64>,
    /// Signed download URL.
    pub url: String,
    /// When the URL stops working.
    #[serde(with = "time::serde::rfc3339")]
    pub url_expires_at: OffsetDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_outcome_round_trips() {
        let o: Outcome = serde_json::from_str("\"SOMETHING_NEW\"").unwrap();
        assert_eq!(o, Outcome::Unknown("SOMETHING_NEW".into()));
        assert_eq!(serde_json::to_string(&o).unwrap(), "\"SOMETHING_NEW\"");
        assert_eq!(Outcome::from("FOUND"), Outcome::Found);
    }

    #[test]
    fn list_params_repeat_array_keys() {
        let q = ListSlipsParams::new()
            .outcome([Outcome::Found, Outcome::Duplicate])
            .limit(50)
            .to_query();
        assert_eq!(
            q,
            vec![
                ("outcome", "FOUND".to_string()),
                ("outcome", "DUPLICATE".to_string()),
                ("limit", "50".to_string()),
            ]
        );
    }
}
