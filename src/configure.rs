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

use crate::DEFAULT_DATABASE_LOCATION;
#[cfg(any(feature = "env_logger", feature = "log4rs"))]
use log::{error, warn};
use serde_derive::{Deserialize, Serialize};
#[cfg(feature = "spdlog-rs")]
use spdlog::prelude::*;
use std::fmt::Debug;
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    addr: String,
    port: u16,
    auth_header: Option<String>,
    public_status_page: bool,
    database_location: Option<String>,
}

impl ServerConfig {
    pub fn addr(&self) -> &str {
        &self.addr
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn auth_header(&self) -> String {
        match self.auth_header {
            None => String::new(),
            Some(ref auth) => auth.clone(),
        }
    }
    pub fn public_status_page(&self) -> bool {
        self.public_status_page
    }
    pub fn database_location(&self) -> String {
        match self.database_location {
            None => DEFAULT_DATABASE_LOCATION.to_string(),
            Some(ref location) => location.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Configure {
    statuspage: StatusPageUpstream,
    components: Components,
    server: ServerConfig,
}

impl Configure {
    pub async fn init_from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Configure> {
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

    pub fn statuspage(&self) -> &StatusPageUpstream {
        &self.statuspage
    }
    pub fn server(&self) -> &ServerConfig {
        &self.server
    }

    pub fn is_empty_services(&self) -> bool {
        self.components.0.is_empty()
    }
    pub fn components(&self) -> &Vec<Component> {
        &self.components.0
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StatusPageUpstream {
    enabled: bool,
    #[serde(default)]
    oauth: String,
}

impl StatusPageUpstream {
    pub fn oauth(&self) -> &str {
        &self.oauth
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Components(Vec<Component>);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Component {
    uuid: String,
    name: String,
    #[serde(default)]
    identity_id: String,
    #[serde(default)]
    page: String,
}

impl Component {
    pub fn report_id(&self) -> &str {
        &self.identity_id
    }

    pub fn page(&self) -> &str {
        &self.page
    }

    pub fn new(uuid: String, name: String, identity_id: String, page: String) -> Self {
        Self {
            uuid,
            name,
            identity_id,
            page,
        }
    }

    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn need_push(&self) -> bool {
        !self.identity_id.is_empty() && !self.page.is_empty()
    }
}
