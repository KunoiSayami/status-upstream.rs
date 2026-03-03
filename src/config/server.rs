use serde::Deserialize;
use std::collections::HashMap;

const DEFAULT_BIND: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 41132;
const DEFAULT_DATABASE: &str = "status-upstream.db";
const DEFAULT_CHECK_INTERVAL: u64 = 60;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerRootConfig {
    server: ServerConfig,
    #[serde(default)]
    components: Vec<ComponentDef>,
    #[serde(default)]
    local_checks: Vec<crate::config::client::CheckConfig>,
    #[serde(default)]
    notifiers: NotifierConfigs,
}

impl ServerRootConfig {
    pub fn server(&self) -> &ServerConfig {
        &self.server
    }

    pub fn components(&self) -> &[ComponentDef] {
        &self.components
    }

    pub fn local_checks(&self) -> &[crate::config::client::CheckConfig] {
        &self.local_checks
    }

    pub fn notifiers(&self) -> &NotifierConfigs {
        &self.notifiers
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    bind: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_database")]
    database: String,
    #[serde(default)]
    auth_token: String,
    #[serde(default = "default_check_interval")]
    check_interval: u64,
    #[serde(default)]
    public_status_page: bool,
}

impl ServerConfig {
    pub fn bind(&self) -> &str {
        &self.bind
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn database(&self) -> &str {
        &self.database
    }

    pub fn auth_token(&self) -> &str {
        &self.auth_token
    }

    pub fn check_interval(&self) -> u64 {
        self.check_interval
    }

    pub fn public_status_page(&self) -> bool {
        self.public_status_page
    }
}

fn default_bind() -> String {
    DEFAULT_BIND.to_string()
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

fn default_database() -> String {
    DEFAULT_DATABASE.to_string()
}

fn default_check_interval() -> u64 {
    DEFAULT_CHECK_INTERVAL
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentDef {
    id: String,
    name: String,
}

impl ComponentDef {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct NotifierConfigs {
    #[serde(default)]
    statuspage: Option<StatusPageConfig>,
    #[serde(default)]
    telegram: Option<TelegramConfig>,
}

impl NotifierConfigs {
    pub fn statuspage(&self) -> Option<&StatusPageConfig> {
        self.statuspage.as_ref()
    }

    pub fn telegram(&self) -> Option<&TelegramConfig> {
        self.telegram.as_ref()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatusPageConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    components: HashMap<String, StatusPageComponentMapping>,
}

impl StatusPageConfig {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn components(&self) -> &HashMap<String, StatusPageComponentMapping> {
        &self.components
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatusPageComponentMapping {
    page_id: String,
    component_id: String,
}

impl StatusPageComponentMapping {
    pub fn page_id(&self) -> &str {
        &self.page_id
    }

    pub fn component_id(&self) -> &str {
        &self.component_id
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    bot_token: String,
    #[serde(default)]
    chat_id: String,
}

impl TelegramConfig {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn bot_token(&self) -> &str {
        &self.bot_token
    }

    pub fn chat_id(&self) -> &str {
        &self.chat_id
    }
}
