//! Local SQLite chat session persistence (separate from read-only LanceDB).

use std::path::Path;

use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::models::{ChatMessageDto, ChatSessionDto, LegalReference};

pub struct ChatDb {
    conn: Connection,
}

impl ChatDb {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS sessions (
                  id TEXT PRIMARY KEY,
                  title TEXT NOT NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS messages (
                  id TEXT PRIMARY KEY,
                  session_id TEXT NOT NULL,
                  role TEXT NOT NULL,
                  content TEXT NOT NULL,
                  created_at TEXT NOT NULL,
                  references_json TEXT NOT NULL DEFAULT '[]',
                  FOREIGN KEY(session_id) REFERENCES sessions(id)
                );
                CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, created_at);
                "#,
            )
            .map_err(|e| e.to_string())?;

        // Existing DBs created before citations were persisted lack this column;
        // CREATE TABLE IF NOT EXISTS does not alter them.
        self.ensure_column("messages", "references_json", "TEXT NOT NULL DEFAULT '[]'")?;
        Ok(())
    }

    fn ensure_column(&self, table: &str, column: &str, decl: &str) -> Result<(), String> {
        if self.column_exists(table, column)? {
            return Ok(());
        }
        tracing::info!(
            table,
            column,
            "Migrating chat SQLite schema: adding missing column"
        );
        self.conn
            .execute(
                &format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"),
                [],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn column_exists(&self, table: &str, column: &str) -> Result<bool, String> {
        // PRAGMA table_info cannot take bound parameters for the table name.
        let mut stmt = self
            .conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(|e| e.to_string())?;
        let names = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| e.to_string())?;
        for name in names {
            if name.map_err(|e| e.to_string())? == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn list_sessions(&self) -> Result<Vec<ChatSessionDto>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, title, created_at, updated_at FROM sessions ORDER BY updated_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ChatSessionDto {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn get_history(&self, session_id: &str) -> Result<Vec<ChatMessageDto>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, session_id, role, content, created_at, references_json
                 FROM messages WHERE session_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![session_id], |row| {
                let refs_json: String = row.get(5)?;
                let references: Vec<LegalReference> =
                    serde_json::from_str(&refs_json).unwrap_or_default();
                Ok(ChatMessageDto {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    created_at: row.get(4)?,
                    references,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn ensure_session(&self, session_id: Option<String>, title_hint: &str) -> Result<String, String> {
        if let Some(id) = session_id {
            let exists: bool = self
                .conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sessions WHERE id = ?1)",
                    params![id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            if exists {
                return Ok(id);
            }
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let title = title_hint.chars().take(80).collect::<String>();
        let title = if title.trim().is_empty() {
            "New chat".into()
        } else {
            title
        };
        self.conn
            .execute(
                "INSERT INTO sessions (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![id, title, now, now],
            )
            .map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub fn append_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        references: &[LegalReference],
    ) -> Result<ChatMessageDto, String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let refs_json = serde_json::to_string(references).unwrap_or_else(|_| "[]".into());
        self.conn
            .execute(
                "INSERT INTO messages (id, session_id, role, content, created_at, references_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, session_id, role, content, now, refs_json],
            )
            .map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
                params![now, session_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(ChatMessageDto {
            id,
            session_id: session_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            created_at: now,
            references: references.to_vec(),
        })
    }
}
