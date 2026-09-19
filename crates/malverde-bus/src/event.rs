use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use malverde_core::types::{EventId, EventType, KnowledgeId, MemoryId, JobId};

pub trait Event: Send + Sync + std::fmt::Debug {
    fn event_type(&self) -> EventType;
    fn id(&self) -> EventId;
    fn timestamp(&self) -> i64;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericEvent {
    pub id: EventId,
    pub event_type: EventType,
    pub timestamp: i64,
    pub data: serde_json::Value,
}

impl GenericEvent {
    pub fn new(event_type: EventType, data: serde_json::Value) -> Self {
        Self {
            id: EventId::new(),
            event_type,
            timestamp: chrono::Utc::now().timestamp(),
            data,
        }
    }
}

impl Event for GenericEvent {
    fn event_type(&self) -> EventType {
        self.event_type.clone()
    }
    fn id(&self) -> EventId {
        self.id
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeAddedEvent {
    pub id: EventId,
    pub knowledge_id: KnowledgeId,
    pub timestamp: i64,
}

impl KnowledgeAddedEvent {
    pub fn new(knowledge_id: KnowledgeId) -> Self {
        Self {
            id: EventId::new(),
            knowledge_id,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

impl Event for KnowledgeAddedEvent {
    fn event_type(&self) -> EventType {
        EventType::KnowledgeAdded
    }
    fn id(&self) -> EventId {
        self.id
    }
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
}