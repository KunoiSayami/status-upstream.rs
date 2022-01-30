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
    use crate::statuspagelib::UPSTREAM_URL;
    use reqwest::header::{HeaderMap, HeaderValue};
    use reqwest::Response;
    use serde_json::json;
    use std::fmt::Formatter;
    use std::time::Duration;

    pub enum ComponentStatus {
        Operational,
        UnderMaintenance,
        DegradedPerformance,
        PartialOutage,
        MajorOutage,
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
        page: String,
        headers: HeaderMap,
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
                page: "".to_string(),
                headers: map,
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
            let client = reqwest::ClientBuilder::new()
                .default_headers(self.headers.clone())
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap();
            Ok(client
                .patch(format!(
                    "{basic_url}v1/pages/{page_id}/components/{component_id}",
                    basic_url = UPSTREAM_URL,
                    page_id = &self.page,
                    component_id = component
                ))
                .json(&payload)
                .send()
                .await?)
        }

        pub async fn reset_component_status(&self, component: &str) -> anyhow::Result<Response> {
            self.set_component_status(component, ComponentStatus::Operational)
                .await
        }
    }
}

pub use v1::ComponentStatus;
pub use v1::Upstream;
