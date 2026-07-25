use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    description TEXT NOT NULL,
    session_pid INTEGER,
    session_name TEXT,
    started_at INTEGER NOT NULL,
    finished_at INTEGER
);
CREATE TABLE IF NOT EXISTS claims (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL REFERENCES tasks(id),
    path TEXT NOT NULL,
    claimed_at INTEGER NOT NULL,
    released_at INTEGER,
    forced INTEGER NOT NULL DEFAULT 0
);
";

pub struct TaskRow {
    pub id: i64,
    pub description: String,
    pub session_pid: Option<i64>,
    pub session_name: Option<String>,
    pub started_at: i64,
    pub finished_at: Option<i64>,
}

pub struct ActiveClaim {
    pub path: String,
    pub claimed_at: i64,
    pub forced: bool,
    pub task: TaskRow,
}

pub fn open(root: &Path) -> Result<Connection> {
    let dir = root.join(".agentcoord");
    std::fs::create_dir_all(&dir)?;
    let conn = Connection::open(dir.join("state.db"))?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

fn row_to_task(r: &rusqlite::Row, offset: usize) -> rusqlite::Result<TaskRow> {
    Ok(TaskRow {
        id: r.get(offset)?,
        description: r.get(offset + 1)?,
        session_pid: r.get(offset + 2)?,
        session_name: r.get(offset + 3)?,
        started_at: r.get(offset + 4)?,
        finished_at: r.get(offset + 5)?,
    })
}

const TASK_COLS: &str = "id, description, session_pid, session_name, started_at, finished_at";

pub fn get_task(conn: &Connection, id: i64) -> Result<Option<TaskRow>> {
    let mut stmt = conn.prepare(&format!("SELECT {TASK_COLS} FROM tasks WHERE id = ?1"))?;
    let mut rows = stmt.query_map([id], |r| row_to_task(r, 0))?;
    Ok(rows.next().transpose()?)
}

pub fn active_tasks(conn: &Connection) -> Result<Vec<TaskRow>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {TASK_COLS} FROM tasks WHERE finished_at IS NULL ORDER BY id"
    ))?;
    let rows = stmt.query_map([], |r| row_to_task(r, 0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn active_claims(conn: &Connection) -> Result<Vec<ActiveClaim>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT c.path, c.claimed_at, c.forced, {}
         FROM claims c JOIN tasks t ON t.id = c.task_id
         WHERE c.released_at IS NULL AND t.finished_at IS NULL
         ORDER BY t.id, c.path",
        TASK_COLS
            .split(", ")
            .map(|c| format!("t.{c}"))
            .collect::<Vec<_>>()
            .join(", ")
    ))?;
    let rows = stmt.query_map([], |r| {
        Ok(ActiveClaim {
            path: r.get(0)?,
            claimed_at: r.get(1)?,
            forced: r.get::<_, i64>(2)? != 0,
            task: row_to_task(r, 3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

pub fn fmt_duration(secs: i64) -> String {
    if secs < 60 {
        format!("{secs} seconds")
    } else if secs < 3600 {
        format!("{} minutes", secs / 60)
    } else {
        format!("{} hours", secs / 3600)
    }
}

pub fn fmt_age(secs: i64) -> String {
    format!("{} ago", fmt_duration(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_creates_db_and_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let conn = open(tmp.path()).unwrap();
        assert!(tmp.path().join(".agentcoord/state.db").exists());
        conn.execute(
            "INSERT INTO tasks (description, started_at) VALUES ('x', 1)",
            [],
        )
        .unwrap();
        drop(conn);
        let conn = open(tmp.path()).unwrap(); // re-open must not wipe data
        let t = get_task(&conn, 1).unwrap().unwrap();
        assert_eq!(t.description, "x");
        assert!(t.finished_at.is_none());
    }

    #[test]
    fn active_queries_filter_finished_and_released() {
        let tmp = tempfile::tempdir().unwrap();
        let conn = open(tmp.path()).unwrap();
        conn.execute("INSERT INTO tasks (description, started_at) VALUES ('a', 1)", []).unwrap();
        conn.execute("INSERT INTO tasks (description, started_at, finished_at) VALUES ('b', 1, 2)", []).unwrap();
        conn.execute("INSERT INTO claims (task_id, path, claimed_at) VALUES (1, 'x.rs', 1)", []).unwrap();
        conn.execute("INSERT INTO claims (task_id, path, claimed_at, released_at) VALUES (1, 'y.rs', 1, 2)", []).unwrap();
        conn.execute("INSERT INTO claims (task_id, path, claimed_at) VALUES (2, 'z.rs', 1)", []).unwrap();
        assert_eq!(active_tasks(&conn).unwrap().len(), 1);
        let claims = active_claims(&conn).unwrap();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].path, "x.rs");
        assert_eq!(claims[0].task.description, "a");
        assert!(!claims[0].forced);
    }

    #[test]
    fn time_formatting() {
        assert_eq!(fmt_duration(5), "5 seconds");
        assert_eq!(fmt_duration(120), "2 minutes");
        assert_eq!(fmt_duration(7300), "2 hours");
        assert_eq!(fmt_age(120), "2 minutes ago");
        assert!(now() > 1_700_000_000);
    }
}
