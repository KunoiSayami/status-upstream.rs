
mod check;
mod runner;

use crate::config::client::ClientRootConfig;
use std::path::Path;

pub async fn run(config_path: &Path, once: bool) -> anyhow::Result<()> {
    let config: ClientRootConfig = crate::config::load_toml(config_path).await?;

    tracing::info!(
        "{} {} starting in client mode (client_id: {})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        config.client().client_id()
    );

    let runner = runner::Runner::from_config(&config);

    if once {
        runner.run_once().await
    } else {
        tokio::select! {
            result = runner.run_loop() => result,
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received Ctrl+C, shutting down client...");
                Ok(())
            }
        }
    }
}
