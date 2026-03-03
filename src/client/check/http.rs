use super::Check;
use crate::model::{CheckReport, ComponentStatus};
use std::time::{Duration, Instant};

pub struct HttpCheck {
    component_id: String,
    url: String,
    expected_status: u16,
    timeout: u64,
}

impl HttpCheck {
    pub fn new(component_id: String, url: String, expected_status: u16, timeout: u64) -> Self {
        Self {
            component_id,
            url,
            expected_status,
            timeout,
        }
    }
}

#[async_trait::async_trait]
impl Check for HttpCheck {
    fn name(&self) -> &str {
        &self.url
    }

    fn component_id(&self) -> &str {
        &self.component_id
    }

    async fn execute(&self) -> CheckReport {
        let start = Instant::now();

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.timeout))
            .build();

        let client = match client {
            Ok(c) => c,
            Err(e) => {
                return CheckReport::new(
                    self.component_id.clone(),
                    ComponentStatus::MajorOutage,
                    Some(format!("Failed to build HTTP client: {e}")),
                    None,
                );
            }
        };

        let result = client.get(&self.url).send().await;
        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                let status_code = response.status().as_u16();
                if status_code == self.expected_status {
                    CheckReport::new(
                        self.component_id.clone(),
                        ComponentStatus::Operational,
                        None,
                        Some(latency),
                    )
                } else {
                    CheckReport::new(
                        self.component_id.clone(),
                        ComponentStatus::MajorOutage,
                        Some(format!(
                            "Expected status {}, got {status_code}",
                            self.expected_status
                        )),
                        Some(latency),
                    )
                }
            }
            Err(e) if e.is_timeout() => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::MajorOutage,
                Some("Request timed out".to_string()),
                Some(latency),
            ),
            Err(e) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::MajorOutage,
                Some(format!("Request failed: {e}")),
                Some(latency),
            ),
        }
    }
}
