mod acp;
mod api;
mod builder;
mod config;
mod git;
mod overlord;
mod persistence;

use std::time::Duration;

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
    let scheduler = overlord::OverlordScheduler::new(repo_root.clone());
    let scheduler = std::sync::Arc::new(scheduler);
    tracing::info!("Overlord scheduler initialized");

    let scheduler_clone = scheduler.clone();
    let join_handle = tokio::spawn(async move {
        if let Err(e) = scheduler_clone.start().await {
            tracing::error!("Overlord scheduler error: {}", e);
        }
    });
    tracing::info!("Overlord scheduler spawned as background task");

    // Initialize Builder Workflow Orchestrator
    let cfg = config::get().expect("Config should be loaded");
    let agent_config = crate::acp::AgentConfig {
        name: "builder".to_string(),
        binary: std::env::current_exe().unwrap_or_else(|_| "/usr/bin/nexum".into()),
        args: vec!["acp".to_string()],
        env: std::collections::HashMap::new(),
    };
    let orchestrator = builder::orchestrator::WorkflowOrchestrator::new(
        repo_root.clone(),
        scheduler.clone(),
        agent_config,
        cfg.global.max_parallel as usize,
        Duration::from_secs(cfg.global.default_timeout_seconds),
        64, // event bus broadcast capacity
    );
    let orchestrator = std::sync::Arc::new(orchestrator);
    tracing::info!(
        concurrency_limit = cfg.global.max_parallel,
        task_timeout_secs = cfg.global.default_timeout_seconds,
        "Builder workflow orchestrator initialized"
    );

    // Start the REST API server
    let state = api::AppState {
        repo_root: repo_root.clone(),
        config: cfg.clone(),
        orchestrator: Some(orchestrator),
    };
    let app = api::create_router(state);

    let bind_addr = format!("{}:{}", cfg.global.server_host, cfg.global.server_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("REST API server listening on {}", bind_addr);

    let server_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("REST API server error: {}", e);
        }
    });
    tracing::info!("REST API server spawned as background task");

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

    // Wait for the API server to shut down
    match tokio::time::timeout(std::time::Duration::from_secs(5), server_handle).await {
        Ok(result) => {
            if let Err(e) = result {
                tracing::error!("API server task panicked: {}", e);
            }
        }
        Err(_) => {
            tracing::warn!("API server did not stop in time, forcing exit");
        }
    }
    tracing::info!("Shutdown complete");

    Ok(())
}
