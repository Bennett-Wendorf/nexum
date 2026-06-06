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

    config::init().map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;
    tracing::info!("Configuration loaded successfully");

    let repo_root = std::env::current_dir()?;
    let scheduler = overlord::OverlordScheduler::new(repo_root);
    let scheduler = std::sync::Arc::new(scheduler);

    let scheduler_clone = scheduler.clone();
    let join_handle = tokio::spawn(async move {
        if let Err(e) = scheduler_clone.start().await {
            tracing::error!("Overlord scheduler error: {}", e);
        }
    });
    tracing::info!("Overlord scheduler spawned as background task");

    // Set up signal handlers
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;

    tokio::select! {
        _ = sigint.recv() => tracing::info!("Received SIGINT, shutting down..."),
        _ = sigterm.recv() => tracing::info!("Received SIGTERM, shutting down..."),
    };

    scheduler.stop();
    let _ = join_handle.await;
    tracing::info!("Shutdown complete");

    Ok(())
}
