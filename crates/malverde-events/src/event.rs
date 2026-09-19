use malverde_core::{EventId, ActorId, ProjectId, OperationId, EventTimestamp, Confidence};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub project_id: Option<ProjectId>,
    pub operation_id: Option<OperationId>,
    pub actor_id: Option<ActorId>,
    pub event_type: EventType,
    pub state: EventState,
    pub payload: EventPayload,
    pub timestamp: EventTimestamp,
    pub confidence: Confidence,
    pub metadata: HashMap<String, String>,
    pub parent_id: Option<EventId>,
    pub child_ids: Vec<EventId>,
    pub sequence: u64,
    pub source: String,
}

impl Event {
    pub fn new(event_type: EventType) -> Self {
        Self {
            id: EventId::new(),
            project_id: None,
            operation_id: None,
            actor_id: None,
            event_type,
            state: EventState::Recorded,
            payload: EventPayload::Empty,
            timestamp: EventTimestamp::now(),
            confidence: Confidence::new(100),
            metadata: HashMap::new(),
            parent_id: None,
            child_ids: Vec::new(),
            sequence: 0,
            source: "unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventType {
    SystemStartup, SystemShutdown, SystemHealthCheck, SystemError,
    ProjectCreated, ProjectUpdated, ProjectDeleted, ProjectStateChanged,
    ActorCreated, ActorUpdated, ActorDeleted, ActorCommandExecuted,
    OperationStarted, OperationCompleted, OperationFailed, OperationPaused, OperationResumed,
    JobCreated, JobStarted, JobProgress, JobCompleted, JobFailed, JobCancelled, JobPaused, JobResumed,
    KnowledgeAdded, KnowledgeUpdated, KnowledgeRemoved, KnowledgeSearched, KnowledgeIndexed,
    MemoryAdded, MemoryUpdated, MemoryRemoved, MemorySearched, MemoryIndexed,
    EvidenceCollected, EvidenceAnalyzed, EvidenceStored, EvidenceRetrieved,
    LearningStarted, LearningProgress, LearningCompleted, LearningFailed, ModelTrained,
    PluginLoaded, PluginUnloaded, PluginError, PluginCommandExecuted,
    SecurityCheckPassed, SecurityCheckFailed, PermissionGranted, PermissionDenied,
    StorageConnected, StorageDisconnected, StorageError, MigrationApplied,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventState {
    Recorded, Processing, Failed, Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPayload {
    Empty,
    Text(String),
    Json(serde_json::Value),
    Structured(HashMap<String, serde_json::Value>),
    Binary(Vec<u8>),
    Reference { uri: String, r#type: String },
}

pub struct EventBuilder {
    event_type: EventType,
    project_id: Option<ProjectId>,
    operation_id: Option<OperationId>,
    actor_id: Option<ActorId>,
    payload: EventPayload,
    metadata: HashMap<String, String>,
    parent_id: Option<EventId>,
    sequence: u64,
    source: String,
    confidence: Confidence,
}

impl EventBuilder {
    pub fn new(event_type: EventType) -> Self {
        Self {
            event_type,
            project_id: None,
            operation_id: None,
            actor_id: None,
            payload: EventPayload::Empty,
            metadata: HashMap::new(),
            parent_id: None,
            sequence: 0,
            source: "unknown".to_string(),
            confidence: Confidence::new(100),
        }
    }

    pub fn project_id(mut self, project_id: ProjectId) -> Self { self.project_id = Some(project_id); self }
    pub fn operation_id(mut self, operation_id: OperationId) -> Self { self.operation_id = Some(operation_id); self }
    pub fn actor_id(mut self, actor_id: ActorId) -> Self { self.actor_id = Some(actor_id); self }
    pub fn payload(mut self, payload: EventPayload) -> Self { self.payload = payload; self }
    pub fn text_payload(mut self, text: String) -> Self { self.payload = EventPayload::Text(text); self }
    pub fn json_payload(mut self, json: serde_json::Value) -> Self { self.payload = EventPayload::Json(json); self }
    pub fn metadata(mut self, key: String, value: String) -> Self { self.metadata.insert(key, value); self }
    pub fn parent_id(mut self, parent_id: EventId) -> Self { self.parent_id = Some(parent_id); self }
    pub fn sequence(mut self, sequence: u64) -> Self { self.sequence = sequence; self }
    pub fn source(mut self, source: String) -> Self { self.source = source; self }
    pub fn confidence(mut self, confidence: Confidence) -> Self { self.confidence = confidence; self }

    pub fn build(self) -> Event {
        Event {
            id: EventId::new(),
            project_id: self.project_id,
            operation_id: self.operation_id,
            actor_id: self.actor_id,
            event_type: self.event_type,
            state: EventState::Recorded,
            payload: self.payload,
            timestamp: EventTimestamp::now(),
            confidence: self.confidence,
            metadata: self.metadata,
            parent_id: self.parent_id,
            child_ids: Vec::new(),
            sequence: self.sequence,
            source: self.source,
        }
    }
}
