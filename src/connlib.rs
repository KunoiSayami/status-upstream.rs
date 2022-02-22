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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ServiceType {
    HTTP,
    SSH,
    TeamSpeak,
    Tcping,
    #[cfg(feature = "ping")]
    ICMP,
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
            if let Ok((amt, _src)) =
                tokio::time::timeout(Duration::from_secs(timeout), socket.recv_from(&mut buf))
                    .await?
            {
                Ok(amt != 0)
            } else {
                Ok(false)
            }
        }
    }
}

pub mod ssh {
    use crate::connlib::ServiceChecker;
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
    }

    #[async_trait::async_trait]
    impl ServiceChecker for SSH {
        async fn ping(&self, timeout: u64) -> anyhow::Result<bool> {
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
            let req = client.get(&self.remote_address).send().await;
            match req {
                Ok(req) => {
                    let status = req.status().as_u16();
                    Ok((300 > status) && (status >= 200))
                }
                Err(e) if e.is_timeout() => Ok(false),
                Err(e) => Err(anyhow::Error::from(e)),
            }
        }
    }
}

pub mod tcping {
    use crate::connlib::ServiceChecker;
    use std::io::ErrorKind;
    use std::time::Duration;
    use tokio::net::TcpStream;

    pub struct Tcping {
        remote_address: String,
    }

    impl Tcping {
        pub fn new(remote_address: &str) -> Self {
            Self {
                remote_address: remote_address.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl ServiceChecker for Tcping {
        async fn ping(&self, timeout: u64) -> anyhow::Result<bool> {
            match tokio::time::timeout(
                Duration::from_secs(timeout),
                TcpStream::connect(&self.remote_address),
            )
            .await?
            {
                Ok(_) => Ok(true),
                Err(e)
                    if e.kind().eq(&ErrorKind::ConnectionRefused)
                        | e.kind().eq(&ErrorKind::ConnectionReset)
                        | e.kind().eq(&ErrorKind::ConnectionAborted) =>
                {
                    Ok(false)
                }
                Err(e) => Err(anyhow::Error::from(e)),
            }
        }
    }

    #[cfg(test)]
    mod tcping_test {
        use crate::connlib::tcping::Tcping;
        use crate::connlib::ServiceChecker;
        use std::time::Duration;

        #[test]
        #[ignore]
        fn test() {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    let remote = Tcping::new("localhost:22");
                    let result = remote.ping(5).await;
                    println!("{:?}", result);
                    tokio::time::sleep(Duration::from_secs(20)).await;
                });
        }
    }
}

#[cfg(feature = "ping")]
pub mod icmp {
    use super::error;
    use super::ServiceChecker;
    use futures_util::stream::StreamExt;
    use std::net::IpAddr;

    pub struct ICMP {
        remote_address: IpAddr,
    }

    impl ICMP {
        pub fn new(remote_address: &str) -> Self {
            Self {
                remote_address: remote_address
                    .parse()
                    .map_err(|e| error!("Got error while parse {}, {:?}", remote_address, e))
                    .unwrap(),
            }
        }
    }

    #[async_trait::async_trait]
    impl ServiceChecker for ICMP {
        async fn ping(&self, _timeout: u64) -> anyhow::Result<bool> {
            let pinger = tokio_icmp_echo::Pinger::new()
                .await
                .map_err(|e| error!("Got error while create pinger: {:?}", e))
                .unwrap();
            let mut take = pinger.chain(self.remote_address).stream().take(1);
            if let Some(ret) = take.next().await {
                let r = ret
                    .map_err(|e| error!("Got error while ping: {:?}", e))
                    .unwrap_or(None);
                Ok(r.is_some())
            } else {
                Ok(false)
            }
        }
    }

