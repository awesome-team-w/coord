use crate::{db, session};
use anyhow::{bail, Result};
use std::path::Path;

pub fn start(root: &Path, description: &str) -> Result<()> {
    let desc = description.trim();
    if desc.is_empty() {
        bail!("task description must not be empty");
    }
    let conn = db::open(root)?;
    let agent = session::detect_agent_process();
    let (pid, name) = match &agent {
        Some((p, n)) => (Some(*p), Some(n.as_str())),
        None => (None, None),
    };
    conn.execute(
        "INSERT INTO tasks (description, session_pid, session_name, started_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![desc, pid, name, db::now()],
    )?;
    let id = conn.last_insert_rowid();
    println!("Started T{id}: {desc}");
    println!("Pass `-t T{id}` on every subsequent coord command for this task.");
    Ok(())
}

pub fn done(root: &Path, id: i64) -> Result<()> {
    let conn = db::open(root)?;
    let Some(task) = db::get_task(&conn, id)? else {
        bail!("no such task: T{id}");
    };
    if task.finished_at.is_some() {
        bail!("T{id} is already done");
    }
    let now = db::now();
    let released: Vec<String> = conn
        .prepare(
            "SELECT path FROM claims WHERE task_id = ?1 AND released_at IS NULL ORDER BY path",
        )?
        .query_map([id], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    conn.execute(
        "UPDATE claims SET released_at = ?1 WHERE task_id = ?2 AND released_at IS NULL",
        rusqlite::params![now, id],
    )?;
    conn.execute(
        "UPDATE tasks SET finished_at = ?1 WHERE id = ?2",
        rusqlite::params![now, id],
    )?;
    println!(
        "T{id} \"{}\" done after {}.",
        task.description,
        db::fmt_duration(now - task.started_at)
    );
    if released.is_empty() {
        println!("Released: (no claims)");
    } else {
        println!("Released: {}", released.join(", "));
    }
    Ok(())
}
