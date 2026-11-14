// qa-customer-text: the doc comments reach docs.rs, including the receipt
// details a customer fills in and what a PUT replaces.
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use super::common::{date_format, Money};
use super::slips::FreeReason;
use crate::query::{push_date, push_opt, push_time, Query};

string_enum! {
    /// How the low-balance threshold was worked out.
    pub enum ThresholdBasis {
        /// Average use over the last 7 days.
        Avg7Days = "AVG_7_DAYS",
        /// 10 percent of the last purchase.
        Fallback10Percent = "FALLBACK_10_PERCENT",
        /// No threshold.
        NoThreshold = "NONE",
    }
}

/// The account's credit balance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Balance {
    /// Account ID.
    pub account_id: Uuid,
    /// Credits left. One `FOUND` lookup uses one credit.
    pub credits: u64,
    /// `true` when `credits` is at or below `threshold`.
    pub low: bool,
    /// The low-balance threshold.
    pub threshold: u64,
    /// How `threshold` was worked out.
    pub threshold_basis: ThresholdBasis,
    /// When the credits expire, if they do.
    #[serde(with = "time::serde::rfc3339::option")]
    pub expires_at: Option<OffsetDateTime>,
    /// When the balance last changed.
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    /// The newest ledger entry, if any.
    #[serde(default)]
    pub last_entry_id: Option<Uuid>,
}

string_enum! {
    /// Who the receipts are made out to.
    pub enum BuyerType {
        /// A person.
        Person = "PERSON",
        /// A company.
        Company = "COMPANY",
    }
}

/// Details printed on receipts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BillingInfo {
    /// Person or company.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buyer_type: Option<BuyerType>,
    /// Name on the receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 13-digit tax ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<String>,
    /// Branch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Email for receipts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

impl BillingInfo {
    /// Returns empty billing details, to fill in with the methods on this type.
    ///
    /// The struct is `#[non_exhaustive]`, so code outside the crate cannot
    /// write it as a struct literal; `new()` and these methods take its place.
    ///
    /// ```
    /// use checkslip::{BillingInfo, BuyerType};
    ///
    /// let info = BillingInfo::new()
    ///     .buyer_type(BuyerType::Company)
    ///     .name("ACME")
    ///     .tax_id("0105555555555")
    ///     .email("billing@example.com");
    /// # let _ = info;
    /// ```
    ///
    /// `PUT /api/billing/billing-info` replaces the whole object, so send
    /// every field you want kept, not only the ones you are changing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Person or company.
    pub fn buyer_type(mut self, buyer_type: BuyerType) -> Self {
        self.buyer_type = Some(buyer_type);
        self
    }

    /// Name on the receipt.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 13-digit tax ID.
    pub fn tax_id(mut self, tax_id: impl Into<String>) -> Self {
        self.tax_id = Some(tax_id.into());
        self
    }

    /// Branch.
    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    /// Address.
    pub fn address(mut self, address: impl Into<String>) -> Self {
        self.address = Some(address.into());
        self
    }

    /// Email for receipts.
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }
}

string_enum! {
    /// Why the credit balance changed.
    pub enum CreditReason {
        /// Credits given for testing.
        TesterGrant = "TESTER_GRANT",
        /// A package purchase.
        Purchase = "PURCHASE",
        /// A `FOUND` lookup.
        SlipCharge = "SLIP_CHARGE",
        /// A lookup that used no credit.
        SlipFree = "SLIP_FREE",
        /// Credits expired.
        Expire = "EXPIRE",
        /// Staff adjustment.
        StaffAdjust = "STAFF_ADJUST",
        /// The account was deleted.
        AccountDeleted = "ACCOUNT_DELETED",
        /// Staff restored the account, and the credits the deletion took came
        /// back, keeping the expiry they already had. Credits already past that
        /// expiry are taken again by the next EXPIRE entry.
        AccountRestored = "ACCOUNT_RESTORED",
        /// Development grant.
        DevGrant = "DEV_GRANT",
    }
}

