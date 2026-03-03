pub mod command;
pub mod http;
#[cfg(feature = "ping")]
pub mod ping;
pub mod ssh;
pub mod subnet;
pub mod tcp;
pub mod teamspeak;

use crate::config::client::CheckConfig;
use crate::model::CheckReport;

#[async_trait::async_trait]
pub trait Check: Send + Sync {
    fn name(&self) -> &str;
    fn component_id(&self) -> &str;
    async fn execute(&self) -> CheckReport;
}

/// Create a `Box<dyn Check>` from a `CheckConfig`.
pub fn from_config(config: &CheckConfig) -> Box<dyn Check> {
    match config {
        CheckConfig::Command {
            component_id,
            command,
            timeout,
        } => Box::new(command::CommandCheck::new(
            component_id.clone(),
            command.clone(),
            *timeout,
        )),
        CheckConfig::Tcp {
            component_id,
            host,
            port,
            timeout,
        } => Box::new(tcp::TcpCheck::new(
            component_id.clone(),
            host.clone(),
            *port,
            *timeout,
        )),
        CheckConfig::Http {
            component_id,
            url,
            expected_status,
            timeout,
        } => Box::new(http::HttpCheck::new(
            component_id.clone(),
            url.clone(),
            *expected_status,
            *timeout,
        )),
        CheckConfig::Ssh {
            component_id,
            host,
            port,
            timeout,
        } => Box::new(ssh::SshCheck::new(
            component_id.clone(),
            host.clone(),
            *port,
            *timeout,
        )),
        CheckConfig::Teamspeak {
            component_id,
            host,
            port,
            timeout,
        } => Box::new(teamspeak::TeamSpeakCheck::new(
            component_id.clone(),
            host.clone(),
            *port,
            *timeout,
        )),
        #[cfg(feature = "ping")]
        CheckConfig::Ping {
            component_id,
            host,
            timeout,
        } => Box::new(ping::PingCheck::new(
            component_id.clone(),
            host.clone(),
            *timeout,
        )),
        CheckConfig::Subnet {
            component_id,
            network,
            port,
            timeout,
        } => Box::new(subnet::SubnetCheck::new(
            component_id.clone(),
            network.clone(),
            *port,
            *timeout,
        )),
    }
}
