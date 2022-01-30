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

use crate::connlib::{ServiceChecker, ServiceType, TeamSpeak, HTTP, SSH};
use crate::statuspagelib::Upstream;
use anyhow::anyhow;
use serde_derive::Deserialize;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::path::Path;

pub struct Configure {
    services: Vec<BoxService>,
    upstream: Upstream,
}

impl Configure {
    pub fn services(&self) -> &Vec<BoxService> {
        &self.services
    }
    pub fn upstream(&self) -> &Upstream {
        &self.upstream
    }
}

impl TryFrom<TomlConfigure> for Configure {
    type Error = anyhow::Error;

    fn try_from(value: TomlConfigure) -> Result<Self, Self::Error> {
        let result = value
            .services
            .0
            .into_iter()
            .map(|ref x| {
                let service = BoxService::try_from(x);
                if let Err(ref e) = service {
                    log::error!(
                        "Got error while processing transform services: {:?} error: {:?}",
                        x,
                        e
                    );
                }
                service.unwrap()
            })
            .collect::<Vec<BoxService>>();
        Ok(Self {
            services: result,
            upstream: Upstream::from_configure(&value),
        })
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TomlConfigure {
    upstream: TomlUpstream,
    services: Services,
    config: ServerConfig,
}

impl TomlConfigure {
    pub async fn init_from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<TomlConfigure> {
        let context = tokio::fs::read_to_string(&path).await;
        if let Err(ref e) = context {
            log::error!(
                "Got error {:?} while reading {:?}",
                e,
                &path.as_ref().display()
            );
        }
        let context = context?;
        let cfg = match toml::from_str(context.as_str()) {
            Ok(cfg) => cfg,
            Err(e) => {
                log::error!(
                    "Got error {:?} while decode toml {:?}",
                    e,
                    path.as_ref().display()
                );
                return Err(anyhow::Error::from(e));
            }
        };
        Ok(cfg)
    }
    pub fn upstream(&self) -> &TomlUpstream {
        &self.upstream
    }
    pub fn services(&self) -> &Services {
        &self.services
    }
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TomlUpstream {
    page: String,
    oauth: String,
}

impl TomlUpstream {
    pub fn page(&self) -> &str {
        &self.page
    }
    pub fn oauth(&self) -> &str {
        &self.oauth
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ServerConfig {
    interval: Option<u64>,
}

impl ServerConfig {
    pub fn interval(&self) -> &Option<u64> {
        &self.interval
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Services(Vec<Service>);

#[derive(Deserialize, Debug, Clone)]
pub struct Service {
    address: String,
    identity_id: String,
    #[serde(rename = "type")]
    service_type: String,
}

impl Service {
    pub fn remote_address(&self) -> &str {
        &self.address
    }
    pub fn report_uuid(&self) -> &str {
        &self.identity_id
    }
    pub fn service_type(&self) -> &str {
        &self.service_type
    }
}

pub struct BoxService {
    report_uuid: String,
    service_type: ServiceType,
    inner: Box<dyn ServiceChecker + Send + Sync>,
}

impl BoxService {
    pub fn report_uuid(&self) -> &str {
        &self.report_uuid
    }
    pub fn service_type(&self) -> &ServiceType {
        &self.service_type
    }
    pub fn inner(&self) -> &Box<dyn ServiceChecker + Send + Sync> {
        &self.inner
    }
}

impl TryFrom<&Service> for BoxService {
    type Error = anyhow::Error;

    fn try_from(s: &Service) -> Result<Self, Self::Error> {
        let service_type = s.service_type().to_lowercase();
        let service_type = match service_type.as_str() {
            "teamspeak" | "ts" => ServiceType::TeamSpeak,
            "ssh" => ServiceType::SSH,
            "http" => ServiceType::HTTP,
            &_ => {
                return Err(anyhow!(
                    "Unexpect service type: {}, report uuid => {}",
                    s.service_type(),
                    s.report_uuid()
                ));
            }
        };
        let inner: Box<dyn ServiceChecker> = match service_type {
            ServiceType::HTTP => Box::new(HTTP::new(s.remote_address())),
            ServiceType::SSH => Box::new(SSH::new(s.remote_address())),
            ServiceType::TeamSpeak => Box::new(TeamSpeak::new(s.remote_address())),
        };

        Ok(Self {
            report_uuid: s.report_uuid().to_string(),
            service_type,
            inner,
        })
    }
}