string_enum! {
    /// What a ledger entry points at.
    pub enum LedgerRefType {
        /// An order.
        Order = "ORDER",
        /// A lookup.
        Check = "CHECK",
        /// A staff action.
        Staff = "STAFF",
        /// The system.
        System = "SYSTEM",
    }
}

/// One change to the credit balance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CreditEntry {
    /// Entry ID.
    pub id: Uuid,
    /// Change in credits; negative when credits were used.
    pub delta: i64,
    /// Balance after this entry.
    pub balance_after: u64,
    /// Credit expiry after this entry.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub expires_at_after: Option<OffsetDateTime>,
    /// Why the balance changed.
    pub reason: CreditReason,
    /// Why a lookup was free, for `SLIP_FREE`.
    #[serde(default)]
    pub free_reason: Option<FreeReason>,
    /// What the entry points at.
    #[serde(default)]
    pub ref_type: Option<LedgerRefType>,
    /// ID of what the entry points at.
    #[serde(default)]
    pub ref_id: Option<String>,
    /// When it happened.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Filters for [`Billing::ledger`](crate::resources::Billing::ledger).
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct LedgerParams {
    /// Only this reason.
    pub reason: Option<CreditReason>,
    /// Start of the range.
    pub from: Option<OffsetDateTime>,
    /// End of the range.
    pub to: Option<OffsetDateTime>,
    /// Page size, 1 to 100.
    pub limit: Option<u32>,
    /// Cursor from a previous page.
    pub cursor: Option<String>,
}

impl LedgerParams {
    pub(crate) fn to_query(&self) -> Query {
        let mut q = Vec::new();
        push_opt(&mut q, "reason", &self.reason);
        push_time(&mut q, "from", self.from);
        push_time(&mut q, "to", self.to);
        push_opt(&mut q, "limit", &self.limit);
        push_opt(&mut q, "cursor", &self.cursor);
        q
    }
}

string_enum! {
    /// State of an order.
    pub enum OrderStatus {
        /// Waiting for payment.
        Pending = "PENDING",
        /// Paid; credits added.
        Paid = "PAID",
        /// Cancelled.
        Cancelled = "CANCELLED",
        /// Held for review.
        Flagged = "FLAGGED",
    }
}

string_enum! {
    /// How an order was paid.
    pub enum PaymentMethod {
        /// Card.
        Card = "CARD",
        /// PromptPay.
        PromptPay = "PROMPTPAY",
    }
}

/// A credit package order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Order {
    /// Order ID.
    pub id: Uuid,
    /// Current state.
    pub status: OrderStatus,
    /// Package code.
    pub package_code: String,
    /// Package name in Thai.
    pub package_name_th: String,
    /// Credits the order adds.
    pub credits: u64,
    /// Price.
    pub amount: Money,
    /// The VAT already inside `amount`, fixed when the order was made and not
    /// changed by a later rate change.
    ///
    /// Do not add it to `amount`: `amount` is what the order asks for. It is
    /// `None` when the order carries no VAT.
    #[serde(default)]
    pub vat: Option<Money>,
    /// How it was paid, once paid.
    #[serde(default)]
    pub payment_method: Option<PaymentMethod>,
    /// Receipt, once paid.
    #[serde(default)]
    pub receipt_id: Option<Uuid>,
    /// When it was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// When it was paid.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub paid_at: Option<OffsetDateTime>,
    /// When it was cancelled.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub cancelled_at: Option<OffsetDateTime>,
    /// `true` while the payment provider is confirming.
    #[serde(default)]
    pub confirming: bool,
    /// Why the order is held, for `FLAGGED`.
    #[serde(default)]
    pub review_reason: Option<String>,
}

