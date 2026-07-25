use crate::db;
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

pub fn run(root: &Path, task_id: i64, message: &str) -> Result<()> {
    if !root.join(".git").exists() {
        bail!(
            "not a git repository: `coord commit` needs git (the ledger itself works without it)"
        );
    }
    let conn = db::open(root)?;
    let Some(task) = db::get_task(&conn, task_id)? else {
        bail!("no such task: T{task_id}");
    };
    if task.finished_at.is_some() {
        bail!("T{task_id} is already done");
    }
    let paths: Vec<String> = conn
        .prepare(
            "SELECT path FROM claims WHERE task_id = ?1 AND released_at IS NULL ORDER BY path",
        )?
        .query_map([task_id], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    if paths.is_empty() {
        bail!("T{task_id} has no claimed files; claim before committing");
    }

    // Git rejects pathspecs matching nothing; keep paths that exist or are tracked.
    let mut specs = Vec::new();
    for p in &paths {
        if root.join(p).exists() || is_tracked(root, p)? {
            specs.push(p.clone());
        }
    }
    if specs.is_empty() {
        bail!("none of T{task_id}'s claimed paths have content to commit");
    }

    git(root, |c| {
        c.args(["add", "-A", "--"]).args(&specs);
    })?;
    let full_message = format!("{message}\n\nCoord-Task: T{task_id} {}", task.description);
    // Pathspec commit: only these paths enter the commit, regardless of
    // whatever other sessions have staged in the shared index.
    git(root, |c| {
        c.args(["commit", "-m", &full_message, "--"]).args(&specs);
    })?;
    println!("Committed T{task_id} scope: {}", specs.join(", "));
    Ok(())
}

fn is_tracked(root: &Path, path: &str) -> Result<bool> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", path])
        .output()
        .context("failed to run git")?;
    Ok(!out.stdout.is_empty())
}

fn git(root: &Path, f: impl FnOnce(&mut Command)) -> Result<()> {
    let mut c = Command::new("git");
    c.arg("-C").arg(root);
    f(&mut c);
    let status = c.status().context("failed to run git")?;
    if !status.success() {
        bail!("git exited with {status}");
    }
    Ok(())
}
