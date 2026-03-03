
pub mod client;
pub mod server;

use std::path::Path;

pub async fn load_toml<T: serde::de::DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let content = tokio::fs::read_to_string(path).await?;
    let config = toml::from_str(&content)?;
    Ok(config)
}