    async fn async_test() {
        let addr = "127.0.0.1".parse().unwrap();

        let pinger = tokio_icmp_echo::Pinger::new();
        let stream = pinger.await.unwrap().chain(addr).stream();
        let mut ret = stream.take(3);
        let r = ret
            .next()
            .await
            .unwrap()
            .map_err(|e| println!("error: {:?}", e))
            .ok();
        println!("result: {:?}", r);
    }

    #[cfg(test)]
    #[test]
    fn test() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async_test());
    }

    pub async fn check_ping_available() -> anyhow::Result<()> {
        tokio_icmp_echo::Pinger::new().await?;
        Ok(())
    }
}

pub mod server_last_status {
    use crate::ComponentStatus;
    use std::fmt::Formatter;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum ServerLastStatus {
        Optional,
        Outage,
        DegradedPerformance,
        PartialOutage,
        Unknown,
    }

    impl From<&ComponentStatus> for ServerLastStatus {
        fn from(status: &ComponentStatus) -> Self {
            match status {
                ComponentStatus::Operational => Self::Optional,
                ComponentStatus::DegradedPerformance => Self::DegradedPerformance,
                ComponentStatus::PartialOutage => Self::PartialOutage,
                _ => Self::Outage,
            }
        }
    }

    impl TryFrom<&str> for ServerLastStatus {
        type Error = anyhow::Error;

        fn try_from(value: &str) -> Result<Self, Self::Error> {
            Ok(match value {
                "operational" => Self::Optional,
                "major_outage" => Self::Outage,
                "degraded_performance" => Self::DegradedPerformance,
                "partial_outage" => Self::PartialOutage,
                _ => Self::Unknown,
            })
        }
    }

