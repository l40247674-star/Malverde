use crate::event::{Event, EventType, EventState, EventPayload};
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::{mpsc, Mutex};
use dashmap::DashMap;

/// Event bus for publish-subscribe pattern
#[derive(Debug, Clone)]
pub struct EventBus {
    subscribers: Arc<DashMap<EventType, Vec<Arc<dyn EventSubscriber + Send + Sync>>>>,
    global_subscribers: Arc<DashMap<String, Arc<dyn EventSubscriber + Send + Sync>>>>,
    event_buffer: Arc<Mutex<Vec<Event>>>,
    buffer_size: usize,
    event_sender: Option<mpsc::Sender<Event>>,
    event_receiver: Option<Mutex<mpsc::Receiver<Event>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(DashMap::new()),
            global_subscribers: Arc::new(DashMap::new()),
            event_buffer: Arc::new(Mutex::new(Vec::new())),
            buffer_size: 1000,
            event_sender: None,
            event_receiver: None,
        }
    }

    pub fn with_buffer_size(buffer_size: usize) -> Self {
        Self { buffer_size, ..Self::new() }
    }

    pub fn with_async_channel(buffer_size: usize) -> Self {
        let (sender, receiver) = mpsc::channel(buffer_size);
        Self {
            buffer_size,
            event_sender: Some(sender),
            event_receiver: Some(Mutex::new(receiver)),
            ..Self::new()
        }
    }

    pub fn subscribe(&self, subscriber: Arc<dyn EventSubscriber + Send + Sync>, event_types: Vec<EventType>) {
        for event_type in event_types {
            let mut subs = self.subscribers.entry(event_type).or_default();
            if !subs.contains(&subscriber) {
                subs.push(subscriber.clone());
            }
        }
    }

    pub fn subscribe_global(&self, subscriber: Arc<dyn EventSubscriber + Send + Sync>, id: String) {
        self.global_subscribers.insert(id, subscriber);
    }

    pub fn unsubscribe(&self, subscriber: &Arc<dyn EventSubscriber + Send + Sync>, event_types: Vec<EventType>) {
        for event_type in event_types {
            if let Some(mut subs) = self.subscribers.get_mut(&event_type) {
                subs.retain(|s| !Arc::ptr_eq(s, subscriber));
            }
        }
    }

    pub fn unsubscribe_global(&self, id: &str) {
        self.global_subscribers.remove(id);
    }

    pub async fn publish(&self, event: Event) {
        self.add_to_buffer(event.clone()).await;
        self.notify_subscribers(event.clone()).await;
        
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(event.clone()).await;
        }
    }

    pub async fn publish_batch(&self, events: Vec<Event>) {
        for event in events {
            self.publish(event).await;
        }
    }

    async fn add_to_buffer(&self, event: Event) {
        let mut buffer = self.event_buffer.lock().await;
        if buffer.len() >= self.buffer_size {
            buffer.remove(0);
        }
        buffer.push(event);
    }

    async fn notify_subscribers(&self, event: Event) {
        if let Some(subs) = self.subscribers.get(&event.event_type) {
            for subscriber in subs.value().iter() {
                let subscriber = subscriber.clone();
                let event = event.clone();
                tokio::spawn(async move {
                    let _ = subscriber.on_event(event).await;
                });
            }
        }
        
        for subscriber in self.global_subscribers.iter() {
            let subscriber = subscriber.value().clone();
            let event = event.clone();
            tokio::spawn(async move {
                let _ = subscriber.on_event(event).await;
            });
        }
    }

    pub async fn get_buffered_events(&self) -> Vec<Event> {
        let buffer = self.event_buffer.lock().await;
        buffer.clone()
    }

    pub async fn get_events_by_type(&self, event_type: EventType) -> Vec<Event> {
        let buffer = self.event_buffer.lock().await;
        buffer.iter().filter(|e| e.event_type == event_type).cloned().collect()
    }

    pub async fn get_pending_events(&self) -> Vec<Event> {
        let buffer = self.event_buffer.lock().await;
        buffer.iter().filter(|e| e.state == EventState::Recorded).cloned().collect()
    }

    pub async fn clear_buffer(&self) {
        let mut buffer = self.event_buffer.lock().await;
        buffer.clear();
    }

    pub fn subscriber_count(&self, event_type: &EventType) -> usize {
        self.subscribers.get(event_type).map(|s| s.value().len()).unwrap_or(0)
    }

    pub fn total_subscribers(&self) -> usize {
        let global = self.global_subscribers.len();
        let typed = self.subscribers.iter().map(|s| s.value().len()).sum::<usize>();
        global + typed
    }

    pub fn scoped(&self, project_id: Option<malverde_core::ProjectId>, operation_id: Option<malverde_core::OperationId>) -> ScopedEventBus {
        ScopedEventBus { bus: self.clone(), project_id, operation_id }
    }
}

/// Scoped event bus
#[derive(Debug, Clone)]
pub struct ScopedEventBus {
    bus: EventBus,
    project_id: Option<malverde_core::ProjectId>,
    operation_id: Option<malverde_core::OperationId>,
}

impl ScopedEventBus {
    pub async fn publish(&self, event_type: EventType, payload: EventPayload) {
        let mut event = Event::new(event_type);
        event.project_id = self.project_id.clone();
        event.operation_id = self.operation_id.clone();
        event.payload = payload;
        self.bus.publish(event).await;
    }
}

/// Trait for event subscribers
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    async fn on_event(&self, event: Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn name(&self) -> &str;
}

/// Logging event subscriber
#[derive(Debug, Clone)]
pub struct LoggingEventSubscriber {
    name: String,
}

#[async_trait]
impl EventSubscriber for LoggingEventSubscriber {
    async fn on_event(&self, event: Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("[LOG] Event: {:?} - {:?}", event.event_type, event.payload);
        Ok(())
    }
    fn name(&self) -> &str { &self.name }
}

impl LoggingEventSubscriber {
    pub fn new(name: String) -> Self { Self { name } }
}

/// Filtered event subscriber
#[derive(Debug, Clone)]
pub struct FilteredEventSubscriber {
    inner: Arc<dyn EventSubscriber + Send + Sync>,
    filter: Box<dyn Fn(&Event) -> bool + Send + Sync>,
}

#[async_trait]
impl EventSubscriber for FilteredEventSubscriber {
    async fn on_event(&self, event: Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if (self.filter)(&event) {
            self.inner.on_event(event).await
        } else {
            Ok(())
        }
    }
    fn name(&self) -> &str { self.inner.name() }
}

/// Multi event subscriber
#[derive(Debug, Clone)]
pub struct MultiEventSubscriber {
    subscribers: Vec<Arc<dyn EventSubscriber + Send + Sync>>,
    name: String,
}

#[async_trait]
impl EventSubscriber for MultiEventSubscriber {
    async fn on_event(&self, event: Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for subscriber in &self.subscribers {
            let _ = subscriber.on_event(event.clone()).await;
        }
        Ok(())
    }
    fn name(&self) -> &str { &self.name }
}

impl MultiEventSubscriber {
    pub fn new(name: String) -> Self { Self { subscribers: Vec::new(), name } }
    pub fn add_subscriber(mut self, subscriber: Arc<dyn EventSubscriber + Send + Sync>) -> Self {
        self.subscribers.push(subscriber);
        self
    }
}
