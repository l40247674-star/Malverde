use crate::audit::{AuditLog, AuditAction, AuditQuery, AuditSummary, AuditStorage};
use malverde_core::{AuditId, ActorId, ProjectId, OperationId, AuditState, MalverdeResult};
use std::sync::{Arc, RwLock};
use std::collections::{VecDeque, HashMap};
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct AuditLogger {
    storage: Option<Arc<dyn AuditStorage + Send + Sync>>,
    buffer: Arc<RwLock<VecDeque<AuditLog>>>,
    max_buffer_size: usize,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self {
            storage: None,
            buffer: Arc::new(RwLock::new(VecDeque::new())),
            max_buffer_size: 1000,
        }
    }

    pub fn with_storage(storage: Arc<dyn AuditStorage + Send + Sync>) -> Self {
        Self { storage: Some(storage), ..Self::new() }
    }

    pub fn log(&self, mut log: AuditLog) -> MalverdeResult<AuditId> {
        if log.id == AuditId::default() {
            log.id = AuditId::new();
        }
        self.add_to_buffer(log.clone())?;
        if let Some(storage) = &self.storage {
            storage.save(&log)?;
        }
        Ok(log.id.clone())
    }

    pub fn log_action(&self, action: AuditAction, resource_type: String, resource_id: String) -> MalverdeResult<AuditId> {
        let log = AuditLog::new(action, resource_type, resource_id);
        self.log(log)
    }

    pub fn add_to_buffer(&self, log: AuditLog) -> MalverdeResult<()> {
        let mut buffer = self.buffer.write().unwrap();
        if buffer.len() >= self.max_buffer_size {
            buffer.pop_front();
        }
        buffer.push_back(log);
        Ok(())
    }

    pub fn get_log(&self, id: &AuditId) -> MalverdeResult<Option<AuditLog>> {
        {
            let buffer = self.buffer.read().unwrap();
            for log in buffer.iter() {
                if &log.id == id { return Ok(Some(log.clone())); }
            }
        }
        if let Some(storage) = &self.storage { return storage.find_by_id(id); }
        Ok(None)
    }

    pub fn get_recent_logs(&self, limit: usize) -> Vec<AuditLog> {
        let buffer = self.buffer.read().unwrap();
        buffer.iter().rev().take(limit).cloned().collect()
    }

    pub fn set_max_buffer_size(&mut self, size: usize) { self.max_buffer_size = size; }
}

#[derive(Debug, Clone)]
pub struct MemoryAuditStorage {
    logs: Arc<RwLock<Vec<AuditLog>>>,
}

impl MemoryAuditStorage {
    pub fn new() -> Self { Self { logs: Arc::new(RwLock::new(Vec::new())) } }
}

impl AuditStorage for MemoryAuditStorage {
    fn save(&self, log: &AuditLog) -> Result<(), Box<dyn std::error::Error>> {
        let mut logs = self.logs.write().unwrap();
        logs.push(log.clone());
        Ok(())
    }

    fn find_by_id(&self, id: &AuditId) -> Result<Option<AuditLog>, Box<dyn std::error::Error>> {
        let logs = self.logs.read().unwrap();
        Ok(logs.iter().find(|log| &log.id == id).cloned())
    }

    fn query(&self, query: &AuditQuery) -> Result<Vec<AuditLog>, Box<dyn std::error::Error>> {
        let logs = self.logs.read().unwrap();
        Ok(logs.iter().filter(|_| true).cloned().collect())
    }

    fn get_summary(&self, _query: &AuditQuery) -> Result<AuditSummary, Box<dyn std::error::Error>> {
        let logs = self.logs.read().unwrap();
        Ok(AuditSummary {
            total_logs: logs.len(),
            by_action: HashMap::new(),
            by_resource_type: HashMap::new(),
            by_actor: HashMap::new(),
            by_project: HashMap::new(),
            success_count: 0,
            failure_count: 0,
            by_state: HashMap::new(),
        })
    }

    fn delete_by_id(&self, id: &AuditId) -> Result<(), Box<dyn std::error::Error>> {
        let mut logs = self.logs.write().unwrap();
        logs.retain(|log| &log.id != id);
        Ok(())
    }

    fn delete_by_query(&self, _query: &AuditQuery) -> Result<usize, Box<dyn std::error::Error>> {
        Ok(0)
    }
}
