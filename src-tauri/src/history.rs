//! Local test history in SQLite: every saved session (controller, date,
//! kind, score, summary and the full result payload as JSON).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

pub const KINDS: [&str; 8] = [
    "test",
    "buttons",
    "buttonHealth",
    "calibration",
    "latency",
    "ghosting",
    "vibration",
    "firmware",
];
const MAX_PAYLOAD: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewSession {
    pub kind: String,
    pub controller_name: String,
    pub controller_key: String,
    pub score: Option<f64>,
    pub summary: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: i64,
    pub created_at: String,
    pub kind: String,
    pub controller_name: String,
    pub controller_key: String,
    pub score: Option<f64>,
    pub summary: String,
    pub payload: serde_json::Value,
}

pub struct History {
    conn: Mutex<Connection>,
}

impl History {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        Self::init(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS sessions (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 created_at TEXT NOT NULL,
                 kind TEXT NOT NULL,
                 controller_name TEXT NOT NULL,
                 controller_key TEXT NOT NULL,
                 score REAL,
                 summary TEXT NOT NULL,
                 payload TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS sessions_created ON sessions(created_at DESC);
             PRAGMA user_version = 1;",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn add(&self, s: &NewSession) -> Result<i64, String> {
        if !KINDS.contains(&s.kind.as_str()) {
            return Err(format!("unknown session kind {}", s.kind));
        }
        let payload = serde_json::to_string(&s.payload).map_err(|e| e.to_string())?;
        if payload.len() > MAX_PAYLOAD {
            return Err("session payload too large".into());
        }
        let summary: String = s.summary.chars().take(500).collect();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (created_at, kind, controller_name, controller_key, score, summary, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                gamepadlab_profiles::now_rfc3339(),
                s.kind,
                s.controller_name.chars().take(120).collect::<String>(),
                s.controller_key.chars().take(120).collect::<String>(),
                s.score.filter(|v| v.is_finite()),
                summary,
                payload
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list(&self, limit: u32) -> Result<Vec<Session>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, created_at, kind, controller_name, controller_key, score, summary, payload
                 FROM sessions ORDER BY id DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit.clamp(1, 5000)], |r| {
                let payload: String = r.get(7)?;
                Ok(Session {
                    id: r.get(0)?,
                    created_at: r.get(1)?,
                    kind: r.get(2)?,
                    controller_name: r.get(3)?,
                    controller_key: r.get(4)?,
                    score: r.get(5)?,
                    summary: r.get(6)?,
                    payload: serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null),
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn delete(&self, id: i64) -> Result<bool, String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])
            .map(|n| n > 0)
            .map_err(|e| e.to_string())
    }

    pub fn clear(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM sessions", [])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn session(kind: &str) -> NewSession {
        NewSession {
            kind: kind.into(),
            controller_name: "DualSense".into(),
            controller_key: "054c:0ce6".into(),
            score: Some(87.0),
            summary: "19 passed".into(),
            payload: json!({ "results": [1, 2, 3] }),
        }
    }

    #[test]
    fn add_list_delete_clear() {
        let h = History::in_memory().unwrap();
        let a = h.add(&session("buttons")).unwrap();
        let b = h.add(&session("calibration")).unwrap();
        let list = h.list(10).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, b, "newest first");
        assert_eq!(list[1].payload["results"][2], 3);
        assert!(h.delete(a).unwrap());
        assert!(!h.delete(a).unwrap());
        assert_eq!(h.list(10).unwrap().len(), 1);
        h.clear().unwrap();
        assert!(h.list(10).unwrap().is_empty());
    }

    #[test]
    fn rejects_unknown_kind_and_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite");
        {
            let h = History::open(&path).unwrap();
            assert!(h.add(&session("bogus")).is_err());
            h.add(&session("latency")).unwrap();
        }
        let h = History::open(&path).unwrap();
        assert_eq!(h.list(5).unwrap()[0].kind, "latency");
    }

    #[test]
    fn non_finite_score_is_stored_as_null() {
        let h = History::in_memory().unwrap();
        let mut s = session("test");
        s.score = Some(f64::NAN);
        h.add(&s).unwrap();
        assert_eq!(h.list(1).unwrap()[0].score, None);
    }
}
