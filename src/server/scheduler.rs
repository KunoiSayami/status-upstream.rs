use std::path::PathBuf;
use std::time::Duration;
use tokio::time;

/// Spawns a background task that periodically runs the local client subprocess.
pub fn spawn_local_client(
    interval_secs: u64,
    client_config_path: PathBuf,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(interval_secs));
        interval.tick().await; // first tick completes immediately

        loop {
            interval.tick().await;

            let exe = match std::env::current_exe() {
                Ok(e) => e,
                Err(e) => {
                    tracing::error!("Failed to get current executable path: {e}");
                    continue;
                }
            };

            let config_str = client_config_path.to_string_lossy().to_string();
            tracing::debug!("Spawning local client with config {config_str}");

            match tokio::process::Command::new(&exe)
                .args(["client", "--config", &config_str, "--once"])
                .output()
                .await
            {
                Ok(output) => {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        tracing::warn!("Local client exited with {}: {stderr}", output.status);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to spawn local client: {e}");
                }
            }
        }
    })
}
