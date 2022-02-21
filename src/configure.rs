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

use crate::cache::CacheData;
use crate::connlib::{PingAbleService, ServerLastStatus, ServiceWrapper};
use crate::statuspagelib::Upstream;
use anyhow::anyhow;
#[cfg(any(feature = "env_logger", feature = "log4rs"))]
use log::{error, warn};
use serde_derive::{Deserialize, Serialize};
#[cfg(feature = "spdlog-rs")]
use spdlog::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::path::Path;
use toml::Value;

#[derive(Clone, Debug)]
pub struct Configure {
    services: Vec<ServiceWrapper>,
    upstream: Upstream,
}

impl Configure {
    pub fn mut_services(&mut self) -> &mut Vec<ServiceWrapper> {
        &mut self.services
    }

    pub fn services(&self) -> &Vec<ServiceWrapper> {
        &self.services
    }

    pub fn upstream(&self) -> &Upstream {
        &self.upstream
    }

    fn convert_cache_vec_to_map(cache: Option<CacheData>) -> HashMap<String, ServerLastStatus> {
        let mut map: HashMap<String, ServerLastStatus> = Default::default();
        if let Some(cache) = cache {
            for status in cache.data() {
                map.insert(
                    status.id().to_string(),
                    ServerLastStatus::try_from(status.last_status()).unwrap(),
                );
            }
        }
        map
    }

    pub async fn try_from(value: TomlConfigure, cache: Option<CacheData>) -> anyhow::Result<Self> {
        let upstream = Upstream::from_configure(&value);
        let cache_data = Self::convert_cache_vec_to_map(cache);
        let mut result = vec![];
        for service in value.services.0 {
            let component: Component = service.try_into()?;
            let service_w = if let Some(status) = cache_data.get(component.report_uuid()) {
                ServiceWrapper::new_with_last_status(&component, *status)
            } else {
                ServiceWrapper::from_service(&upstream, &component).await
            };
            if let Err(ref e) = service_w {
                error!(
                    "Got error while processing transform services: {:?} error: {:?}",
                    &component, e
                );
            }
            result.push(service_w.unwrap());
        }

        // Check duplicate component_id
        let mut id_checker = HashSet::new();
        for service in &result {
            id_checker.insert(service.report_uuid());
        }
        if id_checker.len() != result.len() {
            warn!("Duplicate component_id detected");
        }

        Ok(Self {
            services: result,
            upstream,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TomlConfigure {
    upstream: TomlUpstream,
    services: Components,
    config: ServerConfig,
}

impl TomlConfigure {
    pub async fn init_from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<TomlConfigure> {
        let context = tokio::fs::read_to_string(&path).await;
        if let Err(ref e) = context {
            error!(
                "Got error {:?} while reading {:?}",
                e,
                &path.as_ref().display()
            );
        }
        let context = context?;
        let cfg = match toml::from_str(context.as_str()) {
            Ok(cfg) => cfg,
            Err(e) => {
                error!(
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
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    pub fn is_empty_services(&self) -> bool {
        self.services.0.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TomlUpstream {
    oauth: String,
}

impl TomlUpstream {
    pub fn oauth(&self) -> &str {
        &self.oauth
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    interval: Option<u64>,
    retries_times: Option<u64>,
    retries_interval: Option<u64>,
}

impl ServerConfig {
    pub fn interval(&self) -> &Option<u64> {
        &self.interval
    }
    pub fn retries_times(&self) -> Option<u64> {
        self.retries_times
    }
    pub fn retries_interval(&self) -> Option<u64> {
        self.retries_interval
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Components(Vec<TomlComponent>);

#[derive(Clone, Debug)]
pub struct Service {
    address: String,
    service_type: String,
}

impl Service {
    pub fn address(&self) -> &str {
        &self.address
    }
    pub fn service_type(&self) -> &str {
        &self.service_type
    }
}

impl TryInto<PingAbleService> for Service {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<PingAbleService, Self::Error> {
        PingAbleService::try_from(&self)
    }
}

impl TryFrom<&str> for Service {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if !value.contains('|') {
            return Err(anyhow!("FormatError: missing '|' (raw: {})", value));
        }
        let (address, service_type) = value.split_once('|').unwrap();
        Ok(Self {
            address: address.to_string(),
            service_type: service_type.to_string(),
        })
    }
}

impl TryFrom<&String> for Service {
    type Error = anyhow::Error;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

#[derive(Clone, Debug)]
pub struct Component {
    addresses: Vec<Service>,
    identity_id: String,
    page: String,
}

impl Component {
    pub fn addresses(&self) -> &Vec<Service> {
        &self.addresses
    }
    pub fn report_uuid(&self) -> &str {
        &self.identity_id
    }
    pub fn page(&self) -> &str {
        &self.page
    }
    pub fn new(addresses: Vec<Service>, identity_id: String, page: String) -> Self {
        Component {
            addresses,
            identity_id,
            page,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TomlComponent {
    addresses: toml::Value,
    identity_id: String,
    page: String,
}

impl TomlComponent {
    pub fn try_get_services(&self) -> anyhow::Result<Vec<Service>> {
        self.clone().try_into()
    }
}

impl TryInto<Component> for TomlComponent {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Component, Self::Error> {
        Ok(Component::new(
            self.try_get_services()?,
            self.identity_id,
            self.page,
        ))
    }
}

impl TryInto<Vec<Service>> for TomlComponent {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Vec<Service>, Self::Error> {
        let mut v = Vec::default();

        match self.addresses {
            Value::String(s) => {
                v.push(Service::try_from(&s)?);
                Ok(v)
            }
            Value::Array(array) => {
                for element in array {
                    match element {
                        Value::String(s) => v.push(Service::try_from(&s)?),
                        _ => return Err(anyhow!("Unexpected value inside address array.")),
                    }
                }
                Ok(v)
            }
            _ => Err(anyhow!("Unexpected value in addresses field.")),
        }
    }
}