/// Filters for [`Billing::orders`](crate::resources::Billing::orders).
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct OrdersParams {
    /// Only this status.
    pub status: Option<OrderStatus>,
    /// Page size, 1 to 100.
    pub limit: Option<u32>,
    /// Cursor from a previous page.
    pub cursor: Option<String>,
}

impl OrdersParams {
    pub(crate) fn to_query(&self) -> Query {
        let mut q = Vec::new();
        push_opt(&mut q, "status", &self.status);
        push_opt(&mut q, "limit", &self.limit);
        push_opt(&mut q, "cursor", &self.cursor);
        q
    }
}

/// A credit package you can buy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Package {
    /// Package code.
    pub code: String,
    /// Thai name.
    pub name_th: String,
    /// English name.
    pub name_en: String,
    /// Price.
    pub price: Money,
    /// What a customer pays for this package now, including any VAT due; do
    /// not add VAT again.
    ///
    /// It equals `price` when [`PackageList::vat_rate_bp`] is 0 or
    /// [`PackageList::prices_include_vat`] is true. An order opened now asks
    /// for this amount, so show this one as the price to pay.
    pub price_total: Money,
    /// The price of one credit: `price` divided by `credits`, rounded half up
    /// to a whole satang, so it excludes VAT when
    /// [`PackageList::prices_include_vat`] is false.
    ///
    /// Read the amount to pay from `price_total`; do not multiply this by
    /// `credits` to get it.
    pub price_per_credit: Money,
    /// The price of one credit in baht with 3 decimals, from `price` divided
    /// by `credits`, so it excludes VAT when
    /// [`PackageList::prices_include_vat`] is false.
    ///
    /// Two packages that tie at a whole satang differ here (0.175 against
    /// 0.167). It is display text: show it as sent instead of computing it.
    pub price_per_credit_baht: String,
    /// Credits included.
    pub credits: u64,
    /// Days until the credits expire.
    pub validity_days: u32,
    /// Whether it can be bought now.
    pub for_sale: bool,
    /// Display order.
    pub sort: i32,
}

/// The package list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PackageList {
    /// Packages.
    pub items: Vec<Package>,
    /// Whether prices include VAT.
    pub prices_include_vat: bool,
    /// The VAT rate in force now, in basis points: 700 is 7%, and 0 means no
    /// VAT applies to any package or order yet.
    ///
    /// Use it to show the rate and to tell whether VAT is in effect; read the
    /// amount to pay from each package's [`Package::price_total`] instead of
    /// computing VAT yourself.
    pub vat_rate_bp: u32,
    /// Whether checkout is open.
    pub checkout_open: bool,
}

/// The seller on a receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Seller {
    /// Name.
    pub name: String,
    /// Tax ID.
    #[serde(default)]
    pub tax_id: Option<String>,
    /// Address.
    #[serde(default)]
    pub address: Option<String>,
    /// Branch.
    #[serde(default)]
    pub branch: Option<String>,
}

/// One line on a receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReceiptLine {
    /// What was bought.
    pub description: String,
    /// How many.
    pub quantity: u32,
    /// Line total.
    pub amount: Money,
}

/// VAT on a receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Vat {
    /// Rate in basis points; 700 is 7%.
    pub rate_bp: u32,
    /// Amount before VAT.
    pub base: Money,
    /// VAT amount.
    pub amount: Money,
}

/// A receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Receipt {
    /// Receipt ID.
    pub id: Uuid,
    /// Receipt number, for example `RC-202611-0001`.
    pub receipt_no: String,
    /// The order it is for.
    pub order_id: Uuid,
    /// When it was issued.
    #[serde(with = "time::serde::rfc3339")]
    pub issued_at: OffsetDateTime,
    /// Seller details.
    pub seller: Seller,
    /// Whether the seller details are complete.
    pub seller_ready: bool,
    /// Buyer details.
    pub buyer: BillingInfo,
    /// Lines.
    pub lines: Vec<ReceiptLine>,
    /// VAT, when charged.
    #[serde(default)]
    pub vat: Option<Vat>,
    /// Total.
    pub total: Money,
    /// How it was paid.
    pub payment_method: PaymentMethod,
}

