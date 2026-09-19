use crate::error::JobResult;
use malverde_core::{JobId, JobState, ActorId, ProjectId, OperationId, Confidence};
use serde::{Serialize, Deserialize};
use std::time::{Duration, Instant};
use std::collections::HashMap;

/// Represents a job in the Malverde system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job identifier
    pub id: JobId,
    
    /// Parent project ID
    pub project_id: ProjectId,
    
    /// Parent operation ID
    pub operation_id: OperationId,
    
    /// Actor that created/owns the job
    pub actor_id: ActorId,
    
    /// Job name/description
    pub name: String,
    
    /// Current state of the job
    pub state: JobState,
    
    /// Job priority (0-100, higher is more important)
    pub priority: u8,
    
    /// Creation timestamp
    pub created_at: Instant,
    
    /// Last update timestamp
    pub updated_at: Instant,
    
    /// Start timestamp (when job began execution)
    pub started_at: Option<Instant>,
    
    /// Completion timestamp
    pub completed_at: Option<Instant>,
    
    /// Current progress (0.0 - 1.0)
    pub progress: f32,
    
    /// Confidence score for job results
    pub confidence: Confidence,
    
    /// Job metadata
    pub metadata: HashMap<String, String>,
    
    /// Job payload/data
    pub payload: JobPayload,
    
    /// Results produced by the job
    pub results: Vec<JobResult>,
    
    /// Error message if job failed
    pub error: Option<String>,
    
    /// Retry count
    pub retry_count: u32,
    
    /// Maximum retries allowed
    pub max_retries: u32,
    
    /// Timeout duration
    pub timeout: Option<Duration>,
    
    /// Checkpoint interval
    pub checkpoint_interval: Option<Duration>,
}

impl Job {
    /// Create a new job
    pub fn new(
        id: JobId,
        project_id: ProjectId,
        operation_id: OperationId,
        actor_id: ActorId,
        name: String,
        payload: JobPayload,
    ) -> Self {
        Self {
            id,
            project_id,
            operation_id,
            actor_id,
            name,
            state: JobState::Pending,
            priority: 50,
            created_at: Instant::now(),
            updated_at: Instant::now(),
            started_at: None,
            completed_at: None,
            progress: 0.0,
            confidence: Confidence::new(0),
            metadata: HashMap::new(),
            payload,
            results: Vec::new(),
            error: None,
            retry_count: 0,
            max_retries: 3,
            timeout: None,
            checkpoint_interval: None,
        }
    }

    /// Start the job
    pub fn start(&mut self) -> JobResult<()> {
        if self.state != JobState::Pending {
            return Err(crate::error::JobError::InvalidState(
                format!("Cannot start job in state: {:?}", self.state)
            ));
        }
        self.state = JobState::Running;
        self.started_at = Some(Instant::now());
        self.updated_at = Instant::now();
        Ok(())
    }

    /// Pause the job
    pub fn pause(&mut self) -> JobResult<()> {
        if self.state != JobState::Running {
            return Err(crate::error::JobError::InvalidState(
                format!("Cannot pause job in state: {:?}", self.state)
            ));
        }
        self.state = JobState::Paused;
        self.updated_at = Instant::now();
        Ok(())
    }

    /// Resume the job
    pub fn resume(&mut self) -> JobResult<()> {
        if self.state != JobState::Paused {
            return Err(crate::error::JobError::InvalidState(
                format!("Cannot resume job in state: {:?}", self.state)
            ));
        }
        self.state = JobState::Running;
        self.updated_at = Instant::now();
        Ok(())
    }

    /// Complete the job successfully
    pub fn complete(&mut self, results: Vec<JobResult>) -> JobResult<()> {
        if self.state != JobState::Running {
            return Err(crate::error::JobError::InvalidState(
                format!("Cannot complete job in state: {:?}", self.state)
            ));
        }
        self.state = JobState::Completed;
        self.completed_at = Some(Instant::now());
        self.progress = 1.0;
        self.results = results;
        self.updated_at = Instant::now();
        Ok(())
    }

    /// Fail the job
    pub fn fail(&mut self, error: String) -> JobResult<()> {
        if self.state == JobState::Completed || self.state == JobState::Failed {
            return Err(crate::error::JobError::InvalidState(
                format!("Cannot fail job in state: {:?}", self.state)
            ));
        }
        self.state = JobState::Failed;
        self.error = Some(error);
        self.completed_at = Some(Instant::now());
        self.updated_at = Instant::now();
        Ok(())
    }

