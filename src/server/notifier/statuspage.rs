use super::Notifier;
use crate::config::server::StatusPageConfig;
use crate::model::ComponentStatus;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use std::time::Duration;

const UPSTREAM_URL: &str = "https://api.statuspage.io/";

pub struct StatusPageNotifier {
    client: Client,
    config: StatusPageConfig,
}

impl StatusPageNotifier {
    pub fn new(config: &StatusPageConfig) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_str(config.api_key())?);

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            client,
            config: config.clone(),
        })
    }

    fn build_url(&self, page_id: &str, component_id: &str) -> String {
        format!("{UPSTREAM_URL}v1/pages/{page_id}/components/{component_id}")
    }
}

fn to_statuspage_status(status: ComponentStatus) -> &'static str {
    match status {
        ComponentStatus::Operational => "operational",
        ComponentStatus::DegradedPerformance => "degraded_performance",
        ComponentStatus::PartialOutage => "partial_outage",
        ComponentStatus::MajorOutage => "major_outage",
        ComponentStatus::UnderMaintenance => "under_maintenance",
        ComponentStatus::Unknown => "operational",
    }
}

#[async_trait::async_trait]
impl Notifier for StatusPageNotifier {
    fn name(&self) -> &str {
        "statuspage"
    }

    async fn notify(
        &self,
        component_id: &str,
        _component_name: &str,
        _old: ComponentStatus,
        new: ComponentStatus,
    ) -> anyhow::Result<()> {
        let mapping = match self.config.components().get(component_id) {
            Some(m) => m,
            None => return Ok(()),
        };

        let url = self.build_url(mapping.page_id(), mapping.component_id());
        let payload = serde_json::json!({
            "component": {
                "status": to_statuspage_status(new)
            }
        });

        self.client.patch(&url).json(&payload).send().await?;
        tracing::debug!("Statuspage updated {component_id} to {new}");
        Ok(())
    }
}
