use rusqlite::{Connection, Result};
use std::sync::Arc;

use malverde_core::error::MalverdeError;
use super::connection::StorageConnection;

pub struct Migrations;

impl Migrations {
    pub fn apply(conn: &StorageConnection) -> Result<(), MalverdeError> {
        let inner = conn.as_inner();
        
        inner
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS migrations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE,
                    applied_at INTEGER NOT NULL
                )")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;

        let migrations = vec![
            ("001_create_knowledge_table", Self::create_knowledge_table),
            ("002_create_memory_table", Self::create_memory_table),
            ("003_create_events_table", Self::create_events_table),
            ("004_create_jobs_table", Self::create_jobs_table),
            ("005_create_checkpoints_table", Self::create_checkpoints_table),
            ("006_create_audit_table", Self::create_audit_table),
            ("007_create_evidence_table", Self::create_evidence_table),
            ("008_create_plugins_table", Self::create_plugins_table),
        ];

        for (name, migration_fn) in migrations {
            let exists: bool = inner
                .query_row(
                    "SELECT 1 FROM migrations WHERE name = ?",
                    [name],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if !exists {
                migration_fn(&inner)?;
                inner
                    .execute(
                        "INSERT INTO migrations (name, applied_at) VALUES (?, ?)",
                        [name, chrono::Utc::now().timestamp()],
                    )
                    .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
                tracing::info!("Migration applied: {}", name);
            }
        }

        Ok(())
    }

    fn create_knowledge_table(conn: &Arc<Connection>) -> Result<(), MalverdeError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS knowledge (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '[]',
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        Ok(())
    }

    fn create_memory_table(conn: &Arc<Connection>) -> Result<(), MalverdeError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memory (
                id TEXT PRIMARY KEY,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                session_id TEXT,
                expires_at INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_memory_key ON memory(key)")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        Ok(())
    }

    fn create_events_table(conn: &Arc<Connection>) -> Result<(), MalverdeError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                data TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                processed INTEGER NOT NULL DEFAULT 0
            )")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type)")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        Ok(())
    }

    fn create_jobs_table(conn: &Arc<Connection>) -> Result<(), MalverdeError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                priority TEXT NOT NULL,
                data TEXT NOT NULL DEFAULT '{}',
                checkpoint_id TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                started_at INTEGER,
                completed_at INTEGER,
                FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(id)
            )")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status)")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        Ok(())
    }

    fn create_checkpoints_table(conn: &Arc<Connection>) -> Result<(), MalverdeError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS checkpoints (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL,
                state TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (job_id) REFERENCES jobs(id)
            )")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        Ok(())
    }

    fn create_audit_table(conn: &Arc<Connection>) -> Result<(), MalverdeError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audit (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                user TEXT,
                timestamp INTEGER NOT NULL,
                details TEXT NOT NULL DEFAULT '{}'
            )")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_audit_entity ON audit(entity_type, entity_id)")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        Ok(())
    }

    fn create_evidence_table(conn: &Arc<Connection>) -> Result<(), MalverdeError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS evidence (
                id TEXT PRIMARY KEY,
                knowledge_id TEXT NOT NULL,
                data BLOB NOT NULL,
                mime_type TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (knowledge_id) REFERENCES knowledge(id)
            )")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        Ok(())
    }

    fn create_plugins_table(conn: &Arc<Connection>) -> Result<(), MalverdeError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS plugins (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                description TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                path TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_plugins_enabled ON plugins(enabled)")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        Ok(())
    }
}