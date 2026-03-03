use super::Check;
use crate::model::{CheckReport, ComponentStatus};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;

pub struct TcpCheck {
    component_id: String,
    host: String,
    port: u16,
    timeout: u64,
}

impl TcpCheck {
    pub fn new(component_id: String, host: String, port: u16, timeout: u64) -> Self {
        Self {
            component_id,
            host,
            port,
            timeout,
        }
    }

    fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[async_trait::async_trait]
impl Check for TcpCheck {
    fn name(&self) -> &str {
        "tcp"
    }

    fn component_id(&self) -> &str {
        &self.component_id
    }

    async fn execute(&self) -> CheckReport {
        let addr = self.address();
        let start = Instant::now();

        let result =
            tokio::time::timeout(Duration::from_secs(self.timeout), TcpStream::connect(&addr))
                .await;

        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(_)) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::Operational,
                None,
                Some(latency),
            ),
            Ok(Err(e)) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::MajorOutage,
                Some(format!("Connection failed: {e}")),
                Some(latency),
            ),
            Err(_) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::MajorOutage,
                Some("Connection timed out".to_string()),
                Some(latency),
            ),
        }
    }
}
