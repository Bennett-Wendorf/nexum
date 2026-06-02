mod api;
mod acp;
mod git;
mod persistence;
mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Nexum starting up...");

    // Initialize configuration
    match config::init() {
        Ok(()) => tracing::info!("Configuration loaded successfully"),
        Err(e) => {
            tracing::error!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
