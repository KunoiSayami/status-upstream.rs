/*
 ** Copyright (C) 2021-2022 KunoiSayami
 **
 ** This program is free software: you can redistribute it and/or modify
 ** it under the terms of the GNU Affero General Public License as published by
 ** the Free Software Foundation, either version 3 of the License, or
 ** any later version.
 **
 ** This program is distributed in the hope that it will be useful,
 ** but WITHOUT ANY WARRANTY; without even the implied warranty of
 ** MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 ** GNU Affero General Public License for more details.
 **
 ** You should have received a copy of the GNU Affero General Public License
 ** along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

#[async_trait::async_trait]
pub trait ServiceChecker {
    async fn ping(&self, timeout: u64) -> anyhow::Result<bool>;
}

#[async_trait::async_trait]
impl<F: ?Sized + Sync + Send> ServiceChecker for Box<F>
where
    F: ServiceChecker + Sync + Send,
{
    async fn ping(&self, timeout: u64) -> anyhow::Result<bool> {
        (**self).ping(timeout).await
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ServiceType {
    HTTP,
    SSH,
    TeamSpeak,
}

pub mod teamspeak {
    use crate::connlib::ServiceChecker;
    use tokio::net::UdpSocket;
    use tokio::time::Duration;

    const HEAD_DATA: [u8; 34] =
        hex_literal::hex!("545333494e49543100650000880ef967a500613f9e6966788d480000000000000000");

    pub struct TeamSpeak {
        remote_address: String,
    }

    impl TeamSpeak {
        pub fn new(remote_address: &str) -> Self {
            Self {
                remote_address: remote_address.to_string(),
            }
        }
    }
    #[async_trait::async_trait]
    impl ServiceChecker for TeamSpeak {
        // TODO: Support ipv6
        async fn ping(&self, timeout: u64) -> anyhow::Result<bool> {
            let socket = UdpSocket::bind("0.0.0.0:0").await?;

            socket.send_to(&HEAD_DATA, &self.remote_address).await?;

            //socket.set_read_timeout(Duration::from_secs(1));

            let mut buf = [0; 64];
            if let Ok(ret) =
                tokio::time::timeout(Duration::from_secs(timeout), socket.recv_from(&mut buf)).await
            {
                if let Ok((amt, _src)) = ret {
                    Ok(amt != 0)
                } else {
                    Ok(false)
                }
            } else {
                Ok(false)
            }
        }
    }
}

pub mod ssh {
    use crate::connlib::ServiceChecker;
    use spdlog::error;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;
    use tokio::time::Duration;

    const HEAD_DATA: [u8; 21] = hex_literal::hex!("5353482d322e302d4f70656e5353485f382e370d0a");

    pub struct SSH {
        remote_address: String,
    }

    impl SSH {
        pub fn new(remote_address: &str) -> Self {
            Self {
                remote_address: remote_address.to_string(),
            }
        }

        async fn ping_(&self, timeout: u64) -> anyhow::Result<bool> {
            if let Ok(mut socket) = tokio::time::timeout(
                Duration::from_secs(timeout),
                TcpStream::connect(&self.remote_address),
            )
            .await?
            {
                if let Ok(_) =
                    tokio::time::timeout(Duration::from_secs(timeout), socket.write_all(&HEAD_DATA))
                        .await?
                {
                    let mut buff = [0; 64];
                    if let Ok(_) =
                        tokio::time::timeout(Duration::from_secs(timeout), socket.read(&mut buff))
                            .await
                    {
                        return Ok(String::from_utf8_lossy(&buff).contains("SSH"));
                    }
                }
            }
            Ok(false)
        }
    }

    #[async_trait::async_trait]
    impl ServiceChecker for SSH {
        async fn ping(&self, timeout: u64) -> anyhow::Result<bool> {
            match self.ping_(timeout).await {
                Ok(ret) => Ok(ret),
                Err(e) => {
                    error!("Got error in ping {} {:?}", &self.remote_address, e);
                    Ok(false)
                }
            }
        }
    }
}

pub mod http {
    use crate::connlib::ServiceChecker;
    use reqwest::tls::Version;
    use reqwest::ClientBuilder;
    use std::time::Duration;

    pub struct HTTP {
        remote_address: String,
    }

    impl HTTP {
        pub fn new(remote_address: &str) -> Self {
            Self {
                remote_address: remote_address.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl ServiceChecker for HTTP {
        async fn ping(&self, timeout: u64) -> anyhow::Result<bool> {
            let client = ClientBuilder::new()
                .timeout(Duration::from_secs(timeout))
                .min_tls_version(Version::TLS_1_2)
                .build()?;
            let req = client.get(&self.remote_address).send().await?;
            let status = req.status().as_u16();
            Ok((300 > status) && (status >= 200))
        }
    }
}

#[derive(Debug, Clone)]
pub enum ServerLastStatus {
    Optional,
    Outage,
    Unknown,
}

impl Into<bool> for &ServerLastStatus {
    fn into(self) -> bool {
        match self {
            ServerLastStatus::Optional => true,
            _ => false,
        }
    }
}

impl Into<bool> for ServerLastStatus {
    fn into(self) -> bool {
        match self {
            ServerLastStatus::Optional => true,
            _ => false,
        }
    }
}

impl From<&ComponentStatus> for ServerLastStatus {
    fn from(status: &ComponentStatus) -> Self {
        match status {
            ComponentStatus::Operational => Self::Optional,
            _ => Self::Outage,
        }
    }
}

impl From<bool> for ServerLastStatus {
    fn from(b: bool) -> Self {
        Self::from(&b)
    }
}

impl From<&bool> for ServerLastStatus {
    fn from(b: &bool) -> Self {
        if *b {
            Self::Optional
        } else {
            Self::Outage
        }
    }
}

impl PartialEq<bool> for ServerLastStatus {
    fn eq(&self, other: &bool) -> bool {
        match self {
            ServerLastStatus::Optional => *other,
            ServerLastStatus::Outage => !other,
            // use false to make sure target is updated
            ServerLastStatus::Unknown => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ServiceWrapper {
    last_status: ServerLastStatus,
    remote_address: String,
    report_uuid: String,
    service_type: ServiceType,
    count: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ComponentResponse {
    status: String,
}

impl ComponentResponse {
    pub fn status(&self) -> &str {
        &self.status
    }
}

impl ServiceWrapper {
    pub fn report_uuid(&self) -> &str {
        &self.report_uuid
    }
    pub fn service_type(&self) -> &ServiceType {
        &self.service_type
    }

    pub async fn ping(&self, timeout: u64) -> anyhow::Result<bool> {
        match self.service_type() {
            ServiceType::HTTP => HTTP::new(&self.remote_address).ping(timeout).await,
            ServiceType::SSH => SSH::new(&self.remote_address).ping(timeout).await,
            ServiceType::TeamSpeak => TeamSpeak::new(&self.remote_address).ping(timeout).await,
        }
    }
    pub fn last_status(&self) -> &ServerLastStatus {
        &self.last_status
    }
    pub fn remote_address(&self) -> &str {
        &self.remote_address
    }

    pub fn ongoing_recheck(&self) -> bool {
        self.count > 0
    }

    pub fn check_last_status_eq(&self, last_status: bool) -> bool {
        self.last_status == last_status
    }

    pub fn update_last_status_condition(&mut self, last_status: bool, condition: u64) -> bool {
        if self.last_status != last_status {
            if self.count >= condition {
                self.last_status = ServerLastStatus::from(last_status);
                self.count = 0;
                true
            } else {
                self.count += 1;
                false
            }
        } else {
            self.count = 0;
            false
        }
    }

    pub fn reset_count(&mut self) {
        self.count = 0
    }

    pub async fn from_service(upstream: &Upstream, s: &Service) -> anyhow::Result<Self> {
        let status = upstream.get_component_status(s.report_uuid()).await?;
        let status = status.json::<ComponentResponse>().await?;
        Self::new_with_last_status(s, ServerLastStatus::from(&ComponentStatus::from(&status)))
    }

    pub fn new_with_last_status(
        s: &Service,
        last_status: ServerLastStatus,
    ) -> anyhow::Result<Self> {
        let service_type = s.service_type().to_lowercase();
        let service_type = match service_type.as_str() {
            "teamspeak" | "ts" => ServiceType::TeamSpeak,
            "ssh" => ServiceType::SSH,
            "http" => ServiceType::HTTP,
            &_ => {
                return Err(anyhow!(
                    "Unexpect service type: {}, identify id => {}",
                    s.service_type(),
                    s.report_uuid()
                ));
            }
        };
        Ok(Self::new(
            last_status.clone(),
            s.report_uuid().to_string(),
            service_type,
            s.remote_address().to_string(),
        ))
    }

    pub fn new(
        last_status: ServerLastStatus,
        identify_id: String,
        service_type: ServiceType,
        remote_address: String,
    ) -> Self {
        Self {
            last_status,
            report_uuid: identify_id,
            service_type,
            remote_address,
            count: 0,
        }
    }
}

use crate::configure::Service;
use crate::statuspagelib::Upstream;
use crate::ComponentStatus;
use anyhow::anyhow;
pub use http::HTTP;
use serde_derive::Deserialize;
pub use ssh::SSH;
pub use teamspeak::TeamSpeak;
