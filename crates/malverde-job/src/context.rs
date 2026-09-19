use crate::error::JobResult;
use malverde_core::{JobId, ActorId, ProjectId, OperationId, Command, Intent, EvidenceId};
use std::collections::HashMap;
use std::sync::Arc;

/// Job execution context
#[derive(Debug, Clone)]
pub struct JobContext {
    /// Job ID
    pub job_id: JobId,
    
    /// Project ID
    pub project_id: ProjectId,
    
    /// Operation ID
    pub operation_id: OperationId,
    
    /// Actor ID
    pub actor_id: ActorId,
    
    /// Current command being executed
    pub command: Option<Command>,
    
    /// Current intent
    pub intent: Option<Intent>,
    
    /// Working directory
    pub working_dir: String,
    
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    
    /// Shared resources
    pub resources: HashMap<String, Arc<dyn std::any::Any + Send + Sync>>,
    
    /// Temporary files
    pub temp_files: Vec<String>,
    
    /// Evidence collected during execution
    pub evidence_ids: Vec<EvidenceId>,
    
    /// Execution flags
    pub flags: HashMap<String, bool>,
    
    /// Timeout timestamp
    pub timeout_at: Option<std::time::Instant>,
}

impl JobContext {
    /// Create a new job context
    pub fn new(
        job_id: JobId,
        project_id: ProjectId,
        operation_id: OperationId,
        actor_id: ActorId,
    ) -> Self {
        Self {
            job_id,
            project_id,
            operation_id,
            actor_id,
            command: None,
            intent: None,
            working_dir: "/tmp/malverde".to_string(),
            env_vars: HashMap::new(),
            resources: HashMap::new(),
            temp_files: Vec::new(),
            evidence_ids: Vec::new(),
            flags: HashMap::new(),
            timeout_at: None,
        }
    }

    /// Set the current command
    pub fn set_command(&mut self, command: Command) {
        self.command = Some(command);
    }

    /// Set the current intent
    pub fn set_intent(&mut self, intent: Intent) {
        self.intent = Some(intent);
    }

    /// Add an environment variable
    pub fn set_env(&mut self, key: String, value: String) {
        self.env_vars.insert(key, value);
    }

    /// Get an environment variable
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.env_vars.get(key)
    }

    /// Add a resource
    pub fn add_resource<T: std::any::Any + Send + Sync>(&mut self, key: String, value: T) {
        self.resources.insert(key, Arc::new(value));
    }

    /// Get a resource
    pub fn get_resource<T: std::any::Any + Send + Sync>(&self, key: &str) -> Option<Arc<T>> {
        self.resources
            .get(key)
            .and_then(|rc| rc.downcast_ref::<T>())
            .map(|t| Arc::new(t.clone()))
    }

    /// Add a temporary file
    pub fn add_temp_file(&mut self, path: String) {
        self.temp_files.push(path);
    }

    /// Add evidence ID
    pub fn add_evidence(&mut self, evidence_id: EvidenceId) {
        self.evidence_ids.push(evidence_id);
    }

    /// Set a flag
    pub fn set_flag(&mut self, key: String, value: bool) {
        self.flags.insert(key, value);
    }

    /// Get a flag
    pub fn get_flag(&self, key: &str) -> bool {
        *self.flags.get(key).unwrap_or(&false)
    }

    /// Set timeout
    pub fn set_timeout(&mut self, duration: std::time::Duration) {
        self.timeout_at = Some(std::time::Instant::now() + duration);
    }

    /// Check if timeout has occurred
    pub fn is_timeout(&self) -> bool {
        self.timeout_at
            .map(|t| std::time::Instant::now() > t)
            .unwrap_or(false)
    }

    /// Cleanup temporary files
    pub fn cleanup_temp_files(&self) -> JobResult<()> {
        for path in &self.temp_files {
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }
}

/// Context builder for fluent context creation
pub struct ContextBuilder {
    job_id: JobId,
    project_id: ProjectId,
    operation_id: OperationId,
    actor_id: ActorId,
    working_dir: String,
    env_vars: HashMap<String, String>,
    flags: HashMap<String, bool>,
}

impl ContextBuilder {
    pub fn new(job_id: JobId, project_id: ProjectId, operation_id: OperationId, actor_id: ActorId) -> Self {
        Self {
            job_id,
            project_id,
            operation_id,
            actor_id,
            working_dir: "/tmp/malverde".to_string(),
            env_vars: HashMap::new(),
            flags: HashMap::new(),
        }
    }

    pub fn working_dir(mut self, dir: String) -> Self {
        self.working_dir = dir;
        self
    }

    pub fn env(mut self, key: String, value: String) -> Self {
        self.env_vars.insert(key, value);
        self
    }

    pub fn flag(mut self, key: String, value: bool) -> Self {
        self.flags.insert(key, value);
        self
    }

    pub fn build(self) -> JobContext {
        JobContext {
            job_id: self.job_id,
            project_id: self.project_id,
            operation_id: self.operation_id,
            actor_id: self.actor_id,
            command: None,
            intent: None,
            working_dir: self.working_dir,
            env_vars: self.env_vars,
            resources: HashMap::new(),
            temp_files: Vec::new(),
            evidence_ids: Vec::new(),
            flags: self.flags,
            timeout_at: None,
        }
    }
}

/// Trait for context-aware job execution
pub trait ContextAware {
    fn set_context(&mut self, context: JobContext);
    fn get_context(&self) -> Option<&JobContext>;
}
