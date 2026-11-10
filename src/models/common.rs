use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

time::serde::format_description!(pub(crate) date_format, Date, "[year]-[month]-[day]");

string_enum! {
    /// Currency of a [`Money`] amount. Always `THB` today.
    pub enum Currency {
        /// Thai baht.
        Thb = "THB",
    }
}

/// An amount of money in satang (1 baht = 100 satang).
///
/// The API never uses floating point for money, so neither does this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Money {
    /// The amount in satang. `5000` is 50.00 baht.
    pub satang: i64,
    /// The currency.
    pub currency: Currency,
}

impl Money {
    /// Builds an amount in baht satang.
    pub fn thb(satang: i64) -> Self {
        Money {
            satang,
            currency: Currency::Thb,
        }
    }
}

/// A bank as it appears on a transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Bank {
    /// Short code, for example `KBANK` or `SCB`.
    pub code: String,
    /// Three-digit Bank of Thailand code, for example `004`.
    pub bot_code: String,
    /// Thai name, when known.
    pub name_th: Option<String>,
    /// English name, when known.
    pub name_en: Option<String>,
}

/// The sending side of a transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Party {
    /// The sender's bank.
    pub bank: Bank,
    /// Name in Thai as the bank returned it, usually shortened.
    pub name_th: Option<String>,
    /// Name in English as the bank returned it, usually shortened.
    pub name_en: Option<String>,
    /// Last four digits of the account, when the bank returned them.
    pub account: Option<String>,
}

string_enum! {
    /// What kind of account received the money.
    pub enum ReceiverKind {
        /// A regular bank account.
        BankAccount = "BANK_ACCOUNT",
        /// A bill payment to a company.
        BillPayment = "BILL_PAYMENT",
        /// Something other than a bank account, such as an e-wallet.
        NotBankAccount = "NOT_BANK_ACCOUNT",
    }
}

/// The receiving side of a transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Receiver {
    /// What kind of account it is.
    pub kind: ReceiverKind,
    /// The receiver's bank. `None` for bill payments.
    pub bank: Option<Bank>,
    /// Name in Thai, when the bank returned it.
    pub name_th: Option<String>,
    /// Name in English, when the bank returned it.
    pub name_en: Option<String>,
    /// Last four digits of the account, when the bank returned them.
    pub account: Option<String>,
}

/// The full transfer record the sending bank returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Slip {
    /// The bank's transaction reference.
    pub trans_ref: String,
    /// When the transfer happened, as the bank recorded it.
    #[serde(with = "time::serde::rfc3339")]
    pub transferred_at: OffsetDateTime,
    /// How much was sent.
    pub amount: Money,
    /// Who sent it.
    pub sender: Party,
    /// Who received it.
    pub receiver: Receiver,
    /// `true` when names were erased after the retention period.
    #[serde(default)]
    pub names_erased: bool,
}

/// The parts of a transfer known without the full bank record, for example
/// on `DUPLICATE` or `NOT_FOUND`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SlipSummary {
    /// The transaction reference read from the QR.
    pub trans_ref: String,
    /// The sending bank.
    pub sender_bank: Bank,
    /// The receiving bank, when known.
    #[serde(default)]
    pub receiver_bank: Option<Bank>,
    /// When the transfer happened, when known.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub transferred_at: Option<OffsetDateTime>,
    /// The amount, when known.
    #[serde(default)]
    pub amount: Option<Money>,
}

/// Transfer data on a lookup result: either the full bank record or a
/// summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum SlipData {
    /// The full record (outcome `FOUND`).
    Full(Slip),
    /// A summary (other outcomes).
    Summary(SlipSummary),
}

impl SlipData {
    /// The transaction reference, present in both shapes.
    pub fn trans_ref(&self) -> &str {
        match self {
            SlipData::Full(s) => &s.trans_ref,
            SlipData::Summary(s) => &s.trans_ref,
        }
    }

    /// The amount, when known.
    pub fn amount(&self) -> Option<&Money> {
        match self {
            SlipData::Full(s) => Some(&s.amount),
            SlipData::Summary(s) => s.amount.as_ref(),
        }
    }

    /// The full record, when this is one.
    pub fn as_full(&self) -> Option<&Slip> {
        match self {
            SlipData::Full(s) => Some(s),
            SlipData::Summary(_) => None,
        }
    }
}

/// Points at the earlier lookup of the same transfer on this account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DuplicateOf {
    /// `request_id` of the earlier lookup.
    pub request_id: Uuid,
    /// When the earlier lookup ran.
    #[serde(with = "time::serde::rfc3339")]
    pub checked_at: OffsetDateTime,
}

string_enum! {
    /// Where a request came from.
    pub enum Channel {
        /// An API key.
        Api = "API",
        /// The Checkslip web app.
        Dashboard = "DASHBOARD",
        /// The Checkslip assistant.
        Assistant = "ASSISTANT",
        /// Checkslip staff.
        Staff = "STAFF",
        /// Checkslip itself.
        System = "SYSTEM",
    }
}

/// One page of a list endpoint.
///
/// Pass `next_cursor` back as `cursor` to get the next page, or use the
/// `list_all` methods, which do that for you.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Page<T> {
    /// The items on this page.
    pub items: Vec<T>,
    /// Cursor for the next page. `None` on the last page.
    #[serde(default)]
    pub next_cursor: Option<String>,
}
