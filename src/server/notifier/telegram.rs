use super::Notifier;
use crate::config::server::TelegramConfig;
use crate::model::ComponentStatus;
use reqwest::Client;
use std::time::Duration;

pub struct TelegramNotifier {
    client: Client,
    bot_token: String,
    chat_id: String,
}

impl TelegramNotifier {
    pub fn new(config: &TelegramConfig) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client"),
            bot_token: config.bot_token().to_string(),
            chat_id: config.chat_id().to_string(),
        }
    }

    fn format_message(
        &self,
        component_name: &str,
        old: ComponentStatus,
        new: ComponentStatus,
    ) -> String {
        let emoji = if new.is_healthy() { "✅" } else { "🔴" };
        format!("{emoji} *{component_name}*\nStatus changed: `{old}` → `{new}`")
    }
}

#[async_trait::async_trait]
impl Notifier for TelegramNotifier {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn notify(
        &self,
        _component_id: &str,
        component_name: &str,
        old: ComponentStatus,
        new: ComponentStatus,
    ) -> anyhow::Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);

        let text = self.format_message(component_name, old, new);

        self.client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": self.chat_id,
                "text": text,
                "parse_mode": "Markdown",
            }))
            .send()
            .await?;

        tracing::debug!("Telegram notification sent for {component_name}");
        Ok(())
    }
}
