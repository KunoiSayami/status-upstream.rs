use crate::statuspagelib::ComponentStatus;
use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Formatter;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TransferData {
    status: String,
}

impl TransferData {
    pub fn new(status: String) -> Self {
        Self { status }
    }

    pub fn not_found() -> Self {
        Self {
            status: "NOT_FOUND".to_string(),
        }
    }
    pub fn status(&self) -> &str {
        &self.status
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServerLastStatus {
    Optional,
    Outage,
    DegradedPerformance,
    PartialOutage,
    Unknown,
}

impl TryFrom<&String> for ServerLastStatus {
    type Error = anyhow::Error;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&str> for ServerLastStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "operational" => Self::Optional,
            "major_outage" => Self::Outage,
            "partial_outage" => Self::PartialOutage,
            "degraded_performance" => ServerLastStatus::DegradedPerformance,
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

#[async_trait]
pub trait UpstreamTrait: Send + Sync {
    #[deprecated]
    async fn get_component_status(&self, component: &str, page: &str) -> anyhow::Result<()>;

    async fn set_component_status(
        &self,
        component: &str,
        page: &str,
        status: ComponentStatus,
    ) -> anyhow::Result<()>;
}

#[derive(Copy, Clone, Debug, Default)]
pub struct EmptyUpstream {}

#[async_trait]
impl UpstreamTrait for EmptyUpstream {
    async fn get_component_status(&self, _component: &str, _page: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn set_component_status(
        &self,
        _component: &str,
        _page: &str,
        _status: ComponentStatus,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
