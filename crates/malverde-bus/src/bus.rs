use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use async_trait::async_trait;
use malverde_core::error::MalverdeError;

use super::event::Event;
use super::handler::EventHandler;

#[derive(Debug, Clone)]
pub struct EventBus {
    handlers: Arc<RwLock<HashMap<String, Vec<Arc<dyn EventHandler + Send + Sync>>>>>,
    sender: tokio::sync::mpsc::UnboundedSender<Arc<dyn Event + Send + Sync>>,
    receiver: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Arc<dyn Event + Send + Sync>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }
    
    pub async fn subscribe<E, H>(&self, handler: Arc<H>) -> Result<(), MalverdeError>
    where
        E: Event + 'static,
        H: EventHandler<Event = E> + 'static,
    {
        let type_name = std::any::type_name::<E>().to_string();
        let mut handlers = self.handlers.write().await;
        handlers
            .entry(type_name)
            .or_insert_with(Vec::new)
            .push(handler as Arc<dyn EventHandler + Send + Sync>);
        Ok(())
    }
    
    pub async fn publish<E>(&self, event: E) -> Result<(), MalverdeError>
    where
        E: Event + Send + Sync + 'static,
    {
        let event_arc = Arc::new(event);
        self.sender
            .send(event_arc.clone())
            .map_err(|e| MalverdeError::EventError(e.to_string()))?;
        Ok(())
    }
    
    pub async fn start(&self) -> Result<(), MalverdeError> {
        let receiver = self.receiver.clone();
        let handlers = self.handlers.clone();
        
        tokio::spawn(async move {
            let mut receiver = receiver.lock().await;
            while let Some(event) = receiver.recv().await {
                let handlers = handlers.read().await;
                let type_name = std::any::type_name::<<dyn Event>>(&*event);
                
                if let Some(event_handlers) = handlers.get(type_name) {
                    for handler in event_handlers {
                        if let Err(e) = handler.handle(event.clone()).await {
                            tracing::error!("Error handling event: {}", e);
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}