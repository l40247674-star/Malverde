use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{params, Connection};

use malverde_core::{MalverdeError, MemoryFact, Message, Result, Role, SessionId, SessionMeta};

/// Almacén persistente de sesiones, mensajes y hechos de memoria (SQLite + FTS5).
pub struct MemoryStore {
    conn: Mutex<Connection>,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL DEFAULT 'Nueva sesión',
    model TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    role TEXT NOT NULL,
    content TEXT,
    tool_name TEXT,
    tool_call_id TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS memory (
    id TEXT PRIMARY KEY,
    fact TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, id);
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content, content='messages', content_rowid='id'
);
CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (new.id, new.content);
END;
CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
    fact, content='memory', content_rowid='rowid'
);
CREATE TRIGGER IF NOT EXISTS memory_ai AFTER INSERT ON memory BEGIN
    INSERT INTO memory_fts(rowid, fact) VALUES (new.rowid, new.fact);
END;
CREATE TRIGGER IF NOT EXISTS memory_ad AFTER DELETE ON memory BEGIN
    INSERT INTO memory_fts(memory_fts, rowid, fact) VALUES ('delete', old.rowid, old.fact);
END;
"#;

impl MemoryStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(MalverdeError::io)?;
        }
        let conn = Connection::open(path).map_err(|e| MalverdeError::storage(e.to_string()))?;
        conn.execute_batch(SCHEMA).map_err(|e| MalverdeError::storage(e.to_string()))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| MalverdeError::storage(e.to_string()))?;
        conn.execute_batch(SCHEMA).map_err(|e| MalverdeError::storage(e.to_string()))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    // ----------------------------------------------------------------- sesiones

    pub fn create_session(&self, id: &SessionId, model: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO sessions (id, title, model, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id.0, "Nueva sesión", model, now, now],
        )
        .map_err(db_err)?;
        Ok(())
    }

    pub fn append_message(&self, session_id: &SessionId, message: &Message) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO messages (session_id, role, content, tool_name, tool_call_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session_id.0,
                message.role.as_str(),
                message.content,
                message.name,
                message.tool_call_id,
                now
            ],
        )
        .map_err(db_err)?;

        // Auto-título con el primer mensaje del usuario.
        if message.role == Role::User {
            let title: String = message.content.as_deref().unwrap_or("Sesión").chars().take(60).collect();
            conn.execute(
                "UPDATE sessions SET title = ?2, updated_at = ?3 WHERE id = ?1 AND title = 'Nueva sesión'",
                params![session_id.0, title, now],
            )
            .map_err(db_err)?;
        } else {
            conn.execute("UPDATE sessions SET updated_at = ?2 WHERE id = ?1", params![session_id.0, now])
                .map_err(db_err)?;
        }
        Ok(())
    }

    pub fn session_messages(&self, session_id: &SessionId, limit: usize) -> Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT role, content, tool_name, tool_call_id FROM messages
                 WHERE session_id = ?1 ORDER BY id ASC LIMIT ?2",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![session_id.0, limit as i64], |row| {
                let role: String = row.get(0)?;
                let content: Option<String> = row.get(1)?;
                let tool_name: Option<String> = row.get(2)?;
                let tool_call_id: Option<String> = row.get(3)?;
                Ok((role, content, tool_name, tool_call_id))
            })
            .map_err(db_err)?;
        let mut out = Vec::new();
        for row in rows {
            let (role, content, tool_name, tool_call_id) = row.map_err(db_err)?;
            let role = match role.as_str() {
                "system" => Role::System,
                "assistant" => Role::Assistant,
                "tool" => Role::Tool,
                _ => Role::User,
            };
            out.push(Message {
                role,
                content,
                tool_calls: vec![],
                tool_call_id,
                name: tool_name,
            });
        }
        Ok(out)
    }

    pub fn list_sessions(&self, limit: usize) -> Result<Vec<SessionMeta>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.title, s.model, s.created_at, s.updated_at,
                        (SELECT COUNT(*) FROM messages m WHERE m.session_id = s.id)
                 FROM sessions s ORDER BY s.updated_at DESC LIMIT ?1",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(db_err)?;
        let mut out = Vec::new();
        for row in rows {
            let (id, title, model, created_at, updated_at, message_count) = row.map_err(db_err)?;
            out.push(SessionMeta {
                id: SessionId(id),
                title,
                model,
                created_at: parse_dt(&created_at),
                updated_at: parse_dt(&updated_at),
                message_count: message_count as usize,
            });
        }
        Ok(out)
    }

    /// Búsqueda de texto completo en todos los mensajes históricos (recall cross-sesión).
    pub fn search_messages(&self, query: &str, limit: usize) -> Result<Vec<(String, String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT m.session_id, m.role, m.content FROM messages_fts f
                 JOIN messages m ON m.id = f.rowid
                 WHERE messages_fts MATCH ?1
                 ORDER BY rank LIMIT ?2",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![query, limit as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .map_err(db_err)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(db_err)?);
        }
        Ok(out)
    }

    pub fn delete_session(&self, session_id: &SessionId) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n = conn
            .execute("DELETE FROM messages WHERE session_id = ?1", params![session_id.0])
            .map_err(db_err)?;
        let s = conn
            .execute("DELETE FROM sessions WHERE id = ?1", params![session_id.0])
            .map_err(db_err)?;
        Ok(n > 0 || s > 0)
    }

    pub fn rename_session(&self, session_id: &SessionId, title: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET title = ?2 WHERE id = ?1",
            params![session_id.0, title],
        )
        .map_err(db_err)?;
        Ok(())
    }

    // ----------------------------------------------------------------- memoria

    pub fn remember(&self, fact: &str, tags: &[String]) -> Result<MemoryFact> {
        let id = malverde_core::SessionId::new().0;
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO memory (id, fact, tags, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, fact, tags.join(","), now, now],
        )
        .map_err(db_err)?;
        Ok(MemoryFact { id, fact: fact.to_string(), tags: tags.to_vec(), created_at: parse_dt(&now) })
    }

    pub fn forget(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute("DELETE FROM memory WHERE id = ?1", params![id]).map_err(db_err)?;
        Ok(n > 0)
    }

    pub fn list_facts(&self, limit: usize) -> Result<Vec<MemoryFact>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, fact, tags, created_at FROM memory ORDER BY created_at DESC LIMIT ?1")
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![limit as i64], map_fact)
            .map_err(db_err)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(db_err)?);
        }
        Ok(out)
    }

    pub fn search_facts(&self, query: &str, limit: usize) -> Result<Vec<MemoryFact>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT m.id, m.fact, m.tags, m.created_at FROM memory_fts f
                 JOIN memory m ON m.rowid = f.rowid
                 WHERE memory_fts MATCH ?1 ORDER BY rank LIMIT ?2",
            )
            .map_err(db_err)?;
        let rows = stmt.query_map(params![query, limit as i64], map_fact).map_err(db_err)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(db_err)?);
        }
        Ok(out)
    }

    /// Los hechos más recientes, para inyectarlos en el system prompt.
    pub fn recent_facts(&self, n: usize) -> Result<Vec<MemoryFact>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, fact, tags, created_at FROM memory ORDER BY created_at DESC LIMIT ?1")
            .map_err(db_err)?;
        let rows = stmt.query_map(params![n as i64], map_fact).map_err(db_err)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(db_err)?);
        }
        Ok(out)
    }
}

