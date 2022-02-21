/*
 ** Copyright (C) 2022 KunoiSayami
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

const UPSTREAM_URL: &str = "https://api.statuspage.io/";

mod v1 {
    use crate::configure::TomlConfigure;
    use crate::connlib::ComponentResponse;
    use crate::statuspagelib::UPSTREAM_URL;
    use crate::ServerLastStatus;
    use anyhow::anyhow;
    use reqwest::header::{HeaderMap, HeaderValue};
    use reqwest::{Client, Response};
    use serde_json::json;
    use std::fmt::Formatter;
    use std::time::Duration;

    #[allow(dead_code)]
    pub enum ComponentStatus {
        Operational,
        UnderMaintenance,
        DegradedPerformance,
        PartialOutage,
        MajorOutage,
    }

    impl From<&ComponentResponse> for ComponentStatus {
        fn from(s: &ComponentResponse) -> Self {
            match Self::try_from(s.status()) {
                Ok(s) => s,
                Err(_) => unreachable!("This code maybe outdated, if you sure this is wrong, please open a issue to report."),
            }
        }
    }

    impl TryFrom<&str> for ComponentStatus {
        type Error = anyhow::Error;

        fn try_from(value: &str) -> Result<Self, Self::Error> {
            Ok(match value {
                "operational" => ComponentStatus::Operational,
                "under_maintenance" => ComponentStatus::UnderMaintenance,
                "degraded_performance" => ComponentStatus::DegradedPerformance,
                "partial_outage" => ComponentStatus::PartialOutage,
                "major_outage" => ComponentStatus::MajorOutage,
                &_ => return Err(anyhow!("unexpected value: {}", value)),
            })
        }
    }

    impl From<bool> for ComponentStatus {
        fn from(b: bool) -> Self {
            if b {
                Self::Operational
            } else {
                Self::MajorOutage
            }
        }
    }

    impl std::fmt::Display for ComponentStatus {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    ComponentStatus::Operational => "operational",
                    ComponentStatus::UnderMaintenance => "under_maintenance",
                    ComponentStatus::DegradedPerformance => "degraded_performance",
                    ComponentStatus::PartialOutage => "partial_outage",
                    ComponentStatus::MajorOutage => "major_outage",
                }
            )
        }
    }

    impl From<&ServerLastStatus> for ComponentStatus {
        fn from(status: &ServerLastStatus) -> Self {
            match status {
                ServerLastStatus::Optional => ComponentStatus::Operational,
                ServerLastStatus::Outage => ComponentStatus::MajorOutage,
                ServerLastStatus::DegradedPerformance => ComponentStatus::DegradedPerformance,
                ServerLastStatus::PartialOutage => ComponentStatus::PartialOutage,
                ServerLastStatus::Unknown => unreachable!(),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct Upstream {
        client: Client,
    }

    impl Upstream {
        pub fn from_configure(cfg: &TomlConfigure) -> Upstream {
            let mut map = HeaderMap::new();
            map.insert(
                "Authorization",
                HeaderValue::from_str(cfg.upstream().oauth())
                    .expect("OAuth Header value parse error"),
            );
            Self {
                client: reqwest::ClientBuilder::new()
                    .default_headers(map.clone())
                    .timeout(Duration::from_secs(10))
                    .build()
                    .unwrap(),
            }
        }

        pub async fn set_component_status(
            &self,
            component: &str,
            page: &str,
            status: ComponentStatus,
        ) -> anyhow::Result<Response> {
            //let status = status.to_string();
            let payload = json!({
                "component": {
                    "status": status.to_string()
                }
            });
            Ok(self
                .client
                .patch(self.build_request_url(component, page))
                .json(&payload)
                .send()
                .await?)
        }

        pub fn build_request_url(&self, component_id: &str, page: &str) -> String {
            format!(
                "{basic_url}v1/pages/{page_id}/components/{component_id}",
                basic_url = UPSTREAM_URL,
                page_id = page,
                component_id = component_id
            )
        }

        pub async fn get_component_status(
            &self,
            component: &str,
            page: &str,
        ) -> anyhow::Result<Response> {
            Ok(self
                .client
                .get(self.build_request_url(component, page))
                .send()
                .await?)
        }
    }
}

pub use v1::ComponentStatus;
pub use v1::Upstream;
