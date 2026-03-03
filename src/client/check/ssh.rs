use super::Check;
use crate::model::{CheckReport, ComponentStatus};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// SSH client banner sent during handshake.
const SSH_BANNER: &[u8] = b"SSH-2.0-StatusUpstream\r\n";

pub struct SshCheck {
    component_id: String,
    host: String,
    port: u16,
    timeout: u64,
}

impl SshCheck {
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
impl Check for SshCheck {
    fn name(&self) -> &str {
        "ssh"
    }

    fn component_id(&self) -> &str {
        &self.component_id
    }

    async fn execute(&self) -> CheckReport {
        let addr = self.address();
        let start = Instant::now();
        let timeout = Duration::from_secs(self.timeout);

        let result = tokio::time::timeout(timeout, async {
            let mut stream = TcpStream::connect(&addr).await?;
            stream.write_all(SSH_BANNER).await?;

            let mut buf = [0u8; 64];
            let n = stream.read(&mut buf).await?;
            let response = String::from_utf8_lossy(&buf[..n]);
            Ok::<bool, std::io::Error>(response.contains("SSH"))
        })
        .await;

        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(true)) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::Operational,
                None,
                Some(latency),
            ),
            Ok(Ok(false)) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::MajorOutage,
                Some("No SSH banner in response".to_string()),
                Some(latency),
            ),
            Ok(Err(e)) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::MajorOutage,
                Some(format!("SSH check failed: {e}")),
                Some(latency),
            ),
            Err(_) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::MajorOutage,
                Some("SSH check timed out".to_string()),
                Some(latency),
            ),
        }
    }
}
