use serde::{Deserialize, Serialize};
use malverde_core::types::{CheckpointId, EventId, JobId, KnowledgeId, MemoryId, PluginId};
use malverde_core::types::{JobPriority, JobStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Knowledge {
    pub id: KnowledgeId,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Knowledge {
    pub fn new(title: impl Into<String>, content: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: KnowledgeId::new(),
            title: title.into(),
            content: content.into(),
            tags: Vec::new(),
            metadata: serde_json::Value::Null,
            created_at: now,
            updated_at: now,
        }
    }
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: MemoryId,
    pub key: String,
    pub value: serde_json::Value,
    pub session_id: Option<String>,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Memory {
    pub fn new(key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: MemoryId::new(),
            key: key.into(),
            value: value.into(),
            session_id: None,
            expires_at: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    pub id: EventId,
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: i64,
    pub processed: bool,
}

impl StoredEvent {
    pub fn new(id: EventId, event_type: impl Into<String>, data: impl Into<serde_json::Value>) -> Self {
        Self {
            id,
            event_type: event_type.into(),
            data: data.into(),
            timestamp: chrono::Utc::now().timestamp(),
            processed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub name: String,
    pub description: Option<String>,
    pub status: JobStatus,
    pub priority: JobPriority,
    pub data: serde_json::Value,
    pub checkpoint_id: Option<CheckpointId>,
    pub created_at: i64,
    pub updated_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

impl Job {
    pub fn new(name: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: JobId::new(),
            name: name.into(),
            description: None,
            status: JobStatus::Pending,
            priority: JobPriority::Normal,
            data: serde_json::Value::Null,
            checkpoint_id: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
        }
    }
    pub fn start(&mut self) {
        self.status = JobStatus::Running;
        self.started_at = Some(chrono::Utc::now().timestamp());
        self.updated_at = chrono::Utc::now().timestamp();
    }
    pub fn complete(&mut self) {
        self.status = JobStatus::Completed;
        self.completed_at = Some(chrono::Utc::now().timestamp());
        self.updated_at = chrono::Utc::now().timestamp();
    }
    pub fn fail(&mut self) {
        self.status = JobStatus::Failed;
        self.updated_at = chrono::Utc::now().timestamp();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: CheckpointId,
    pub job_id: JobId,
    pub state: serde_json::Value,
    pub created_at: i64,
}

impl Checkpoint {
    pub fn new(job_id: JobId, state: impl Into<serde_json::Value>) -> Self {
        Self {
            id: CheckpointId::new(),
            job_id,
            state: state.into(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i64,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub user: Option<String>,
    pub timestamp: i64,
    pub details: serde_json::Value,
}

impl AuditLog {
    pub fn new(action: impl Into<String>, entity_type: impl Into<String>, entity_id: impl Into<String>) -> Self {
        Self {
            id: 0,
            action: action.into(),
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            user: None,
            timestamp: chrono::Utc::now().timestamp(),
            details: serde_json::Value::Null,
        }
    }
}