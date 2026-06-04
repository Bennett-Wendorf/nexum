mod api;
mod acp;
mod git;
mod persistence;
mod config;
mod overlord;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Nexum starting up...");

    // Initialize configuration
    config::init().map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;
    tracing::info!("Configuration loaded successfully");

    // Initialize Overlord scheduler
    let repo_root = std::env::current_dir()?;
    let scheduler = overlord::OverlordScheduler::new(repo_root);

    tracing::info!("Overlord scheduler initialized");
    let scheduler = std::sync::Arc::new(scheduler);
    let scheduler_clone = scheduler.clone();
    tokio::spawn(async move {
        if let Err(e) = scheduler_clone.start().await {
            tracing::error!("Overlord scheduler error: {}", e);
        }
    });
    tracing::info!("Overlord scheduler spawned as background task");

    Ok(())
}
