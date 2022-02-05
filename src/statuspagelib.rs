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
            match s.status() {
                "operational" => ComponentStatus::Operational,
                "under_maintenance" => ComponentStatus::UnderMaintenance,
                "degraded_performance" => ComponentStatus::DegradedPerformance,
                "partial_outage" => ComponentStatus::PartialOutage,
                "major_outage" => ComponentStatus::MajorOutage,
                &_ => unreachable!("This code maybe outdated, if you sure this is wrong, please open a issue to report.")
            }
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

    #[derive(Debug, Clone)]
    pub struct Upstream {
        client: Client,
        page: String,
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
                page: cfg.upstream().page().to_string(),
            }
        }

        pub async fn set_component_status(
            &self,
            component: &str,
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
                .patch(self.build_request_url(component))
                .json(&payload)
                .send()
                .await?)
        }

        pub fn build_request_url(&self, component_id: &str) -> String {
            format!(
                "{basic_url}v1/pages/{page_id}/components/{component_id}",
                basic_url = UPSTREAM_URL,
                page_id = &self.page,
                component_id = component_id
            )
        }

        #[deprecated(since = "0.5.0")]
        pub async fn reset_component_status(&self, component: &str) -> anyhow::Result<Response> {
            self.set_component_status(component, ComponentStatus::Operational)
                .await
        }

        pub async fn get_component_status(&self, component: &str) -> anyhow::Result<Response> {
            Ok(self
                .client
                .get(self.build_request_url(component))
                .send()
                .await?)
        }
    }
}

pub use v1::ComponentStatus;
pub use v1::Upstream;
