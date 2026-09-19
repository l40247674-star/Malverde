use async_trait::async_trait;
use std::sync::Arc;

use malverde_core::error::MalverdeError;

use super::event::Event;

#[async_trait]
pub trait EventHandler: Send + Sync {
    type Event: Event + 'static;
    async fn handle(&self, event: Arc<Self::Event>) -> Result<(), MalverdeError>;
    fn name(&self) -> &str;
}

pub struct LogHandler;

#[async_trait]
impl EventHandler for LogHandler {
    type Event = GenericEvent;
    async fn handle(&self, event: Arc<GenericEvent>) -> Result<(), MalverdeError> {
        tracing::info!(
            "Event received: type={:?}, id={}",
            event.event_type(),
            event.id()
        );
        Ok(())
    }
    fn name(&self) -> &str {
        "LogHandler"
    }
}