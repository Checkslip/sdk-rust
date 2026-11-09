//! A blocking client for code without an async runtime (feature `blocking`).
//!
//! It runs the async [`Client`](crate::Client) on a small private Tokio
//! runtime. Do not call it from inside an async context; Tokio will panic.
//! In async code use [`crate::Client`] directly.
//!
//! ```no_run
//! use checkslip::blocking::Client;
//! use checkslip::LookupRequest;
//!
//! let client = Client::new("csk_live_...")?;
//! let check = client.lookup(LookupRequest::qr("00330006000001010306902126302105340875102TH9104F86F"))?;
//! println!("{}", check.outcome);
//!
//! // Anything else: run the async call to completion.
//! let balance = client.block_on(client.inner().billing().balance())?;
//! println!("{} credits left", balance.credits);
//! # Ok::<(), checkslip::Error>(())
//! ```

use std::future::Future;
use std::sync::Arc;

use uuid::Uuid;

use crate::error::{Error, Result};
use crate::models::{
    Balance, ListSlipsParams, Page, SlipCheck, SlipCheckRecord, SupportedBank, Usage, UsageParams,
};
use crate::{ClientBuilder, LookupRequest};

/// Blocking wrapper around [`crate::Client`].
#[derive(Debug, Clone)]
pub struct Client {
    inner: crate::Client,
    rt: Arc<tokio::runtime::Runtime>,
}

impl Client {
    /// A blocking client for the production API.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::from_builder(crate::Client::builder().api_key(api_key))
    }

    /// A blocking client from a configured [`ClientBuilder`].
    pub fn from_builder(builder: ClientBuilder) -> Result<Self> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::InvalidArgument(format!("could not start a runtime: {e}")))?;
        // reqwest wants a runtime context while it builds its connector.
        let inner = {
            let _guard = rt.enter();
            builder.build()?
        };
        Ok(Client {
            inner,
            rt: Arc::new(rt),
        })
    }

    /// The async client inside, for calls this wrapper has no method for.
    pub fn inner(&self) -> &crate::Client {
        &self.inner
    }

    /// Runs any future from the async client to completion.
    pub fn block_on<F: Future>(&self, fut: F) -> F::Output {
        self.rt.block_on(fut)
    }

    /// See [`Slips::lookup`](crate::resources::Slips::lookup).
    pub fn lookup(&self, req: LookupRequest) -> Result<SlipCheck> {
        self.block_on(self.inner.slips().lookup(req))
    }

    /// See [`Slips::get`](crate::resources::Slips::get).
    pub fn get_slip(&self, request_id: Uuid) -> Result<SlipCheckRecord> {
        self.block_on(self.inner.slips().get(request_id))
    }

    /// See [`Slips::list`](crate::resources::Slips::list).
    pub fn list_slips(&self, params: &ListSlipsParams) -> Result<Page<SlipCheckRecord>> {
        self.block_on(self.inner.slips().list(params))
    }

    /// See [`Billing::balance`](crate::resources::Billing::balance).
    pub fn balance(&self) -> Result<Balance> {
        self.block_on(self.inner.billing().balance())
    }

    /// See [`crate::Client::usage`].
    pub fn usage(&self, params: &UsageParams) -> Result<Usage> {
        self.block_on(self.inner.usage(params))
    }

    /// See [`crate::Client::banks`].
    pub fn banks(&self) -> Result<Vec<SupportedBank>> {
        self.block_on(self.inner.banks())
    }
}
