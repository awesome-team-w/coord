use crate::{db, paths, session};
use anyhow::{bail, Result};
use rusqlite::TransactionBehavior;
use std::path::Path;

pub fn run(root: &Path, cwd: &Path, task_id: i64, inputs: &[String], force: bool) -> Result<i32> {
    let mut conn = db::open(root)?;
    let now = db::now();

    let mut normalized = Vec::new();
    for input in inputs {
        normalized.push(paths::normalize(root, cwd, input)?);
    }
    normalized.sort();
    normalized.dedup();

    // IMMEDIATE takes the write lock before we read, so check-then-insert
    // is atomic across concurrent coord processes.
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let Some(task) = db::get_task(&tx, task_id)? else {
        bail!("no such task: T{task_id}");
    };
    if task.finished_at.is_some() {
        bail!("T{task_id} is already done; start a new task");
    }

    let active = db::active_claims(&tx)?;
    let mut conflicts = 0usize;
    for path in &normalized {
        if active
            .iter()
            .any(|c| c.task.id == task_id && c.path == *path)
        {
            println!("already claimed: {path}");
            continue;
        }
        let holders: Vec<_> = active
            .iter()
            .filter(|c| c.task.id != task_id && paths::overlaps(&c.path, path))
            .collect();
        let live: Vec<_> = holders
            .iter()
            .filter(|c| !session::is_stale(&c.task, now))
            .collect();
        if !live.is_empty() && !force {
            conflicts += 1;
            println!("CLAIMED {path}");
            for c in &live {
                let session = c
                    .task
                    .session_pid
                    .map(|p| format!("session {p}, "))
                    .unwrap_or_default();
                println!(
                    "  by T{} \"{}\" ({}claimed {})",
                    c.task.id,
                    c.task.description,
                    session,
                    db::fmt_age(now - c.claimed_at)
                );
            }
            continue;
        }
        let forced = force && !live.is_empty();
        tx.execute(
            "INSERT INTO claims (task_id, path, claimed_at, forced) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![task_id, path, now, forced as i64],
        )?;
        if forced {
            let others: Vec<String> = live.iter().map(|c| format!("T{}", c.task.id)).collect();
            println!(
                "registered {path} (FORCED co-edit with {})",
                others.join(", ")
            );
        } else if !holders.is_empty() {
            let stale: Vec<String> = holders.iter().map(|c| format!("T{}", c.task.id)).collect();
            println!(
                "registered {path} (taken over from stale {})",
                stale.join(", ")
            );
        } else {
            println!("registered {path}");
        }
    }
    tx.commit()?;

    if conflicts > 0 {
        println!();
        println!(
            "{conflicts} path(s) occupied. Work on other files first, or check `coord status`;"
        );
        println!(
            "if the edits are truly parallel-safe, re-run with --force to register co-editing."
        );
        return Ok(2);
    }
    Ok(0)
}
