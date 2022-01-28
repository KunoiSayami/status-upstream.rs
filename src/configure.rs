/*
 ** Copyright (C) 2021 KunoiSayami
 **
 ** This file is part of status-upstream.rs and is released under
 ** the AGPL v3 License: https://www.gnu.org/licenses/agpl-3.0.txt
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

use std::convert::TryFrom;
use crate::connlib::{HTTP, ServiceChecker, ServiceType, SSH, TeamSpeak};
use serde_derive::Deserialize;
use log::error;

#[derive(Deserialize, Debug, Clone)]
pub struct Configure {
    services: Services
}

#[derive(Deserialize, Debug, Clone)]
impl Configure {

}


#[derive(Deserialize, Debug, Clone)]
struct Upstream {
    server: String,
    token: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Services(Vec<Service>);

#[derive(Deserialize, Debug, Clone)]
pub struct Service {
    address: String,
    uuid: String,
    #[serde(rename = "type")]
    service_type: String,
}

impl Service {
    pub fn remote_address(&self) -> &str {
        &self.address
    }
    pub fn report_uuid(&self) -> &str {
        &self.uuid
    }
    pub fn service_type(&self) -> &str {
        &self.service_type
    }
}

pub struct BoxService {
    report_uuid: String,
    service_type: ServiceType,
    inner: Box<dyn ServiceChecker>,
}

impl TryFrom<&Service> for BoxService {
    type Error = ();

    fn try_from(s: &Service) -> Result<Self, Self::Error> {
        let service_type = s.service_type().to_lowercase();
        let service_type = match service_type.as_str() {
            "teamspeak" | "ts" => ServiceType::TeamSpeak,
            "ssh" => ServiceType::SSH,
            "http" => ServiceType::HTTP,
            &_ => {
                error!("Unexpect service type: {}, report uuid => {}", s.service_type(), s.report_uuid());
                return Err(());
            }
        };
        let inner: Box<dyn ServiceChecker> = match service_type {
            ServiceType::HTTP => Box::new(HTTP::new(s.remote_address())),
            ServiceType::SSH => Box::new(SSH::new(s.remote_address())),
            ServiceType::TeamSpeak => Box::new(TeamSpeak::new(s.remote_address())),
        };

        Ok(Self{
            report_uuid: s.report_uuid().to_string(),
            service_type,
            inner,
        })

    }
}