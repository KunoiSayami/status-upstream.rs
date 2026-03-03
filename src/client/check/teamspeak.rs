use super::Check;
use crate::model::{CheckReport, ComponentStatus};
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;

/// TS3INIT handshake packet header.
const TS3_INIT_HEADER: [u8; 34] =
    hex_literal::hex!("545333494e49543100650000880ef967a500613f9e6966788d480000000000000000");

pub struct TeamSpeakCheck {
    component_id: String,
    host: String,
    port: u16,
    timeout: u64,
}

impl TeamSpeakCheck {
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
impl Check for TeamSpeakCheck {
    fn name(&self) -> &str {
        "teamspeak"
    }

    fn component_id(&self) -> &str {
        &self.component_id
    }

    async fn execute(&self) -> CheckReport {
        let addr = self.address();
        let start = Instant::now();
        let timeout = Duration::from_secs(self.timeout);

        let result = tokio::time::timeout(timeout, async {
            // TODO: Support IPv6
            let socket = UdpSocket::bind("0.0.0.0:0").await?;
            socket.send_to(&TS3_INIT_HEADER, &addr).await?;

            let mut buf = [0u8; 64];
            let (amt, _) = socket.recv_from(&mut buf).await?;
            Ok::<bool, std::io::Error>(amt > 0)
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
                Some("Empty response from TeamSpeak server".to_string()),
                Some(latency),
            ),
            Ok(Err(e)) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::MajorOutage,
                Some(format!("TeamSpeak check failed: {e}")),
                Some(latency),
            ),
            Err(_) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::MajorOutage,
                Some("TeamSpeak check timed out".to_string()),
                Some(latency),
            ),
        }
    }
}
