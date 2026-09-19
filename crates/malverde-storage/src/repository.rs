use std::path::Path;
use std::sync::Arc;

use malverde_core::error::MalverdeError;
use malverde_core::types::*;

use super::connection::StorageConnection;
use super::models::*;

#[derive(Debug, Clone)]
pub struct Storage {
    conn: StorageConnection,
}

impl Storage {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, MalverdeError> {
        let conn = StorageConnection::new(path)?;
        Migrations::apply(&conn)?;
        Ok(Self { conn })
    }

    pub async fn save_knowledge(&self, knowledge: &Knowledge) -> Result<(), MalverdeError> {
        let tags_json = serde_json::to_string(&knowledge.tags)
            .map_err(|e| MalverdeError::SerializationError(e.to_string()))?;
        let metadata_json = serde_json::to_string(&knowledge.metadata)
            .map_err(|e| MalverdeError::SerializationError(e.to_string()))?;

        self.conn.execute(
            "INSERT OR REPLACE INTO knowledge (id, title, content, tags, metadata, created_at, updated_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            [
                knowledge.id.to_string(),
                knowledge.title.clone(),
                knowledge.content.clone(),
                tags_json,
                metadata_json,
                knowledge.created_at,
                knowledge.updated_at,
            ],
        )?;
        Ok(())
    }

    pub async fn get_knowledge(&self, id: KnowledgeId) -> Result<Option<Knowledge>, MalverdeError> {
        let result: Option<Knowledge> = self.conn.query_map(
            "SELECT id, title, content, tags, metadata, created_at, updated_at 
             FROM knowledge WHERE id = ?",
            [id.to_string()],
            |row| {
                Ok(Knowledge {
                    id: KnowledgeId(uuid::Uuid::parse_str(row.get::<_, String>(0)?).unwrap()),
                    title: row.get(1)?,
                    content: row.get(2)?,
                    tags: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
                    metadata: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )?;
        Ok(result)
    }

    pub async fn delete_knowledge(&self, id: KnowledgeId) -> Result<(), MalverdeError> {
        self.conn.execute("DELETE FROM knowledge WHERE id = ?", [id.to_string()])?;
        Ok(())
    }

    pub async fn search_knowledge(&self, query: &str) -> Result<Vec<Knowledge>, MalverdeError> {
        let results: Vec<Knowledge> = self.conn.query_map(
            "SELECT k.id, k.title, k.content, k.tags, k.metadata, k.created_at, k.updated_at 
             FROM knowledge k 
             JOIN fts_knowledge f ON k.id = f.rowid 
             WHERE f MATCH ?",
            [format!("'{}'", query)],
            |row| {
                Ok(Knowledge {
                    id: KnowledgeId(uuid::Uuid::parse_str(row.get::<_, String>(0)?).unwrap()),
                    title: row.get(1)?,
                    content: row.get(2)?,
                    tags: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
                    metadata: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )?;
        Ok(results)
    }

    pub async fn save_memory(&self, memory: &Memory) -> Result<(), MalverdeError> {
        let value_json = serde_json::to_string(&memory.value)
            .map_err(|e| MalverdeError::SerializationError(e.to_string()))?;
        self.conn.execute(
            "INSERT OR REPLACE INTO memory (id, key, value, session_id, expires_at, created_at, updated_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            [
                memory.id.to_string(),
                memory.key.clone(),
                value_json,
                memory.session_id.clone().unwrap_or_default(),
                memory.expires_at.unwrap_or(0),
                memory.created_at,
                memory.updated_at,
            ],
        )?;
        Ok(())
    }

    pub async fn get_memory(&self, key: &str) -> Result<Option<Memory>, MalverdeError> {
        let result: Option<Memory> = self.conn.query_map(
            "SELECT id, key, value, session_id, expires_at, created_at, updated_at 
             FROM memory WHERE key = ?",
            [key],
            |row| {
                Ok(Memory {
                    id: MemoryId(uuid::Uuid::parse_str(row.get::<_, String>(0)?).unwrap()),
                    key: row.get(1)?,
                    value: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                    session_id: row.get::<_, Option<String>>(3)?,
                    expires_at: row.get::<_, Option<i64>>(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )?;
        Ok(result)
    }

    pub async fn delete_memory(&self, id: MemoryId) -> Result<(), MalverdeError> {
        self.conn.execute("DELETE FROM memory WHERE id = ?", [id.to_string()])?;
        Ok(())
    }

    pub async fn save_event(&self, event: &StoredEvent) -> Result<(), MalverdeError> {
        let data_json = serde_json::to_string(&event.data)
            .map_err(|e| MalverdeError::SerializationError(e.to_string()))?;
        self.conn.execute(
            "INSERT INTO events (id, event_type, data, timestamp, processed) 
             VALUES (?, ?, ?, ?, ?)",
            [
                event.id.to_string(),
                event.event_type.clone(),
                data_json,
                event.timestamp,
                event.processed as i32,
            ],
        )?;
        Ok(())
    }

    pub async fn get_unprocessed_events(&self) -> Result<Vec<StoredEvent>, MalverdeError> {
        let results: Vec<StoredEvent> = self.conn.query_map(
            "SELECT id, event_type, data, timestamp, processed 
             FROM events WHERE processed = 0",
            [],
            |row| {
                Ok(StoredEvent {
                    id: EventId(uuid::Uuid::parse_str(row.get::<_, String>(0)?).unwrap()),
                    event_type: row.get(1)?,
                    data: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                    timestamp: row.get(3)?,
                    processed: row.get::<_, i32>(4)? != 0,
                })
            },
        )?;
        Ok(results)
    }

    pub async fn mark_event_processed(&self, id: EventId) -> Result<(), MalverdeError> {
        self.conn.execute("UPDATE events SET processed = 1 WHERE id = ?", [id.to_string()])?;
        Ok(())
    }

    pub async fn save_job(&self, job: &Job) -> Result<(), MalverdeError> {
        let data_json = serde_json::to_string(&job.data)
            .map_err(|e| MalverdeError::SerializationError(e.to_string()))?;
        let status_str = format!("{:?}", job.status);
        let priority_str = format!("{:?}", job.priority);
        self.conn.execute(
            "INSERT OR REPLACE INTO jobs (id, name, description, status, priority, data, checkpoint_id, created_at, updated_at, started_at, completed_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                job.id.to_string(),
                job.name.clone(),
                job.description.clone().unwrap_or_default(),
                status_str,
                priority_str,
                data_json,
                job.checkpoint_id.map(|c| c.to_string()).unwrap_or_default(),
                job.created_at,
                job.updated_at,
                job.started_at.unwrap_or(0),
                job.completed_at.unwrap_or(0),
            ],
        )?;
        Ok(())
    }

    pub async fn get_job(&self, id: JobId) -> Result<Option<Job>, MalverdeError> {
        let result: Option<Job> = self.conn.query_map(
            "SELECT id, name, description, status, priority, data, checkpoint_id, created_at, updated_at, started_at, completed_at 
             FROM jobs WHERE id = ?",
            [id.to_string()],
            |row| {
                let status_str: String = row.get(3)?;
                let priority_str: String = row.get(4)?;
                Ok(Job {
                    id: JobId(uuid::Uuid::parse_str(row.get::<_, String>(0)?).unwrap()),
                    name: row.get(1)?,
                    description: row.get::<_, Option<String>>(2)?,
                    status: match status_str.as_str() {
                        "Pending" => JobStatus::Pending,
                        "Running" => JobStatus::Running,
                        "Paused" => JobStatus::Paused,
                        "Completed" => JobStatus::Completed,
                        "Failed" => JobStatus::Failed,
                        _ => JobStatus::Pending,
                    },
                    priority: match priority_str.as_str() {
                        "Low" => JobPriority::Low,
                        "Normal" => JobPriority::Normal,
                        "High" => JobPriority::High,
                        "Critical" => JobPriority::Critical,
                        _ => JobPriority::Normal,
                    },
                    data: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    checkpoint_id: row.get::<_, Option<String>>(6)?.map(|s| CheckpointId(uuid::Uuid::parse_str(&s).unwrap())),
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    started_at: row.get::<_, Option<i64>>(9)?,
                    completed_at: row.get::<_, Option<i64>>(10)?,
                })
            },
        )?;
        Ok(result)
    }

    pub async fn save_checkpoint(&self, checkpoint: &Checkpoint) -> Result<(), MalverdeError> {
        let state_json = serde_json::to_string(&checkpoint.state)
            .map_err(|e| MalverdeError::SerializationError(e.to_string()))?;
        self.conn.execute(
            "INSERT OR REPLACE INTO checkpoints (id, job_id, state, created_at) 
             VALUES (?, ?, ?, ?)",
            [
                checkpoint.id.to_string(),
                checkpoint.job_id.to_string(),
                state_json,
                checkpoint.created_at,
            ],
        )?;
        Ok(())
    }

    pub async fn get_latest_checkpoint(&self, job_id: JobId) -> Result<Option<Checkpoint>, MalverdeError> {
        let result: Option<Checkpoint> = self.conn.query_map(
            "SELECT id, job_id, state, created_at FROM checkpoints WHERE job_id = ? ORDER BY created_at DESC LIMIT 1",
            [job_id.to_string()],
            |row| {
                Ok(Checkpoint {
                    id: CheckpointId(uuid::Uuid::parse_str(row.get::<_, String>(0)?).unwrap()),
                    job_id: JobId(uuid::Uuid::parse_str(row.get::<_, String>(1)?).unwrap()),
                    state: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                    created_at: row.get(3)?,
                })
            },
        )?;
        Ok(result)
    }

    pub async fn cleanup_expired_memories(&self) -> Result<(), MalverdeError> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute("DELETE FROM memory WHERE expires_at IS NOT NULL AND expires_at < ?", [now])?;
        Ok(())
    }
}