string_enum! {
    /// Bucket size for usage reports.
    pub enum UsageInterval {
        /// One bucket per day.
        Day = "DAY",
        /// One bucket per month.
        Month = "MONTH",
    }
}

/// Parameters for [`Client::usage`](crate::Client::usage).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct UsageParams {
    /// First day, Bangkok time.
    pub from: Date,
    /// Last day, Bangkok time.
    pub to: Date,
    /// Bucket size. The API uses `DAY` when left out.
    pub interval: Option<UsageInterval>,
}

impl UsageParams {
    /// Usage from `from` to `to`, inclusive.
    pub fn new(from: Date, to: Date) -> Self {
        UsageParams {
            from,
            to,
            interval: None,
        }
    }

    /// Sets the bucket size.
    pub fn interval(mut self, interval: UsageInterval) -> Self {
        self.interval = Some(interval);
        self
    }

    pub(crate) fn to_query(&self) -> Query {
        let mut q = Vec::new();
        push_opt(&mut q, "interval", &self.interval);
        push_date(&mut q, "from", Some(self.from));
        push_date(&mut q, "to", Some(self.to));
        q
    }
}

/// Lookup and credit counts for a period.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UsageCounts {
    /// All lookups.
    pub checks: u64,
    /// Lookups that used a credit.
    pub charged: u64,
    /// Free: `NOT_FOUND`.
    pub free_not_found: u64,
    /// Free: `DUPLICATE`.
    pub free_duplicate: u64,
    /// Free: bank unavailable.
    pub free_unavailable: u64,
    /// Free: unreadable QR.
    pub free_invalid_qr: u64,
    /// Free: unreadable file.
    pub free_invalid_file: u64,
    /// Free: QR checksum mismatch.
    pub free_qr_tampered: u64,
    /// Free: transfer too old.
    pub free_too_old: u64,
    /// Free: Checkslip error.
    pub free_system_error: u64,
    /// Free: not charged, no reason given.
    ///
    /// This counts any reason this version of the crate does not know. It
    /// reads as 0 against a server that does not send the field yet.
    ///
    /// The API used to also return `free_no_credit` and `free_hold_expired`.
    /// Both had been returning 0 on every account, and those lookups count
    /// here.
    #[serde(default)]
    pub free_other: u64,
}

/// Counts for one usage bucket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UsageBucket {
    /// First day of the bucket.
    #[serde(with = "date_format")]
    pub start: Date,
    /// The counts.
    #[serde(flatten)]
    pub counts: UsageCounts,
}

/// A usage report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Usage {
    /// Bucket size.
    pub interval: UsageInterval,
    /// When the report was built.
    #[serde(with = "time::serde::rfc3339")]
    pub as_of: OffsetDateTime,
    /// First day.
    #[serde(with = "date_format")]
    pub from: Date,
    /// Last day.
    #[serde(with = "date_format")]
    pub to: Date,
    /// Always `Asia/Bangkok`.
    pub timezone: String,
    /// One entry per bucket.
    pub buckets: Vec<UsageBucket>,
    /// Totals over the whole range.
    pub totals: UsageCounts,
}

/// A bank Checkslip can look up transfers from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SupportedBank {
    /// Short code, for example `KBANK`.
    pub code: String,
    /// Three-digit Bank of Thailand code.
    pub bot_code: String,
    /// Thai name.
    pub name_th: String,
    /// English name.
    pub name_en: String,
    /// When support started.
    #[serde(with = "date_format")]
    pub supported_since: Date,
    /// Whether it is shown on the landing page.
    pub landing_page: bool,
    /// How many days back the bank lets us look up.
    pub lookback_days: u32,
}
