use serde::Deserialize;

const DEFAULT_CHECK_INTERVAL: u64 = 30;
const DEFAULT_TIMEOUT: u64 = 10;

#[derive(Debug, Clone, Deserialize)]
pub struct ClientRootConfig {
    client: ClientConfig,
    #[serde(default)]
    checks: Vec<CheckConfig>,
}

impl ClientRootConfig {
    pub fn client(&self) -> &ClientConfig {
        &self.client
    }

    pub fn checks(&self) -> &[CheckConfig] {
        &self.checks
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClientConfig {
    server_url: String,
    #[serde(default)]
    auth_token: String,
    #[serde(default = "default_client_id")]
    client_id: String,
    #[serde(default = "default_check_interval")]
    check_interval: u64,
}

impl ClientConfig {
    pub fn server_url(&self) -> &str {
        &self.server_url
    }

    pub fn auth_token(&self) -> &str {
        &self.auth_token
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn check_interval(&self) -> u64 {
        self.check_interval
    }
}

fn default_client_id() -> String {
    "unknown-client".to_string()
}

fn default_check_interval() -> u64 {
    DEFAULT_CHECK_INTERVAL
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckConfig {
    Command {
        component_id: String,
        command: String,
        #[serde(default = "default_timeout")]
        timeout: u64,
    },
    Tcp {
        component_id: String,
        host: String,
        port: u16,
        #[serde(default = "default_timeout")]
        timeout: u64,
    },
    Http {
        component_id: String,
        url: String,
        #[serde(default = "default_expected_status")]
        expected_status: u16,
        #[serde(default = "default_timeout")]
        timeout: u64,
    },
    Ssh {
        component_id: String,
        host: String,
        #[serde(default = "default_ssh_port")]
        port: u16,
        #[serde(default = "default_timeout")]
        timeout: u64,
    },
    Teamspeak {
        component_id: String,
        host: String,
        #[serde(default = "default_ts_port")]
        port: u16,
        #[serde(default = "default_timeout")]
        timeout: u64,
    },
    #[cfg(feature = "ping")]
    Ping {
        component_id: String,
        host: String,
        #[serde(default = "default_timeout")]
        timeout: u64,
    },
    Subnet {
        component_id: String,
        network: String,
        port: u16,
        #[serde(default = "default_timeout")]
        timeout: u64,
    },
}

impl CheckConfig {
    pub fn component_id(&self) -> &str {
        match self {
            Self::Command { component_id, .. }
            | Self::Tcp { component_id, .. }
            | Self::Http { component_id, .. }
            | Self::Ssh { component_id, .. }
            | Self::Teamspeak { component_id, .. }
            | Self::Subnet { component_id, .. } => component_id,
            #[cfg(feature = "ping")]
            Self::Ping { component_id, .. } => component_id,
        }
    }
}

fn default_expected_status() -> u16 {
    200
}

fn default_ssh_port() -> u16 {
    22
}

fn default_ts_port() -> u16 {
    9987
}
