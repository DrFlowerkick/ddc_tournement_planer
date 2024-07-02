//! main.rs

use ddc_tournement_planer::{
    configuration::get_configuration,
    error::AppResult,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> AppResult<()> {
    let subscriber = get_subscriber(
        "ddc_tournement_planer".into(),
        "info".into(),
        std::io::stdout,
    );
    init_subscriber(subscriber);

    // Panic if we can't read configuration
    let _configuration = get_configuration().expect("Failed to read configuration.");

    Ok(())
}
