use anyhow::Result;
use tracing::info;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Project Needle desktop app");
    needle_ui::run_native()?;
    Ok(())
}
