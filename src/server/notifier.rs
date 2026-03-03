pub mod statuspage;
pub mod telegram;

use crate::model::ComponentStatus;

#[async_trait::async_trait]
pub trait Notifier: Send + Sync {
    fn name(&self) -> &str;

    async fn notify(
        &self,
        component_id: &str,
        component_name: &str,
        old: ComponentStatus,
        new: ComponentStatus,
    ) -> anyhow::Result<()>;
}

pub struct NotifierRegistry {
    notifiers: Vec<Box<dyn Notifier>>,
}

impl NotifierRegistry {
    pub fn new() -> Self {
        Self {
            notifiers: Vec::new(),
        }
    }

    pub fn register(&mut self, notifier: Box<dyn Notifier>) {
        tracing::info!("Registered notifier: {}", notifier.name());
        self.notifiers.push(notifier);
    }

    pub async fn notify_all(
        &self,
        component_id: &str,
        component_name: &str,
        old: ComponentStatus,
        new: ComponentStatus,
    ) {
        for notifier in &self.notifiers {
            if let Err(e) = notifier
                .notify(component_id, component_name, old, new)
                .await
            {
                tracing::error!("Notifier '{}' failed: {e}", notifier.name());
            }
        }
    }
}
