use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// Nearest ancestor containing `.agentcoord/`, else nearest containing `.git`, else None.
pub fn find_root(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        if d.join(".agentcoord").is_dir() {
            return Some(d.to_path_buf());
        }
        dir = d.parent();
    }
    let mut dir = Some(start);
    while let Some(d) = dir {
        if d.join(".git").exists() {
            return Some(d.to_path_buf());
        }
        dir = d.parent();
    }
    None
}

/// Accepts "12", "T12", "t12".
pub fn parse_task_id(s: &str) -> Result<i64> {
    let t = s.trim();
    let t = t.strip_prefix('T').or_else(|| t.strip_prefix('t')).unwrap_or(t);
    t.parse::<i64>()
        .map_err(|_| anyhow!("invalid task id: {s} (expected e.g. T12)"))
}