fn map_fact(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryFact> {
    let id: String = row.get(0)?;
    let fact: String = row.get(1)?;
    let tags: String = row.get(2)?;
    let created_at: String = row.get(3)?;
    Ok(MemoryFact {
        id,
        fact,
        tags: tags.split(',').filter(|t| !t.is_empty()).map(String::from).collect(),
        created_at: parse_dt(&created_at),
    })
}

fn parse_dt(raw: &str) -> chrono::DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn db_err(e: rusqlite::Error) -> MalverdeError {
    MalverdeError::storage(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_and_fts_work() {
        let store = MemoryStore::open_in_memory().unwrap();
        let sid = SessionId::new();
        store.create_session(&sid, "test-model").unwrap();
        store.append_message(&sid, &Message::user("el proyecto Malverde usa Rust")).unwrap();
        store.append_message(&sid, &Message::assistant("entendido")).unwrap();

        let msgs = store.session_messages(&sid, 50).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, Role::User);

        let sessions = store.list_sessions(10).unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(sessions[0].title.contains("Malverde"));

        let hits = store.search_messages("Malverde", 5).unwrap();
        assert!(!hits.is_empty());
        assert!(store.delete_session(&sid).unwrap());
    }

    #[test]
    fn facts_remember_forget_search() {
        let store = MemoryStore::open_in_memory().unwrap();
        let fact = store.remember("El usuario prefiere Rust", &["preferencias".into()]).unwrap();
        assert_eq!(store.list_facts(10).unwrap().len(), 1);
        let hits = store.search_facts("Rust", 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(store.forget(&fact.id).unwrap());
        assert!(store.list_facts(10).unwrap().is_empty());
    }
}
