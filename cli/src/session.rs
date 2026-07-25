use crate::db::TaskRow;
use sysinfo::{Pid, System};

const SHELLS: &[&str] = &["sh", "bash", "zsh", "fish", "dash", "ksh", "coord"];

/// Walk up the ancestor chain from this process and return the first
/// non-shell ancestor (normally the coding agent). Best effort: None in
/// odd process trees; callers degrade to time-based staleness.
pub fn detect_agent_process() -> Option<(i64, String)> {
    let sys = System::new_all();
    let mut pid = Pid::from_u32(std::process::id());
    for _ in 0..15 {
        let parent = sys.process(pid)?.parent()?;
        if parent.as_u32() <= 1 {
            return None;
        }
        let name = sys.process(parent)?.name().to_string();
        if !SHELLS.contains(&name.as_str()) {
            return Some((parent.as_u32() as i64, name));
        }
        pid = parent;
    }
    None
}

/// Pid exists and its process name matches (guards against pid reuse).
pub fn is_alive(pid: i64, expected_name: &str) -> bool {
    if pid <= 0 {
        return false;
    }
    let sys = System::new_all();
    match sys.process(Pid::from_u32(pid as u32)) {
        Some(p) => p.name() == expected_name,
        None => false,
    }
}

pub fn stale_limit_secs() -> i64 {
    std::env::var("COORD_STALE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7200)
}

/// A task is stale when its registration outlived the limit, or its
/// recorded session process is gone.
pub fn is_stale(task: &TaskRow, now: i64) -> bool {
    if now - task.started_at > stale_limit_secs() {
        return true;
    }
    match (task.session_pid, task.session_name.as_deref()) {
        (Some(pid), Some(name)) => !is_alive(pid, name),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::TaskRow;

    fn task(pid: Option<i64>, name: Option<&str>, started_at: i64) -> TaskRow {
        TaskRow {
            id: 1,
            description: "x".into(),
            session_pid: pid,
            session_name: name.map(String::from),
            started_at,
            finished_at: None,
        }
    }

    #[test]
    fn own_process_is_alive_under_its_real_name() {
        let sys = sysinfo::System::new_all();
        let me = sysinfo::Pid::from_u32(std::process::id());
        let my_name = sys.process(me).unwrap().name().to_string();
        assert!(is_alive(std::process::id() as i64, &my_name));
        assert!(!is_alive(std::process::id() as i64, "definitely-not-this-name"));
    }

    #[test]
    fn dead_process_is_not_alive() {
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let pid = child.id() as i64;
        child.wait().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(!is_alive(pid, "true"));
    }

    #[test]
    fn staleness_by_age_and_by_dead_pid() {
        let now = 10_000;
        // Live pid (ours), fresh: not stale.
        let sys = sysinfo::System::new_all();
        let me = sysinfo::Pid::from_u32(std::process::id());
        let my_name = sys.process(me).unwrap().name().to_string();
        assert!(!is_stale(&task(Some(std::process::id() as i64), Some(&my_name), now - 10), now));
        // Fresh but pid dead: stale.
        assert!(is_stale(&task(Some(99_999_999), Some("ghost"), now - 10), now));
        // No pid recorded, young: not stale; over the limit: stale.
        assert!(!is_stale(&task(None, None, now - 10), now));
        assert!(is_stale(&task(None, None, now - stale_limit_secs() - 1), now));
    }

    #[test]
    fn detect_does_not_panic() {
        let _ = detect_agent_process(); // value depends on environment; just exercise it
    }
}
