# checkslip (ภาษาไทย)

ไลบรารี Rust อย่างเป็นทางการสำหรับ API ของ [Checkslip](https://checkslip.com)

[English](README.md)

Checkslip ดึงข้อมูลรายการโอนเงินของธนาคารไทยจากสลิป คุณส่งข้อความใน QR ของสลิป
หรือส่งรูปสลิปมา แล้ว Checkslip ไปถามธนาคารผู้โอนว่ามีรายการนั้นหรือไม่
สิ่งที่ได้กลับไปคือข้อมูลที่ธนาคารมี ได้แก่ ยอดเงิน เวลา ผู้โอน และผู้รับ
การตัดสินใจว่าจะรับเงินหรือไม่เป็นของคุณ ให้เทียบข้อมูลเหล่านี้กับออเดอร์ของคุณเองก่อน

เอกสาร API: <https://checkslip.com/docs/reference>

## ติดตั้ง

```sh
cargo add checkslip
cargo add tokio --features "macros rt-multi-thread"
```

ต้องใช้ Rust 1.80 ขึ้นไป · รุ่น 0.4.0 และก่อนนั้นใช้ 1.75 แล้วขยับใน 0.4.1
เพื่อให้ feature `native-tls` ใช้ OpenSSL รุ่นที่ปิดช่องโหว่แล้วได้

## เริ่มใช้งาน

ออก API key ในเว็บแอปของ Checkslip (ขึ้นต้นด้วย `csk_`) อย่าเขียน key
ลงในซอร์สโค้ด ให้ส่งผ่าน environment variable แทน

```rust,no_run
use checkslip::{Client, LookupRequest, Outcome};

#[tokio::main]
async fn main() -> checkslip::Result<()> {
    let client = Client::from_env()?; // อ่านจาก CHECKSLIP_API_KEY

    let check = client
        .slips()
        .lookup(
            LookupRequest::qr("00330006000001010306902126302105340875102TH9104F86F")
                .client_reference("order-1042"),
        )
        .await?;

    match check.outcome {
        Outcome::Found => {
            let slip = check.slip.as_full().expect("FOUND มีข้อมูลเต็ม");
            println!("{} สตางค์ จาก {}", slip.amount.satang, slip.sender.bank.code);
        }
        Outcome::Duplicate => println!("บัญชีนี้เคยดึงรายการนี้แล้ว"),
        Outcome::NotFound => println!("ธนาคารไม่มีรายการตามเลขอ้างอิงนี้"),
        Outcome::Pending => println!("ธนาคารยังไม่ขึ้นรายการ"),
        other => println!("ผล {other}"),
    }
    Ok(())
}
```

หักเครดิตเฉพาะเมื่อผลเป็น `FOUND`

ถ้าลูกค้าอัปโหลดรูปสลิป (JPEG, PNG หรือ WebP ไม่เกิน 10 MB) ส่งไบต์ของรูปได้เลย
ไลบรารีส่งเป็น `multipart/form-data` และดูชนิดไฟล์จากส่วนหัวของไฟล์ให้

```rust,no_run
# async fn run(client: checkslip::Client, bytes: Vec<u8>) -> checkslip::Result<()> {
use checkslip::LookupRequest;

let check = client.slips().lookup(LookupRequest::image(bytes)).await?;
# Ok(())
# }
```

Checkslip อ่าน QR จากรูปแล้วไม่เก็บรูปไว้

คำสั่งอื่นมีครบตามตารางใน [README ภาษาอังกฤษ](README.md#other-calls) เช่น
`client.billing().balance().get()` ดูเครดิตคงเหลือ, `client.usage().get(..)` ดูยอดใช้งาน,
`client.banks().list()` ดูรายชื่อธนาคารที่รองรับ (ไม่ต้องใช้ API key)
และ `client.webhooks()` จัดการ webhook

## ตั้งค่า

```rust,no_run
use std::time::Duration;

let client = checkslip::Client::builder()
    .api_key(std::env::var("CHECKSLIP_API_KEY").unwrap())
    .timeout(Duration::from_secs(20)) // ต่อหนึ่งครั้งที่ส่ง ค่าเริ่มต้น 30 วินาที
    .max_retries(3)                   // ค่าเริ่มต้น 2
    .user_agent("my-shop/1.4")
    .build()?;
# Ok::<(), checkslip::Error>(())
```

`base_url` ใช้ชี้ไปที่อื่น เช่น mock server ในเทสของคุณ ส่วน `http_client`
รับ `reqwest::Client` ของคุณเองเมื่อต้องใช้ proxy หรือตั้งค่า TLS เอง

## ข้อผิดพลาด

API ตอบข้อผิดพลาดเป็น problem document ตาม RFC 9457
(`application/problem+json`) ทุกอันมี `code` ที่ไม่เปลี่ยน ให้เขียนโค้ดเทียบกับ `code`
ส่วน `title` และ `detail` เป็นข้อความสำหรับคนอ่าน และมักเป็นภาษาไทย

```rust,no_run
# async fn run(client: checkslip::Client) {
use checkslip::{Error, LookupRequest};

match client.slips().lookup(LookupRequest::qr("...")).await {
    Ok(check) => println!("{}", check.outcome),
    Err(Error::Api { code, .. }) if code == "NO_CREDIT" => println!("เครดิตหมด"),
    Err(Error::Api { status: 422, code, .. }) => println!("สลิปมีปัญหา: {code}"),
    Err(Error::RateLimited { retry_after, .. }) => println!("รอ {retry_after:?} แล้วลองใหม่"),
    Err(e) => println!("{e}"),
}
# }
```

| ชนิด | เกิดเมื่อ |
|------|----------|
| `Error::Api { status, code, title, detail, request_id, problem }` | เซิร์ฟเวอร์ตอบสถานะผิดพลาด `problem` คือ body ทั้งก้อน รวมข้อผิดพลาดรายฟิลด์ |
| `Error::RateLimited { retry_after, request_id }` | ได้ HTTP 429 และลองซ้ำครบจำนวนแล้ว |
| `Error::Http(reqwest::Error)` | ไม่ได้คำตอบ เช่น ต่อไม่ติด TLS ล้มเหลว หรือหมดเวลา |
| `Error::Decode { source, request_id }` | ได้คำตอบสำเร็จแต่ body ไม่ตรงกับชนิดที่คาดไว้ |
| `Error::InvalidArgument(String)` | ค่าที่ส่งไม่ได้ เช่น API key มีขึ้นบรรทัดใหม่ |

เวลาติดต่อทีมงาน ให้แนบ `request_id` ไปด้วย

## การลองซ้ำ

ไลบรารีลองซ้ำให้เอง ค่าเริ่มต้น `max_retries(2)` แปลว่าส่งได้มากที่สุด 3 ครั้ง

- HTTP 429 ลองใหม่หลังรอตามจำนวนวินาทีใน `Retry-After` ถ้าเซิร์ฟเวอร์ให้รอเกิน
  60 วินาที (เปลี่ยนได้ด้วย `max_retry_after`) จะคืน `Error::RateLimited` ทันที
  ให้โค้ดของคุณตัดสินใจเอง
- เครือข่ายล่ม หมดเวลา หรือได้ 5xx จะลองซ้ำเฉพาะเมื่อส่งซ้ำแล้วไม่เกิดผลเสีย คือคำขอ
  `GET`, `PUT`, `DELETE` และ `POST` ที่มี `Idempotency-Key` ส่วน `POST` ที่ไม่มี key
  เช่น การสร้าง webhook endpoint จะไม่ส่งซ้ำ
- ถ้าการดึงข้อมูลสลิปได้ `409 IDEMPOTENCY_IN_FLIGHT` จะลองซ้ำ เพราะคำขอแรกที่ใช้ key
  เดียวกันยังทำงานอยู่
- ช่วงรอเริ่มที่ 500 มิลลิวินาที เพิ่มเป็นสองเท่าทุกครั้งจนถึง 8 วินาที และสุ่มค่าระหว่างครึ่งหนึ่ง
  ถึงเต็มช่วง เพื่อไม่ให้หลายเครื่องที่ล้มพร้อมกันกลับมายิงพร้อมกัน

4xx อื่น ๆ ไม่ลองซ้ำ

## Idempotency

การดึงข้อมูลสลิปทุกครั้งส่ง header `Idempotency-Key` ถ้าไม่ได้กำหนดเอง
ไลบรารีจะสุ่ม UUID ให้หนึ่งค่าต่อการเรียก และใช้ค่าเดิมทุกครั้งที่ลองซ้ำ
ผลจึงเป็นผลเดิมและหักเครดิตไม่เกินหนึ่งครั้ง

key ที่สุ่มให้มีอายุแค่ระหว่างการเรียกครั้งนั้น ถ้าโปรแกรมของคุณอาจล่มแล้วส่งสลิปเดิมใหม่ภายหลัง
ให้กำหนด key เองจากค่าที่ไม่เปลี่ยน เช่น เลขออเดอร์

```rust,no_run
# async fn run(client: checkslip::Client) -> checkslip::Result<()> {
use checkslip::LookupRequest;

let check = client
    .slips()
    .lookup(LookupRequest::qr("...").idempotency_key("order-1042-slip-1"))
    .await?;
# Ok(())
# }
```

ใช้ key เดิมกับคำขอที่ต่างกันไม่ได้ API จะตอบ `422 IDEMPOTENCY_KEY_REUSED`

คำสั่ง webhook ที่ต้องมี key (`recover`, `rotate_secret`, `send_test`, `redeliver`)
และ `audit().exports().create` รับ `Option<&str>` ส่ง `None` แล้วไลบรารีสร้างให้

## การแบ่งหน้า

endpoint ที่เป็นรายการตอบทีละหน้าพร้อม `next_cursor` แต่ละอันเรียกได้สองแบบ

- `list(&params)` ได้ `Page<T>` หนึ่งหน้า มี `items` และ `next_cursor`
- `list_all(params)` ได้ `Stream` ของรายการ และไปดึงหน้าถัดไปให้เมื่ออ่านถึง

```rust,no_run
# async fn run(client: checkslip::Client) -> checkslip::Result<()> {
use futures_util::TryStreamExt;
use checkslip::{ListSlipsParams, Outcome};

let mut found = client
    .slips()
    .list_all(ListSlipsParams::new().outcome([Outcome::Found]).limit(100));

while let Some(record) = found.try_next().await? {
    println!("{} {:?}", record.request_id, record.client_reference);
}
# Ok(())
# }
```

`TryStreamExt` มาจาก `futures-util` (หรือ `futures`) ถ้าดึงหน้าใดไม่สำเร็จ stream
จะคืน error นั้นแล้วจบ

## Webhook

เมื่อการดึงข้อมูลสลิปเสร็จ Checkslip ส่งข้อความ `slip.checked` ไปที่ endpoint ของคุณ
รวมถึงรายการ `PENDING` ที่ได้ผลภายหลัง ข้อความมีลายเซ็นแบบ
[Standard Webhooks](https://www.standardwebhooks.com/) เปิด feature `webhooks`
เพื่อใช้ตัวตรวจลายเซ็น

```sh
cargo add checkslip --features webhooks
```

```rust,no_run
# #[cfg(feature = "webhooks")]
# mod example {
use checkslip::webhook::Webhook;
use checkslip::WebhookEvent;
# fn handle(headers: &reqwest::header::HeaderMap, raw_body: &[u8]) -> Result<(), Box<dyn std::error::Error>> {

// secret whsec_... ที่ได้ตอนสร้าง endpoint โหลดจาก config ของคุณ
let hook = Webhook::new(&std::env::var("CHECKSLIP_WEBHOOK_SECRET")?)?;

// ใช้ body ดิบก่อน parse JSON
match hook.verify(headers, raw_body)? {
    WebhookEvent::SlipChecked { data, .. } => {
        println!("รายการ {} เสร็จแล้ว: {}", data.request_id, data.outcome);
    }
    WebhookEvent::Test { .. } => println!("ข้อความทดสอบ"),
    other => println!("ชนิดใหม่: {}", other.event_type()),
}
# Ok(())
# }
# }
```

`verify` อ่าน header `webhook-id`, `webhook-timestamp` และ `webhook-signature`
และไม่รับข้อความที่ลายเซ็นไม่ตรง หรือเวลาห่างจากนาฬิกาของคุณเกิน 5 นาที
(เปลี่ยนได้ด้วย `Webhook::tolerance`) ใน 24 ชั่วโมงหลังเปลี่ยน secret
ข้อความจะมีลายเซ็นสองชุด ตรงชุดใดชุดหนึ่งก็ผ่าน

ข้อความเดียวกันอาจมาถึงมากกว่าหนึ่งครั้ง เช่น หลังการส่งซ้ำหรือการเรียก `redeliver`
แต่ `webhook-id` ไม่เปลี่ยน ให้เก็บ ID ที่จัดการแล้วและข้ามตัวที่ซ้ำ

`axum` และ `actix-web` 4 ใช้ `HeaderMap` ของ `http` 1.x ซึ่งเป็นชนิดที่ `verify` รับ
ถ้าใช้ framework อื่น ให้อ่าน header ทั้งสามเองแล้วเรียก `verify_signature`

## Feature

| Feature | ค่าเริ่มต้น | เพิ่มอะไร |
|---------|------------|-----------|
| `rustls-tls` | เปิด | TLS ผ่าน rustls |
| `native-tls` | ปิด | TLS ผ่านไลบรารีของระบบ (OpenSSL, Schannel หรือ Secure Transport) |
| `webhooks` | ปิด | `checkslip::webhook` สำหรับตรวจลายเซ็น webhook |
| `blocking` | ปิด | `checkslip::blocking::Client` สำหรับโปรแกรมที่ไม่มี async runtime |

ถ้าจะใช้ TLS ของระบบแทน rustls

```sh
cargo add checkslip --no-default-features --features native-tls
```

blocking client รัน async client บน Tokio runtime ส่วนตัว ห้ามเรียกจากโค้ด async

## รองรับค่าใหม่ในอนาคต

enum ทุกตัวในไลบรารีเป็น `#[non_exhaustive]` และมี `Unknown(String)` เมื่อ API
เพิ่มค่าใหม่ เช่น outcome ใหม่ ไลบรารีรุ่นเก่ายังอ่านคำตอบได้และส่งข้อความดิบให้คุณ
เวลา `match` ให้ใส่แขนที่รับกรณีอื่นไว้เสมอ

## สัญญาอนุญาต

MIT ดู [LICENSE](LICENSE) แจ้งปัญหาด้านความปลอดภัยตาม [SECURITY.md](SECURITY.md)
