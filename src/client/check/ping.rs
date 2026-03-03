use super::Check;
use crate::model::{CheckReport, ComponentStatus};
use futures_util::stream::StreamExt;
use std::net::IpAddr;
use std::time::Instant;

pub struct PingCheck {
    component_id: String,
    host: String,
    timeout: u64,
}

impl PingCheck {
    pub fn new(component_id: String, host: String, timeout: u64) -> Self {
        Self {
            component_id,
            host,
            timeout,
        }
    }
}

#[async_trait::async_trait]
impl Check for PingCheck {
    fn name(&self) -> &str {
        "ping"
    }

    fn component_id(&self) -> &str {
        &self.component_id
    }

    async fn execute(&self) -> CheckReport {
        let start = Instant::now();

        let addr: IpAddr = match self.host.parse() {
            Ok(a) => a,
            Err(e) => {
                return CheckReport::new(
                    self.component_id.clone(),
                    ComponentStatus::MajorOutage,
                    Some(format!("Invalid IP address '{}': {e}", self.host)),
                    None,
                );
            }
        };

        let pinger = match tokio_icmp_echo::Pinger::new().await {
            Ok(p) => p,
            Err(e) => {
                return CheckReport::new(
                    self.component_id.clone(),
                    ComponentStatus::MajorOutage,
                    Some(format!("Failed to create pinger (need root?): {e}")),
                    None,
                );
            }
        };

        let result = pinger.chain(addr).stream().take(1).next().await;
        let latency = start.elapsed().as_millis() as u64;

        match result {
            Some(Ok(Some(_duration))) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::Operational,
                None,
                Some(latency),
            ),
            Some(Ok(None)) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::MajorOutage,
                Some("No ping response".to_string()),
                Some(latency),
            ),
            Some(Err(e)) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::MajorOutage,
                Some(format!("Ping error: {e}")),
                Some(latency),
            ),
            None => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::MajorOutage,
                Some("Ping stream ended unexpectedly".to_string()),
                Some(latency),
            ),
        }
    }
}
