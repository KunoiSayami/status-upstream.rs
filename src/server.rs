mod api;
mod db;
pub mod notifier;
mod scheduler;

use crate::config::server::ServerRootConfig;
use notifier::NotifierRegistry;
use std::path::Path;
use std::sync::Arc;

pub async fn run(config_path: &Path) -> anyhow::Result<()> {
    let config: ServerRootConfig = crate::config::load_toml(config_path).await?;

    tracing::info!(
        "{} {} starting in server mode",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    let pool = db::connect(config.server().database()).await?;
    tracing::info!("Database connected: {}", config.server().database());

    for component in config.components() {
        db::ensure_component(&pool, component.id(), component.name()).await?;
        tracing::info!(
            "Component registered: {} ({})",
            component.name(),
            component.id()
        );
    }

    let mut notifiers = NotifierRegistry::new();

    if let Some(sp_config) = config.notifiers().statuspage() {
        if sp_config.enabled() {
            let n = notifier::statuspage::StatusPageNotifier::new(sp_config)?;
            notifiers.register(Box::new(n));
        }
    }

    if let Some(tg_config) = config.notifiers().telegram() {
        if tg_config.enabled() {
            let n = notifier::telegram::TelegramNotifier::new(tg_config);
            notifiers.register(Box::new(n));
        }
    }

    let state = Arc::new(api::AppState::new(
        pool,
        notifiers,
        config.server().auth_token().to_string(),
        config.server().public_status_page(),
    ));

    let router = api::make_router(state);

    let bind_addr = format!("{}:{}", config.server().bind(), config.server().port());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("Server listening on {bind_addr}");

    // Spawn local client scheduler if local checks are configured
    let _scheduler_handle = if !config.local_checks().is_empty() {
        let local_config_path = config_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("local-client.toml");
        tracing::info!(
            "Local checks configured, scheduler will use {}",
            local_config_path.display()
        );
        Some(scheduler::spawn_local_client(
            config.server().check_interval(),
            local_config_path,
        ))
    } else {
        None
    };

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");
    tracing::info!("Received Ctrl+C, shutting down...");
}
