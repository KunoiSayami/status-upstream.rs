use super::Check;
use crate::model::{CheckReport, ComponentStatus};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;

pub struct SubnetCheck {
    component_id: String,
    network: String,
    port: u16,
    timeout: u64,
}

impl SubnetCheck {
    pub fn new(component_id: String, network: String, port: u16, timeout: u64) -> Self {
        Self {
            component_id,
            network,
            port,
            timeout,
        }
    }
}

#[async_trait::async_trait]
impl Check for SubnetCheck {
    fn name(&self) -> &str {
        "subnet"
    }

    fn component_id(&self) -> &str {
        &self.component_id
    }

    async fn execute(&self) -> CheckReport {
        let start = Instant::now();

        let network: ipnet::IpNet = match self.network.parse() {
            Ok(n) => n,
            Err(e) => {
                return CheckReport::new(
                    self.component_id.clone(),
                    ComponentStatus::MajorOutage,
                    Some(format!("Invalid network '{}': {e}", self.network)),
                    None,
                );
            }
        };

        let hosts: Vec<_> = network.hosts().collect();
        if hosts.is_empty() {
            return CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::Unknown,
                Some("No hosts in network".to_string()),
                None,
            );
        }

        let timeout = Duration::from_secs(self.timeout);
        let port = self.port;

        let mut handles = Vec::with_capacity(hosts.len());
        for host in &hosts {
            let addr = format!("{host}:{port}");
            handles.push(tokio::spawn(async move {
                tokio::time::timeout(timeout, TcpStream::connect(&addr))
                    .await
                    .map_or(false, |r| r.is_ok())
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            results.push(handle.await.unwrap_or(false));
        }

        let latency = start.elapsed().as_millis() as u64;
        let passed = results.iter().filter(|&&ok| ok).count();
        let total = results.len();
        let status = ComponentStatus::from(results.as_slice());

        CheckReport::new(
            self.component_id.clone(),
            status,
            Some(format!("{passed}/{total} hosts reachable on port {port}")),
            Some(latency),
        )
    }
}
