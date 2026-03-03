use super::Check;
use crate::model::{CheckReport, ComponentStatus};
use std::time::{Duration, Instant};

pub struct CommandCheck {
    component_id: String,
    command: String,
    timeout: u64,
}

impl CommandCheck {
    pub fn new(component_id: String, command: String, timeout: u64) -> Self {
        Self {
            component_id,
            command,
            timeout,
        }
    }
}

#[async_trait::async_trait]
impl Check for CommandCheck {
    fn name(&self) -> &str {
        &self.command
    }

    fn component_id(&self) -> &str {
        &self.component_id
    }

    async fn execute(&self) -> CheckReport {
        let start = Instant::now();

        let result = tokio::time::timeout(
            Duration::from_secs(self.timeout),
            run_command(&self.command),
        )
        .await;

        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok((code, stderr))) => {
                let status = ComponentStatus::from_exit_code(code);
                let message = if stderr.is_empty() {
                    None
                } else {
                    Some(stderr)
                };
                CheckReport::new(self.component_id.clone(), status, message, Some(latency))
            }
            Ok(Err(e)) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::MajorOutage,
                Some(format!("Failed to run command: {e}")),
                Some(latency),
            ),
            Err(_) => CheckReport::new(
                self.component_id.clone(),
                ComponentStatus::MajorOutage,
                Some("Command timed out".to_string()),
                Some(latency),
            ),
        }
    }
}

async fn run_command(command: &str) -> anyhow::Result<(i32, String)> {
    let output = if cfg!(target_os = "windows") {
        tokio::process::Command::new("cmd")
            .args(["/C", command])
            .output()
            .await?
    } else {
        tokio::process::Command::new("sh")
            .args(["-c", command])
            .output()
            .await?
    };

    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Ok((code, stderr))
}
