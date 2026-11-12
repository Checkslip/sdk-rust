use bytes::Bytes;
use reqwest::Method;
use uuid::Uuid;

use crate::client::{Client, MultipartImage, RequestSpec};
use crate::error::{Error, Result};
use crate::models::{
    CountSlipsParams, ExportPart, ListSlipsParams, Page, SlipCheck, SlipCheckRecord, SlipCounts,
    SlipExport, SlipExportRequest,
};
use crate::pagination::{paginate, ItemStream};

/// What to look up: the QR text from a slip, or the slip image.
///
/// ```
/// use checkslip::LookupRequest;
///
/// // QR text you read yourself
/// let by_qr = LookupRequest::qr("00330006000001010306902126302105340875102TH9104F86F")
///     .client_reference("order-1042");
///
/// // or the image the customer uploaded (JPEG, PNG or WebP, up to 10 MB)
/// # let image_bytes: Vec<u8> = vec![0xFF, 0xD8, 0xFF];
/// let by_image = LookupRequest::image(image_bytes)
///     .idempotency_key("order-1042-slip-1");
/// # let _ = (by_qr, by_image);
/// ```
#[derive(Debug, Clone)]
pub struct LookupRequest {
    input: LookupInput,
    client_reference: Option<String>,
    idempotency_key: Option<String>,
}

#[derive(Debug, Clone)]
enum LookupInput {
    Qr(String),
    Image {
        bytes: Bytes,
        file_name: String,
        content_type: String,
    },
}

impl LookupRequest {
    /// Look up by the text in the slip's QR code.
    pub fn qr(qr: impl Into<String>) -> Self {
        LookupRequest {
            input: LookupInput::Qr(qr.into()),
            client_reference: None,
            idempotency_key: None,
        }
    }

    /// Look up by the slip image. The server reads the QR from it and does
    /// not keep the image.
    ///
    /// The content type is worked out from the first bytes (JPEG, PNG or
    /// WebP). Use [`image_with_type`](Self::image_with_type) to set it
    /// yourself.
    pub fn image(bytes: impl Into<Bytes>) -> Self {
        let bytes = bytes.into();
        let (content_type, ext) = sniff_image(&bytes);
        LookupRequest {
            input: LookupInput::Image {
                bytes,
                file_name: format!("slip.{ext}"),
                content_type: content_type.to_owned(),
            },
            client_reference: None,
            idempotency_key: None,
        }
    }

    /// Look up by the slip image with an explicit content type, such as
    /// `image/png`.
    pub fn image_with_type(bytes: impl Into<Bytes>, content_type: impl Into<String>) -> Self {
        LookupRequest {
            input: LookupInput::Image {
                bytes: bytes.into(),
                file_name: "slip".to_owned(),
                content_type: content_type.into(),
            },
            client_reference: None,
            idempotency_key: None,
        }
    }

    /// Your own reference, such as an order number (up to 128 printable
    /// ASCII characters). It comes back on the result and on webhooks.
    pub fn client_reference(mut self, reference: impl Into<String>) -> Self {
        self.client_reference = Some(reference.into());
        self
    }

    /// The `Idempotency-Key` to send.
    ///
    /// When you leave it out, the client makes a random one per call and
    /// reuses it for that call's retries. Set your own, for example from
    /// your order ID, if you may send the same lookup again after a crash
    /// or restart: the API then returns the first result and does not
    /// charge twice.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

fn sniff_image(b: &[u8]) -> (&'static str, &'static str) {
    if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        ("image/jpeg", "jpg")
    } else if b.starts_with(&[0x89, b'P', b'N', b'G']) {
        ("image/png", "png")
    } else if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        ("image/webp", "webp")
    } else {
        ("application/octet-stream", "bin")
    }
}

/// Slip lookups and lookup history. Get one from [`Client::slips`].
#[derive(Debug, Clone)]
pub struct Slips {
    client: Client,
}

impl Slips {
    /// CSV exports of past lookups.
    pub fn exports(&self) -> SlipExports {
        SlipExports {
            client: self.client.clone(),
        }
    }

    pub(crate) fn new(client: Client) -> Self {
        Slips { client }
    }

