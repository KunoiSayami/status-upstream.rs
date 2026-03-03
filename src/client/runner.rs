use crate::client::check::{self, Check};
use crate::config::client::ClientRootConfig;
use crate::model::{CheckReport, ClientReport};
use std::time::Duration;

pub struct Runner {
    checks: Vec<Box<dyn Check>>,
    server_url: String,
    auth_token: String,
    client_id: String,
    check_interval: u64,
}

impl Runner {
    pub fn from_config(config: &ClientRootConfig) -> Self {
        let checks: Vec<Box<dyn Check>> = config.checks().iter().map(check::from_config).collect();

        tracing::info!("Loaded {} checks", checks.len());
        for c in &checks {
            tracing::debug!("  Check: {} -> component {}", c.name(), c.component_id());
        }

        Self {
            checks,
            server_url: config.client().server_url().to_string(),
            auth_token: config.client().auth_token().to_string(),
            client_id: config.client().client_id().to_string(),
            check_interval: config.client().check_interval(),
        }
    }

    pub async fn run_once(&self) -> anyhow::Result<()> {
        let reports = self.execute_checks().await;
        self.send_report(reports).await
    }

    pub async fn run_loop(&self) -> anyhow::Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(self.check_interval));

        loop {
            interval.tick().await;

            let reports = self.execute_checks().await;
            if let Err(e) = self.send_report(reports).await {
                tracing::error!("Failed to send report: {e}");
            }
        }
    }

    async fn execute_checks(&self) -> Vec<CheckReport> {
        let mut reports = Vec::with_capacity(self.checks.len());

        for check in &self.checks {
            tracing::debug!("Running check: {} ({})", check.name(), check.component_id());
            let report = check.execute().await;
            tracing::info!(
                "Check {} [{}]: {}",
                check.component_id(),
                check.name(),
                report.status()
            );
            reports.push(report);
        }

        reports
    }

    async fn send_report(&self, checks: Vec<CheckReport>) -> anyhow::Result<()> {
        if checks.is_empty() {
            return Ok(());
        }

        let report = ClientReport::new(self.client_id.clone(), checks);
        let url = format!("{}/api/v1/report", self.server_url.trim_end_matches('/'));

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let mut request = client.post(&url).json(&report);
        if !self.auth_token.is_empty() {
            request = request.bearer_auth(&self.auth_token);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            tracing::debug!("Report sent successfully");
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::warn!("Server responded with {status}: {body}");
        }

        Ok(())
    }
}
