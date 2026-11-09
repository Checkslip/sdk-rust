//! Look up one slip by its QR text.
//!
//! ```sh
//! CHECKSLIP_API_KEY=csk_... cargo run --example lookup_qr -- '<qr text>'
//! ```

use checkslip::{Client, LookupRequest, Outcome};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let qr = std::env::args()
        .nth(1)
        .ok_or("pass the slip QR text as the first argument")?;
    let client = Client::from_env()?;

    let check = client.slips().lookup(LookupRequest::qr(qr)).await?;
    println!("request_id: {}", check.request_id);
    println!("outcome:    {}", check.outcome);

    if check.outcome == Outcome::Found {
        if let Some(slip) = check.slip.as_full() {
            println!(
                "amount:     {}.{:02} THB",
                slip.amount.satang / 100,
                slip.amount.satang % 100
            );
            println!("sent at:    {}", slip.transferred_at);
            println!("from bank:  {}", slip.sender.bank.code);
        }
    }
    Ok(())
}