    /// Asks the sending bank for the transfer behind a slip.
    ///
    /// Check `outcome` on the result. Only `FOUND` uses a credit. On
    /// `PENDING` the bank has not posted the transfer yet; Checkslip keeps
    /// asking and sends a `slip.checked` webhook, or you can call
    /// [`get`](Self::get) with the `request_id` later.
    ///
    /// Checkslip reports what the bank has. Whether that matches your order
    /// (amount, receiver, time) is for your code to decide.
    ///
    /// Every lookup carries an `Idempotency-Key`, so retries after a network
    /// error or a 5xx cannot charge twice.
    pub async fn lookup(&self, req: LookupRequest) -> Result<SlipCheck> {
        let spec = RequestSpec::new(Method::POST, &["api", "slips", "lookup"])
            .idempotency_key(req.idempotency_key);
        let spec = match req.input {
            LookupInput::Qr(qr) => {
                #[derive(serde::Serialize)]
                struct Body<'a> {
                    qr: &'a str,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    client_reference: Option<&'a str>,
                }
                spec.json(&Body {
                    qr: &qr,
                    client_reference: req.client_reference.as_deref(),
                })?
            }
            LookupInput::Image {
                bytes,
                file_name,
                content_type,
            } => spec.multipart(MultipartImage {
                image: bytes,
                file_name,
                content_type,
                client_reference: req.client_reference,
            }),
        };
        self.client.send_json(spec).await
    }

    /// One lookup from the history, by its `request_id`.
    pub async fn get(&self, request_id: Uuid) -> Result<SlipCheckRecord> {
        self.client
            .get_json(&["api", "slips", &request_id.to_string()], Vec::new())
            .await
    }

    /// One page of lookup history, newest first.
    pub async fn list(&self, params: &ListSlipsParams) -> Result<Page<SlipCheckRecord>> {
        self.client
            .get_json(&["api", "slips"], params.to_query())
            .await
    }

    /// Every lookup matching `params`, fetching pages as you read.
    ///
    /// ```no_run
    /// # async fn run(client: checkslip::Client) -> checkslip::Result<()> {
    /// use futures_util::TryStreamExt;
    /// use checkslip::{ListSlipsParams, Outcome};
    ///
    /// let mut all = client
    ///     .slips()
    ///     .list_all(ListSlipsParams::new().outcome([Outcome::Found]).limit(100));
    /// while let Some(record) = all.try_next().await? {
    ///     println!("{} {}", record.request_id, record.outcome);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_all(&self, params: ListSlipsParams) -> ItemStream<SlipCheckRecord> {
        let client = self.clone();
        let start = params.cursor.clone();
        paginate(start, move |cursor| {
            let client = client.clone();
            let mut p = params.clone();
            p.cursor = cursor;
            async move { client.list(&p).await }
        })
    }

    /// Lookup counts per time bucket.
    pub async fn counts(&self, params: &CountSlipsParams) -> Result<SlipCounts> {
        self.client
            .get_json(&["api", "slips", "counts"], params.to_query())
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::sniff_image;

    #[test]
    fn sniffs_common_formats() {
        assert_eq!(sniff_image(&[0xFF, 0xD8, 0xFF, 0xE0]).0, "image/jpeg");
        assert_eq!(sniff_image(b"\x89PNG\r\n\x1a\n").0, "image/png");
        assert_eq!(sniff_image(b"RIFF\0\0\0\0WEBPVP8 ").0, "image/webp");
        assert_eq!(sniff_image(b"GIF89a").0, "application/octet-stream");
    }
}

/// CSV exports of past lookups.
#[derive(Debug, Clone)]
pub struct SlipExports {
    client: Client,
}

impl SlipExports {
    /// Starts a CSV export of lookup history. Small exports come back
    /// `READY`; larger ones are `QUEUED` and you poll
    /// [`get`](Self::get).
    ///
    /// Not available yet: the API does not accept this request, so calling it
    /// returns an error.
    pub async fn create(&self, req: &SlipExportRequest) -> Result<SlipExport> {
        let spec = RequestSpec::new(Method::POST, &["api", "slips", "exports"]).json(req)?;
        self.client.send_json(spec).await
    }
    /// The state of a CSV export.
    ///
    /// Not available yet: the API does not accept this request, so calling it
    /// returns an error.
    pub async fn get(&self, export_id: Uuid) -> Result<SlipExport> {
        self.client
            .get_json(
                &["api", "slips", "exports", &export_id.to_string()],
                Vec::new(),
            )
            .await
    }
    /// Downloads one CSV part of a `READY` export.
    ///
    /// Not available yet: the API does not accept this request, so calling it
    /// returns an error.
    pub async fn download_part(&self, export_id: Uuid, part: &ExportPart) -> Result<Bytes> {
        let sig = url::Url::parse(&part.url)
            .ok()
            .and_then(|u| {
                u.query_pairs()
                    .find(|(k, _)| k == "sig")
                    .map(|(_, v)| v.into_owned())
            })
            .ok_or_else(|| Error::InvalidArgument("export part URL has no sig".to_owned()))?;
        let spec = RequestSpec::new(
            Method::GET,
            &[
                "api",
                "slips",
                "exports",
                &export_id.to_string(),
                "parts",
                &part.part.to_string(),
            ],
        )
        .query(vec![("sig", sig)]);
        self.client.send_bytes(spec).await
    }
}
