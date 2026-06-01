mod api;
mod acp;
mod git;
mod persistence;
mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Nexum starting up...");
    Ok(())
}
