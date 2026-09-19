use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::Arc;

use malverde_core::error::MalverdeError;

#[derive(Debug, Clone)]
pub struct StorageConnection {
    inner: Arc<rusqlite::Connection>,
}

impl StorageConnection {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, MalverdeError> {
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_URI;

        let conn = Connection::open_with_flags(path, flags)
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;

        conn.execute_batch("CREATE VIRTUAL TABLE IF NOT EXISTS fts_knowledge USING fts5(title, content)")
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;

        Ok(Self {
            inner: Arc::new(conn),
        })
    }

    pub fn execute(&self, query: &str, params: &[&dyn rusqlite::ToSql]) -> Result<(), MalverdeError> {
        self.inner
            .execute(query, params)
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        Ok(())
    }

    pub fn query_map<T, F>(
        &self,
        query: &str,
        params: &[&dyn rusqlite::ToSql],
        mut map_fn: F,
    ) -> Result<Vec<T>, MalverdeError>
    where
        F: FnMut(&rusqlite::Row) -> Result<T, rusqlite::Error>,
    {
        let mut stmt = self.inner
            .prepare(query)
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;

        let rows = stmt
            .query_map(params, |row| map_fn(row))
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;

        rows.collect::<Result<Vec<T>, _>>()
            .map_err(|e| MalverdeError::StorageError(e.to_string()))
    }

    pub fn execute_batch(&self, query: &str) -> Result<(), MalverdeError> {
        self.inner
            .execute_batch(query)
            .map_err(|e| MalverdeError::StorageError(e.to_string()))?;
        Ok(())
    }

    pub fn as_inner(&self) -> Arc<rusqlite::Connection> {
        self.inner.clone()
    }
}

impl std::ops::Deref for StorageConnection {
    type Target = Arc<rusqlite::Connection>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}