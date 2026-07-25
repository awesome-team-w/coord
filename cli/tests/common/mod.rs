#![allow(dead_code)]
use assert_cmd::Command;
use std::path::Path;

pub fn coord(dir: &Path) -> Command {
    let mut c = Command::cargo_bin("coord").unwrap();
    c.current_dir(dir);
    c.env_remove("COORD_STALE_SECS");
    c
}

pub fn setup(dir: &Path) {
    coord(dir).arg("init").assert().success();
}

/// Starts a task, returns its id string, e.g. "T1".
pub fn start_task(dir: &Path, desc: &str) -> String {
    let out = coord(dir).args(["task", "start", desc]).output().unwrap();
    assert!(out.status.success(), "task start failed: {out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    // First line: "Started T3: <desc>"
    stdout
        .split_whitespace()
        .nth(1)
        .unwrap()
        .trim_end_matches(':')
        .to_string()
}

pub fn git_repo(dir: &Path) {
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test"],
    ] {
        let st = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(&args)
            .status()
            .unwrap();
        assert!(st.success(), "git {args:?} failed");
    }
}
