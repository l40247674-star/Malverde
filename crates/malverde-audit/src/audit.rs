use malverde_core::{AuditId, ActorId, ProjectId, OperationId, JobId, EventId, AuditState};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: AuditId,
    pub timestamp: DateTime<Utc>,
    pub actor_id: Option<ActorId>,
    pub project_id: Option<ProjectId>,
    pub operation_id: Option<OperationId>,
    pub job_id: Option<JobId>,
    pub event_id: Option<EventId>,
    pub action: AuditAction,
    pub resource_type: String,
    pub resource_id: String,
    pub state: AuditState,
    pub previous_state: Option<AuditState>,
    pub details: HashMap<String, String>,
    pub success: bool,
    pub error: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl AuditLog {
    pub fn new(action: AuditAction, resource_type: String, resource_id: String) -> Self {
        Self {
            id: AuditId::new(),
            timestamp: Utc::now(),
            actor_id: None,
            project_id: None,
            operation_id: None,
            job_id: None,
            event_id: None,
            action,
            resource_type,
            resource_id,
            state: AuditState::Recorded,
            previous_state: None,
            details: HashMap::new(),
            success: true,
            error: None,
            ip_address: None,
            user_agent: None,
            metadata: HashMap::new(),
        }
    }
    pub fn with_actor(mut self, actor_id: ActorId) -> Self { self.actor_id = Some(actor_id); self }
    pub fn with_project(mut self, project_id: ProjectId) -> Self { self.project_id = Some(project_id); self }
    pub fn with_operation(mut self, operation_id: OperationId) -> Self { self.operation_id = Some(operation_id); self }
    pub fn with_job(mut self, job_id: JobId) -> Self { self.job_id = Some(job_id); self }
    pub fn with_event(mut self, event_id: EventId) -> Self { self.event_id = Some(event_id); self }
    pub fn with_state_change(mut self, new_state: AuditState, previous_state: AuditState) -> Self {
        self.state = new_state; self.previous_state = Some(previous_state); self
    }
    pub fn with_success(mut self, success: bool) -> Self { self.success = success; self }
    pub fn with_error(mut self, error: String) -> Self { self.success = false; self.error = Some(error); self }
    pub fn mark_recorded(&mut self) { self.state = AuditState::Recorded; }
    pub fn mark_processed(&mut self) { self.state = AuditState::Processed; }
    pub fn mark_archived(&mut self) { self.state = AuditState::Archived; }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AuditAction {
    SystemStartup, SystemShutdown, SystemConfigurationChanged,
    ProjectCreated, ProjectUpdated, ProjectDeleted, ProjectStateChanged,
    ActorCreated, ActorUpdated, ActorDeleted, ActorCommandExecuted,
    OperationCreated, OperationStarted, OperationCompleted, OperationFailed, OperationPaused, OperationResumed,
    JobCreated, JobStarted, JobCompleted, JobFailed, JobCancelled, JobPaused, JobResumed,
    KnowledgeAdded, KnowledgeUpdated, KnowledgeRemoved, KnowledgeSearched,
    MemoryAdded, MemoryUpdated, MemoryRemoved, MemorySearched,
    EvidenceCollected, EvidenceAnalyzed, EvidenceStored, EvidenceRetrieved, EvidenceDeleted,
    LearningStarted, LearningCompleted, LearningFailed, ModelTrained,
    PluginLoaded, PluginUnloaded, PluginError, PluginCommandExecuted,
    SecurityCheckPassed, SecurityCheckFailed, PermissionGranted, PermissionDenied, AuthenticationSuccess, AuthenticationFailed,
    StorageConnected, StorageDisconnected, MigrationApplied, BackupCreated,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditQuery {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub actor_id: Option<ActorId>,
    pub project_id: Option<ProjectId>,
    pub operation_id: Option<OperationId>,
    pub job_id: Option<JobId>,
    pub actions: Option<Vec<AuditAction>>,
    pub resource_types: Option<Vec<String>>,
    pub resource_ids: Option<Vec<String>>,
    pub success: Option<bool>,
    pub states: Option<Vec<AuditState>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_logs: usize,
    pub by_action: HashMap<AuditAction, usize>,
    pub by_resource_type: HashMap<String, usize>,
    pub by_actor: HashMap<ActorId, usize>,
    pub by_project: HashMap<ProjectId, usize>,
    pub success_count: usize,
    pub failure_count: usize,
    pub by_state: HashMap<AuditState, usize>,
}

pub trait AuditStorage: Send + Sync {
    fn save(&self, log: &AuditLog) -> Result<(), Box<dyn std::error::Error>>;
    fn find_by_id(&self, id: &AuditId) -> Result<Option<AuditLog>, Box<dyn std::error::Error>>;
    fn query(&self, query: &AuditQuery) -> Result<Vec<AuditLog>, Box<dyn std::error::Error>>;
    fn get_summary(&self, query: &AuditQuery) -> Result<AuditSummary, Box<dyn std::error::Error>>;
    fn delete_by_id(&self, id: &AuditId) -> Result<(), Box<dyn std::error::Error>>;
    fn delete_by_query(&self, query: &AuditQuery) -> Result<usize, Box<dyn std::error::Error>>;
}
