use crate::error::{JobError, JobResult};
use crate::job::{Job, JobBuilder, JobPayload, JobState};
use crate::context::{JobContext, ContextBuilder};
use malverde_core::{JobId, ActorId, ProjectId, OperationId, MalverdeResult};
use malverde_storage::repository::Repository;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{channel, Sender, Receiver};
use tokio::task::JoinHandle;

/// Job manager for managing job lifecycle
#[derive(Debug)]
pub struct JobManager {
    /// Map of job ID to job
    jobs: Arc<RwLock<HashMap<JobId, Job>>>,
    
    /// Map of job ID to context
    contexts: Arc<RwLock<HashMap<JobId, JobContext>>>,
    
    /// Queue of pending jobs
    pending_queue: Arc<RwLock<VecDeque<JobId>>>,
    
    /// Queue of running jobs
    running_queue: Arc<RwLock<VecDeque<JobId>>>,
    
    /// Maximum concurrent jobs
    max_concurrent: usize,
    
    /// Job repository for persistence
    repository: Option<Arc<dyn Repository<Job = Job>>>,
    
    /// Checkpoint interval
    checkpoint_interval: Duration,
    
    /// Last checkpoint time
    last_checkpoint: Instant,
}

impl JobManager {
    /// Create a new job manager
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            contexts: Arc::new(RwLock::new(HashMap::new())),
            pending_queue: Arc::new(RwLock::new(VecDeque::new())),
            running_queue: Arc::new(RwLock::new(VecDeque::new())),
            max_concurrent,
            repository: None,
            checkpoint_interval: Duration::from_secs(30),
            last_checkpoint: Instant::now(),
        }
    }

    /// Create a new job manager with repository
    pub fn with_repository(
        max_concurrent: usize,
        repository: Arc<dyn Repository<Job = Job>>,
    ) -> Self {
        Self {
            repository: Some(repository),
            ..Self::new(max_concurrent)
        }
    }

    /// Add a job to the manager
    pub fn add_job(&self, job: Job) -> JobResult<JobId> {
        let mut jobs = self.jobs.write().unwrap();
        let mut pending = self.pending_queue.write().unwrap();
        
        if jobs.contains_key(&job.id) {
            return Err(JobError::AlreadyExists(job.id.to_string()));
        }
        
        jobs.insert(job.id.clone(), job);
        pending.push_back(job.id.clone());
        
        Ok(job.id)
    }

    /// Create a job using the builder
    pub fn create_job(
        &self,
        project_id: ProjectId,
        operation_id: OperationId,
        actor_id: ActorId,
        name: String,
        payload: JobPayload,
    ) -> JobResult<JobId> {
        let job = JobBuilder::new()
            .id(JobId::new())
            .project_id(project_id)
            .operation_id(operation_id)
            .actor_id(actor_id)
            .name(name)
            .payload(payload)
            .build()?;
        
        self.add_job(job)
    }

    /// Get a job by ID
    pub fn get_job(&self, job_id: &JobId) -> JobResult<Job> {
        let jobs = self.jobs.read().unwrap();
        jobs.get(job_id)
            .cloned()
            .ok_or_else(|| JobError::NotFound(job_id.to_string()))
    }

    /// Get all jobs
    pub fn get_all_jobs(&self) -> Vec<Job> {
        let jobs = self.jobs.read().unwrap();
        jobs.values().cloned().collect()
    }

    /// Get jobs by state
    pub fn get_jobs_by_state(&self, state: JobState) -> Vec<Job> {
        let jobs = self.jobs.read().unwrap();
        jobs.values()
            .filter(|j| j.state == state)
            .cloned()
            .collect()
    }

    /// Get jobs by project
    pub fn get_jobs_by_project(&self, project_id: &ProjectId) -> Vec<Job> {
        let jobs = self.jobs.read().unwrap();
        jobs.values()
            .filter(|j| &j.project_id == project_id)
            .cloned()
            .collect()
    }

    /// Get jobs by actor
    pub fn get_jobs_by_actor(&self, actor_id: &ActorId) -> Vec<Job> {
        let jobs = self.jobs.read().unwrap();
        jobs.values()
            .filter(|j| &j.actor_id == actor_id)
            .cloned()
            .collect()
    }

    /// Start a job
    pub fn start_job(&self, job_id: &JobId) -> JobResult<()> {
        let mut jobs = self.jobs.write().unwrap();
        let mut pending = self.pending_queue.write().unwrap();
        let mut running = self.running_queue.write().unwrap();
        
        if let Some(job) = jobs.get_mut(job_id) {
            if running.len() >= self.max_concurrent {
                return Err(JobError::InvalidState(
                    "Maximum concurrent jobs reached".to_string()
                ));
            }
            
            job.start()?;
            
            // Remove from pending queue
            if let Some(pos) = pending.iter().position(|id| id == job_id) {
                pending.remove(pos);
            }
            
            // Add to running queue
            running.push_back(job_id.clone());
            
            // Create context
            let context = ContextBuilder::new(
                job_id.clone(),
                job.project_id.clone(),
                job.operation_id.clone(),
                job.actor_id.clone(),
            )
            .build();
            
            let mut contexts = self.contexts.write().unwrap();
            contexts.insert(job_id.clone(), context);
            
            Ok(())
        } else {
            Err(JobError::NotFound(job_id.to_string()))
        }
    }

    /// Complete a job
    pub fn complete_job(&self, job_id: &JobId, results: Vec<crate::job::JobResult>) -> JobResult<()> {
        let mut jobs = self.jobs.write().unwrap();
        let mut running = self.running_queue.write().unwrap();
        
        if let Some(job) = jobs.get_mut(job_id) {
            job.complete(results)?;
            
            // Remove from running queue
            if let Some(pos) = running.iter().position(|id| id == job_id) {
                running.remove(pos);
            }
            
            // Cleanup context
            let mut contexts = self.contexts.write().unwrap();
            contexts.remove(job_id);
            
            Ok(())
        } else {
            Err(JobError::NotFound(job_id.to_string()))
        }
    }

    /// Fail a job
    pub fn fail_job(&self, job_id: &JobId, error: String) -> JobResult<()> {
        let mut jobs = self.jobs.write().unwrap();
        let mut running = self.running_queue.write().unwrap();
        
        if let Some(job) = jobs.get_mut(job_id) {
            job.fail(error)?;
            
            // Remove from running queue
            if let Some(pos) = running.iter().position(|id| id == job_id) {
                running.remove(pos);
            }
            
            // Cleanup context
            let mut contexts = self.contexts.write().unwrap();
            contexts.remove(job_id);
            
            Ok(())
        } else {
            Err(JobError::NotFound(job_id.to_string()))
        }
    }

    /// Cancel a job
    pub fn cancel_job(&self, job_id: &JobId) -> JobResult<()> {
        let mut jobs = self.jobs.write().unwrap();
        let mut pending = self.pending_queue.write().unwrap();
        let mut running = self.running_queue.write().unwrap();
        
        if let Some(job) = jobs.get_mut(job_id) {
            job.cancel()?;
            
            // Remove from queues
            if let Some(pos) = pending.iter().position(|id| id == job_id) {
                pending.remove(pos);
            }
            if let Some(pos) = running.iter().position(|id| id == job_id) {
                running.remove(pos);
            }
            
            // Cleanup context
            let mut contexts = self.contexts.write().unwrap();
            contexts.remove(job_id);
            
            Ok(())
        } else {
            Err(JobError::NotFound(job_id.to_string()))
        }
    }

    /// Pause a job
    pub fn pause_job(&self, job_id: &JobId) -> JobResult<()> {
        let mut jobs = self.jobs.write().unwrap();
        
        if let Some(job) = jobs.get_mut(job_id) {
            job.pause()
        } else {
            Err(JobError::NotFound(job_id.to_string()))
        }
    }

    /// Resume a job
    pub fn resume_job(&self, job_id: &JobId) -> JobResult<()> {
        let mut jobs = self.jobs.write().unwrap();
        
        if let Some(job) = jobs.get_mut(job_id) {
            job.resume()
        } else {
            Err(JobError::NotFound(job_id.to_string()))
        }
    }

    /// Remove a job
    pub fn remove_job(&self, job_id: &JobId) -> JobResult<Job> {
        let mut jobs = self.jobs.write().unwrap();
        let mut pending = self.pending_queue.write().unwrap();
        let mut running = self.running_queue.write().unwrap();
        let mut contexts = self.contexts.write().unwrap();
        
        if let Some(job) = jobs.remove(job_id) {
            // Remove from queues
            if let Some(pos) = pending.iter().position(|id| id == job_id) {
                pending.remove(pos);
            }
            if let Some(pos) = running.iter().position(|id| id == job_id) {
                running.remove(pos);
            }
            
            // Cleanup context
            contexts.remove(job_id);
            
            Ok(job)
        } else {
            Err(JobError::NotFound(job_id.to_string()))
        }
    }

    /// Get job context
    pub fn get_context(&self, job_id: &JobId) -> Option<JobContext> {
        let contexts = self.contexts.read().unwrap();
        contexts.get(job_id).cloned()
    }

    /// Get statistics
    pub fn stats(&self) -> JobStats {
        let jobs = self.jobs.read().unwrap();
        let pending = self.pending_queue.read().unwrap();
        let running = self.running_queue.read().unwrap();
        
        let mut stats = HashMap::new();
        for job in jobs.values() {
            *stats.entry(job.state.clone()).or_insert(0) += 1;
        }
        
        JobStats {
            total: jobs.len(),
            pending: pending.len(),
            running: running.len(),
            by_state: stats,
        }
    }

    /// Process next job in queue
    pub fn process_next(&self) -> JobResult<Option<JobId>> {
        let mut pending = self.pending_queue.write().unwrap();
        let mut running = self.running_queue.write().unwrap();
        
        if running.len() >= self.max_concurrent {
            return Ok(None);
        }
        
        if let Some(job_id) = pending.pop_front() {
            self.start_job(&job_id)?;
            Ok(Some(job_id))
        } else {
            Ok(None)
        }
    }

    /// Process all pending jobs
    pub fn process_all(&self) -> JobResult<Vec<JobId>> {
        let mut started = Vec::new();
        
        while self.max_concurrent > 0 {
            match self.process_next()? {
                Some(job_id) => started.push(job_id),
                None => break,
            }
        }
        
        Ok(started)
    }

    /// Save all jobs (checkpoint)
    pub fn save_all(&self) -> MalverdeResult<()> {
        if let Some(repo) = &self.repository {
            let jobs = self.jobs.read().unwrap();
            for job in jobs.values() {
                repo.save(job)?;
            }
        }
        self.last_checkpoint = Instant::now();
        Ok(())
    }

    /// Load all jobs from repository
    pub fn load_all(&self) -> MalverdeResult<()> {
        if let Some(repo) = &self.repository {
            let jobs = repo.find_all()?;
            let mut manager_jobs = self.jobs.write().unwrap();
            for job in jobs {
                manager_jobs.insert(job.id.clone(), job);
                
                match job.state {
                    JobState::Pending => {
                        let mut pending = self.pending_queue.write().unwrap();
                        pending.push_back(job.id.clone());
                    }
                    JobState::Running | JobState::Paused => {
                        let mut running = self.running_queue.write().unwrap();
                        running.push_back(job.id.clone());
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// Start background processor
    pub fn start_background_processor(&self) -> JoinHandle<()> {
        let manager = Arc::new(self.clone());
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                
                // Process pending jobs
                let _ = manager.process_all();
                
                // Check for timeouts
                manager.check_timeouts();
                
                // Auto-checkpoint
                manager.auto_checkpoint();
            }
        })
    }

    /// Check for timeout jobs
    fn check_timeouts(&self) {
        let jobs = self.jobs.read().unwrap();
        let running = self.running_queue.read().unwrap();
        
        for job_id in running.iter() {
            if let Some(job) = jobs.get(job_id) {
                if let Some(timeout) = job.timeout {
                    if let Some(started) = job.started_at {
                        if started.elapsed() > timeout {
                            let _ = self.fail_job(job_id, "Job timeout".to_string());
                        }
                    }
                }
            }
        }
    }

    /// Auto checkpoint
    fn auto_checkpoint(&self) {
        if self.last_checkpoint.elapsed() >= self.checkpoint_interval {
            let _ = self.save_all();
        }
    }
}

impl Clone for JobManager {
    fn clone(&self) -> Self {
        Self {
            jobs: Arc::clone(&self.jobs),
            contexts: Arc::clone(&self.contexts),
            pending_queue: Arc::clone(&self.pending_queue),
            running_queue: Arc::clone(&self.running_queue),
            max_concurrent: self.max_concurrent,
            repository: self.repository.clone(),
            checkpoint_interval: self.checkpoint_interval,
            last_checkpoint: self.last_checkpoint,
        }
    }
}

/// Job statistics
#[derive(Debug, Clone)]
pub struct JobStats {
    pub total: usize,
    pub pending: usize,
    pub running: usize,
    pub by_state: HashMap<JobState, usize>,
}

/// Job event types for async processing
#[derive(Debug, Clone)]
pub enum JobEvent {
    JobAdded(JobId),
    JobStarted(JobId),
    JobCompleted(JobId),
    JobFailed(JobId, String),
    JobCancelled(JobId),
    JobProgress(JobId, f32),
}

/// Async job manager with Tokio
#[derive(Debug)]
pub struct AsyncJobManager {
    manager: Arc<JobManager>,
    event_sender: Sender<JobEvent>,
    event_receiver: Receiver<JobEvent>,
}

impl AsyncJobManager {
    pub fn new(max_concurrent: usize) -> Self {
        let (sender, receiver) = channel(100);
        Self {
            manager: Arc::new(JobManager::new(max_concurrent)),
            event_sender: sender,
            event_receiver: receiver,
        }
    }

    pub fn manager(&self) -> Arc<JobManager> {
        Arc::clone(&self.manager)
    }

    pub fn event_sender(&self) -> Sender<JobEvent> {
        self.event_sender.clone()
    }

    pub async fn event_receiver(&mut self) -> Option<JobEvent> {
        self.event_receiver.recv().await
    }

    pub fn start_event_processor(&self) -> JoinHandle<()> {
        let mut receiver = self.event_receiver.clone();
        let manager = Arc::clone(&self.manager);
        
        tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                match event {
                    JobEvent::JobAdded(job_id) => {
                        let _ = manager.process_next();
                    }
                    JobEvent::JobStarted(job_id) => {
                        // Handle job started
                    }
                    JobEvent::JobCompleted(job_id) => {
                        let _ = manager.process_next();
                    }
                    JobEvent::JobFailed(job_id, error) => {
                        // Handle job failed
                    }
                    JobEvent::JobCancelled(job_id) => {
                        let _ = manager.process_next();
                    }
                    JobEvent::JobProgress(job_id, progress) => {
                        // Handle progress update
                    }
                }
            }
        })
    }
}
