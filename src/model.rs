
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::SystemTime;

/// Unified component status used across the entire application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStatus {
    Operational,
    DegradedPerformance,
    PartialOutage,
    MajorOutage,
    UnderMaintenance,
    Unknown,
}

impl ComponentStatus {
    pub fn from_exit_code(code: i32) -> Self {
        if code == 0 {
            Self::Operational
        } else {
            Self::MajorOutage
        }
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Operational)
    }
}

impl fmt::Display for ComponentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Operational => "operational",
            Self::DegradedPerformance => "degraded_performance",
            Self::PartialOutage => "partial_outage",
            Self::MajorOutage => "major_outage",
            Self::UnderMaintenance => "under_maintenance",
            Self::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

/// Derives an aggregate `ComponentStatus` from a list of individual check results.
///
/// Ported from the old connlib `ServerLastStatus::from(Vec<bool>)` logic:
/// - All pass → Operational
/// - None pass → MajorOutage
/// - Partial → DegradedPerformance or PartialOutage based on success ratio
impl From<&[bool]> for ComponentStatus {
    fn from(results: &[bool]) -> Self {
        if results.is_empty() {
            return Self::Unknown;
        }
        let total = results.len();
        let passed = results.iter().filter(|&&ok| ok).count();

        if passed == total {
            return Self::Operational;
        }
        if passed == 0 {
            return Self::MajorOutage;
        }

        // With 2 services, any failure is a partial outage.
        // With 3+, use 2/3 threshold: above → degraded, below → partial outage.
        if total <= 2 {
            Self::PartialOutage
        } else {
            let ratio = passed as f64 / total as f64;
            if ratio >= 2.0 / 3.0 {
                Self::DegradedPerformance
            } else {
                Self::PartialOutage
            }
        }
    }
}

/// A single check result reported from client to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckReport {
    component_id: String,
    status: ComponentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u64>,
    timestamp: u64,
}

impl CheckReport {
    pub fn new(
        component_id: String,
        status: ComponentStatus,
        message: Option<String>,
        latency_ms: Option<u64>,
    ) -> Self {
        Self {
            component_id,
            status,
            message,
            latency_ms,
            timestamp: current_timestamp(),
        }
    }

    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    pub fn status(&self) -> ComponentStatus {
        self.status
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn latency_ms(&self) -> Option<u64> {
        self.latency_ms
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

/// Batch report sent from a client to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientReport {
    client_id: String,
    checks: Vec<CheckReport>,
}

impl ClientReport {
    pub fn new(client_id: String, checks: Vec<CheckReport>) -> Self {
        Self { client_id, checks }
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn checks(&self) -> &[CheckReport] {
        &self.checks
    }
}

/// Response to a client report submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportResponse {
    accepted: usize,
    status_changes: Vec<String>,
}

impl ReportResponse {
    pub fn new(accepted: usize, status_changes: Vec<String>) -> Self {
        Self {
            accepted,
            status_changes,
        }
    }
}

/// Component info returned by status query endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfo {
    id: String,
    name: String,
    current_status: ComponentStatus,
    last_updated: u64,
}

impl ComponentInfo {
    pub fn new(
        id: String,
        name: String,
        current_status: ComponentStatus,
        last_updated: u64,
    ) -> Self {
        Self {
            id,
            name,
            current_status,
            last_updated,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn current_status(&self) -> ComponentStatus {
        self.current_status
    }

    pub fn last_updated(&self) -> u64 {
        self.last_updated
    }
}

/// A single history entry for a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    status: ComponentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reported_by: Option<String>,
    created_at: u64,
}

impl HistoryEntry {
    pub fn new(
        status: ComponentStatus,
        message: Option<String>,
        latency_ms: Option<u64>,
        reported_by: Option<String>,
        created_at: u64,
    ) -> Self {
        Self {
            status,
            message,
            latency_ms,
            reported_by,
            created_at,
        }
    }
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}
