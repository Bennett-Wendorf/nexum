mod api;
mod acp;
mod git;
mod persistence;
mod config;
mod overlord;

use tokio::signal::unix::{signal, SignalKind};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Nexum starting up...");

    // Set up signal handlers before spawning any tasks
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;

    // Initialize configuration
    config::init().map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;
    tracing::info!("Configuration loaded successfully");

    // Initialize Overlord scheduler
    let repo_root = std::env::current_dir()?;
    let scheduler = overlord::OverlordScheduler::new(repo_root);
    let scheduler = std::sync::Arc::new(scheduler);
    tracing::info!("Overlord scheduler initialized");

    let scheduler_clone = scheduler.clone();
    let join_handle = tokio::spawn(async move {
        if let Err(e) = scheduler_clone.start().await {
            tracing::error!("Overlord scheduler error: {}", e);
        }
    });
    tracing::info!("Overlord scheduler spawned as background task");

    tokio::select! {
        _ = sigint.recv() => tracing::info!("Received SIGINT, shutting down..."),
        _ = sigterm.recv() => tracing::info!("Received SIGTERM, shutting down..."),
    };

    scheduler.stop();
    match tokio::time::timeout(std::time::Duration::from_secs(5), join_handle).await {
        Ok(result) => {
            if let Err(e) = result {
                tracing::error!("Scheduler task panicked: {}", e);
            }
        }
        Err(_) => {
            tracing::warn!("Scheduler did not stop in time, forcing exit");
        }
    }
    tracing::info!("Shutdown complete");

    Ok(())
}
