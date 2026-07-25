use crate::{db, session};
use anyhow::Result;
use std::path::Path;

pub fn run(root: &Path) -> Result<()> {
    let conn = db::open(root)?;
    let now = db::now();
    let tasks = db::active_tasks(&conn)?;
    if tasks.is_empty() {
        println!("No active tasks.");
        return Ok(());
    }
    let claims = db::active_claims(&conn)?;
    for task in &tasks {
        let stale = if session::is_stale(task, now) {
            "  [STALE — safe to take over]"
        } else {
            ""
        };
        println!(
            "T{}  \"{}\"  started {}{stale}",
            task.id,
            task.description,
            db::fmt_age(now - task.started_at)
        );
        for c in claims.iter().filter(|c| c.task.id == task.id) {
            let forced = if c.forced { "  [forced co-edit]" } else { "" };
            println!(
                "  {}  claimed {}{forced}",
                c.path,
                db::fmt_age(now - c.claimed_at)
            );
        }
    }
    Ok(())
}