    /// Cancel the job
    pub fn cancel(&mut self) -> JobResult<()> {
        if self.state == JobState::Completed || self.state == JobState::Failed {
            return Err(crate::error::JobError::InvalidState(
                format!("Cannot cancel job in state: {:?}", self.state)
            ));
        }
        self.state = JobState::Cancelled;
        self.completed_at = Some(Instant::now());
        self.updated_at = Instant::now();
        Ok(())
    }

    /// Update progress (0.0 - 1.0)
    pub fn update_progress(&mut self, progress: f32) -> JobResult<()> {
        if progress < 0.0 || progress > 1.0 {
            return Err(crate::error::JobError::InvalidState(
                format!("Progress must be between 0.0 and 1.0, got: {}", progress)
            ));
        }
        self.progress = progress;
        self.updated_at = Instant::now();
        Ok(())
    }

    /// Check if job is completed (successfully or failed)
    pub fn is_completed(&self) -> bool {
        matches!(
            self.state,
            JobState::Completed | JobState::Failed | JobState::Cancelled
        )
    }

    /// Check if job is active
    pub fn is_active(&self) -> bool {
        matches!(self.state, JobState::Running | JobState::Paused)
    }

    /// Get job duration
    pub fn duration(&self) -> Option<Duration> {
        self.started_at.map(|start| {
            self.completed_at
                .map(|end| end - start)
                .unwrap_or_else(|| start.elapsed())
        })
    }
}

/// Job payload types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobPayload {
    /// Execute a command
    Command { command: String, args: Vec<String> },
    
    /// Process data
    Data { data: String, format: String },
    
    /// Analyze knowledge
    Knowledge { query: String, context: Option<String> },
    
    /// Train model
    Train { dataset: String, config: HashMap<String, String> },
    
    /// Custom payload
    Custom(HashMap<String, serde_json::Value>),
}

/// Job result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    /// Result data
    pub data: String,
    
    /// Result type
    pub result_type: String,
    
    /// Confidence score
    pub confidence: Confidence,
    
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Job builder for fluent job creation
pub struct JobBuilder {
    id: Option<JobId>,
    project_id: Option<ProjectId>,
    operation_id: Option<OperationId>,
    actor_id: Option<ActorId>,
    name: Option<String>,
    payload: Option<JobPayload>,
    priority: u8,
    max_retries: u32,
    timeout: Option<Duration>,
    checkpoint_interval: Option<Duration>,
    metadata: HashMap<String, String>,
}

impl JobBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            project_id: None,
            operation_id: None,
            actor_id: None,
            name: None,
            payload: None,
            priority: 50,
            max_retries: 3,
            timeout: None,
            checkpoint_interval: None,
            metadata: HashMap::new(),
        }
    }

    pub fn id(mut self, id: JobId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn project_id(mut self, project_id: ProjectId) -> Self {
        self.project_id = Some(project_id);
        self
    }

    pub fn operation_id(mut self, operation_id: OperationId) -> Self {
        self.operation_id = Some(operation_id);
        self
    }

    pub fn actor_id(mut self, actor_id: ActorId) -> Self {
        self.actor_id = Some(actor_id);
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn payload(mut self, payload: JobPayload) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn priority(mut self, priority: u8) -> Self {
        self.priority = priority.clamp(0, 100);
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn checkpoint_interval(mut self, interval: Duration) -> Self {
        self.checkpoint_interval = Some(interval);
        self
    }

    pub fn metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn build(self) -> JobResult<Job> {
        Ok(Job {
            id: self.id.ok_or_else(|| {
                crate::error::JobError::InvalidState("Job ID is required".to_string())
            })?,
            project_id: self.project_id.ok_or_else(|| {
                crate::error::JobError::InvalidState("Project ID is required".to_string())
            })?,
            operation_id: self.operation_id.ok_or_else(|| {
                crate::error::JobError::InvalidState("Operation ID is required".to_string())
            })?,
            actor_id: self.actor_id.ok_or_else(|| {
                crate::error::JobError::InvalidState("Actor ID is required".to_string())
            })?,
            name: self.name.ok_or_else(|| {
                crate::error::JobError::InvalidState("Job name is required".to_string())
            })?,
            payload: self.payload.ok_or_else(|| {
                crate::error::JobError::InvalidState("Job payload is required".to_string())
            })?,
            state: JobState::Pending,
            priority: self.priority,
            created_at: Instant::now(),
            updated_at: Instant::now(),
            started_at: None,
            completed_at: None,
            progress: 0.0,
            confidence: Confidence::new(0),
            metadata: self.metadata,
            results: Vec::new(),
            error: None,
            retry_count: 0,
            max_retries: self.max_retries,
            timeout: self.timeout,
            checkpoint_interval: self.checkpoint_interval,
        })
    }
}
