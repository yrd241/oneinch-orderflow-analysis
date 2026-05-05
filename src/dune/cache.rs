use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde_json::Value;

use crate::model::QueryKind;

pub struct Cache {
    conn: Connection,
}

impl Cache {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                tracing::warn!("failed to create cache dir {}: {e}", parent.display());
            }
        }
        let conn = Connection::open(path).with_context(|| format!("open sqlite {}", path.display()))?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS raw_rows (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                payload TEXT NOT NULL,
                ingested_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_raw_kind ON raw_rows(kind);
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn insert_rows(&mut self, kind: QueryKind, rows: &[Value]) -> Result<()> {
        let kind_str = kind.as_str();
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM raw_rows WHERE kind = ?1", params![kind_str])?;
        for row in rows {
            let payload = serde_json::to_string(row)?;
            tx.execute(
                "INSERT INTO raw_rows (kind, payload) VALUES (?1, ?2)",
                params![kind_str, payload],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_kind(&self, kind: QueryKind) -> Result<Vec<Value>> {
        let kind_str = kind.as_str();
        let mut stmt = self
            .conn
            .prepare("SELECT payload FROM raw_rows WHERE kind = ?1")?;
        let rows = stmt
            .query_map(params![kind_str], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        rows.into_iter()
            .map(|s| serde_json::from_str(&s).context("deserialize cached row"))
            .collect()
    }
}
