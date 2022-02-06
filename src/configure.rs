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
use crate::connlib::{ServerLastStatus, ServiceWrapper};
use crate::statuspagelib::Upstream;
use crate::ComponentStatus;
use serde_derive::Deserialize;
use spdlog::prelude::*;
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::Path;

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

    fn convert_cache_vec_to_map(cache: Option<CacheData>) -> HashMap<String, ComponentStatus> {
        let mut map: HashMap<String, ComponentStatus> = Default::default();
        if let Some(cache) = cache {
            for status in cache.data() {
                map.insert(
                    status.id().to_string(),
                    ComponentStatus::from(status.last_status()),
                );
            }
        }
        map
    }

    pub async fn try_from(value: TomlConfigure, cache: Option<CacheData>) -> anyhow::Result<Self> {
        let upstream = Upstream::from_configure(&value);
        let cache_data = Self::convert_cache_vec_to_map(cache);
        let mut result = vec![];
        for service in &value.services.0 {
            let service_w = if let Some(status) = cache_data.get(service.report_uuid()) {
                ServiceWrapper::new_with_last_status(service, ServerLastStatus::from(status))
            } else {
                ServiceWrapper::from_service(&upstream, service).await
            };
            if let Err(ref e) = service_w {
                error!(
                    "Got error while processing transform services: {} error: {:?}",
                    service.remote_address(),
                    e
                );
            }
            result.push(service_w.unwrap());
        }
        Ok(Self {
            services: result,
            upstream,
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
