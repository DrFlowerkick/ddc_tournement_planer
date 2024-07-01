//! main.rs

use ddc_tournement_planer::{error::DTPResult, telemetry::{get_subscriber, init_subscriber}};

#[tokio::main]
async fn main() -> DTPResult<()> {
    let subscriber = get_subscriber("ddc_tournement_planer".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    Ok(())
}