    impl std::fmt::Display for ServerLastStatus {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    ServerLastStatus::Optional => "operational",
                    ServerLastStatus::Outage => "major_outage",
                    ServerLastStatus::DegradedPerformance => "degraded_performance",
                    ServerLastStatus::PartialOutage => "partial_outage",
                    ServerLastStatus::Unknown => "unknown",
                }
            )
        }
    }

    impl From<Vec<bool>> for ServerLastStatus {
        fn from(v: Vec<bool>) -> Self {
            if v.is_empty() {
                return Self::Unknown;
            }
            if v.iter().all(|x| *x) {
                return Self::Optional;
            }
            if !v.iter().any(|x| *x) {
                return Self::Outage;
            }
            let answer = v.iter().filter(|x| **x == true).count();
            match v.len() {
                2 => Self::PartialOutage,
                n if n > 2 => {
                    let degraded_level = n as f32 / 3.0 * 2.0;
                    if answer as f32 / n as f32 >= degraded_level {
                        Self::DegradedPerformance
                    } else {
                        Self::PartialOutage
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}

pub use server_last_status::ServerLastStatus;

#[derive(Clone, Debug)]
pub struct PingAbleService {
    remote_address: String,
    service_type: ServiceType,
}

impl PingAbleService {
    pub fn remote_address(&self) -> &str {
        &self.remote_address
    }
    pub fn service_type(&self) -> ServiceType {
        self.service_type
    }

    pub async fn ping(service: PingAbleService, timeout: u64) -> bool {
        let ret = match service.service_type() {
            ServiceType::HTTP => HTTP::new(&service.remote_address()).ping(timeout).await,
            ServiceType::SSH => SSH::new(&service.remote_address()).ping(timeout).await,
            ServiceType::TeamSpeak => {
                TeamSpeak::new(&service.remote_address())
                    .ping(timeout)
                    .await
            }
            ServiceType::Tcping => Tcping::new(&service.remote_address()).ping(timeout).await,
            #[cfg(feature = "ping")]
            ServiceType::ICMP => ICMP::new(&service.remote_address()).ping(timeout).await,
        };
        match ret {
            Ok(ret) => ret,
            Err(e) if e.is::<tokio::time::error::Elapsed>() => false,
            Err(e) => {
                error!("Got error while ping {}: {:?}", service.remote_address(), e);
                false
            }
        }
    }
}

impl TryFrom<&Service> for PingAbleService {
    type Error = anyhow::Error;

    fn try_from(value: &Service) -> Result<Self, Self::Error> {
        let service_type = value.service_type().to_lowercase();
        let service_type = match service_type.as_str() {
            "teamspeak" | "ts" => ServiceType::TeamSpeak,
            "ssh" => ServiceType::SSH,
            "http" => ServiceType::HTTP,
            "tcping" => ServiceType::Tcping,
            #[cfg(feature = "ping")]
            "icmp" | "ping" => ServiceType::ICMP,
            &_ => {
                return Err(anyhow!(
                    "Unexpect service type: {}, address => {}",
                    value.service_type(),
                    value.address()
                ));
            }
        };
        Ok(Self {
            remote_address: value.address().to_string(),
            service_type,
        })
    }
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

#[derive(Clone, Debug)]
pub struct ServiceWrapper {
    last_status: ServerLastStatus,
    services: Vec<PingAbleService>,
    report_uuid: String,
    page: String,
    count: u64,
}

impl ServiceWrapper {
    pub fn report_uuid(&self) -> &str {
        &self.report_uuid
    }

    pub fn last_status(&self) -> &ServerLastStatus {
        &self.last_status
    }

    pub fn ongoing_recheck(&self) -> bool {
        self.count > 0
    }

    pub async fn ping(&self, timeout: u64) -> Vec<bool> {
        let mut v = Vec::new();
        let services = self.services.clone();
        for element in services
            .into_iter()
            .map(|x| tokio::spawn(PingAbleService::ping(x, timeout)))
        {
            v.push(element.await.unwrap())
        }
        v
    }

    pub fn update_last_status_condition(
        &mut self,
        last_status: ServerLastStatus,
        condition: u64,
    ) -> bool {
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

    pub async fn from_service(upstream: &Upstream, s: &Component) -> anyhow::Result<Self> {
        let status = upstream
            .get_component_status(s.report_uuid(), s.page())
            .await?;
        let status = status.json::<ComponentResponse>().await?;
        Self::new_with_last_status(s, ServerLastStatus::from(&ComponentStatus::from(&status)))
    }

    pub fn new_with_last_status(
        s: &Component,
        last_status: ServerLastStatus,
    ) -> anyhow::Result<Self> {
        let mut v = Vec::new();
        for service in s.addresses() {
            v.push(PingAbleService::try_from(service)?)
        }

        Ok(Self::new(
            v,
            last_status.clone(),
            s.report_uuid().to_string(),
            s.page().to_lowercase(),
        ))
    }

    pub fn new(
        services: Vec<PingAbleService>,
        last_status: ServerLastStatus,
        identify_id: String,
        page: String,
    ) -> Self {
        Self {
            last_status,
            services,
            report_uuid: identify_id,
            page,
            count: 0,
        }
    }

    pub fn page(&self) -> &str {
        &self.page
    }

    pub fn remote_address(&self) -> String {
        if !self.services.is_empty() {
            return self.services.get(0).unwrap().remote_address().to_string();
        }
        self.report_uuid.clone()
    }

    #[cfg(feature = "ping")]
    pub fn has_icmp_ping(&self) -> bool {
        self.services
            .iter()
            .any(|x| x.service_type() == ServiceType::ICMP)
    }
}

use crate::configure::{Component, Service};
use crate::connlib::tcping::Tcping;
use crate::statuspagelib::Upstream;
use crate::ComponentStatus;
use anyhow::anyhow;
pub use http::HTTP;
use serde_derive::Deserialize;
pub use ssh::SSH;
pub use teamspeak::TeamSpeak;

#[cfg(feature = "ping")]
use crate::connlib::icmp::ICMP;
#[cfg(any(feature = "env_logger", feature = "log4rs"))]
use log::error;
#[cfg(feature = "spdlog-rs")]
use spdlog::prelude::*;
