mod common;
use predicates::prelude::*;
use std::path::Path;

fn git_stdout(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?} failed: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn commit_stages_only_claimed_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    common::git_repo(dir);
    common::setup(dir);
    std::fs::write(dir.join("base.txt"), "base\n").unwrap();
    git_stdout(dir, &["add", "-A"]);
    git_stdout(dir, &["commit", "-q", "-m", "initial"]);

    let t1 = common::start_task(dir, "write a");
    common::coord(dir).args(["claim", "-t", &t1, "a.txt"]).assert().success();
    std::fs::write(dir.join("a.txt"), "A\n").unwrap();

    // Another session's mess: an untracked file and a staged file.
    std::fs::write(dir.join("other.txt"), "other\n").unwrap();
    std::fs::write(dir.join("staged.txt"), "staged\n").unwrap();
    git_stdout(dir, &["add", "staged.txt"]);

    common::coord(dir)
        .args(["commit", "-t", &t1, "-m", "feat: add a"])
        .assert()
        .success();

    let files = git_stdout(dir, &["show", "--name-only", "--format=", "HEAD"]);
    assert!(files.contains("a.txt"));
    assert!(!files.contains("other.txt"));
    assert!(!files.contains("staged.txt"));
    let msg = git_stdout(dir, &["log", "-1", "--format=%B"]);
    assert!(msg.contains("feat: add a"));
    assert!(msg.contains("Coord-Task: T1 write a"));
    // The other session's staged file is still staged, untouched.
    let status = git_stdout(dir, &["status", "--porcelain"]);
    assert!(status.contains("staged.txt"));
    assert!(status.contains("other.txt"));
}

#[test]
fn commit_handles_directory_claims_and_deletions() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    common::git_repo(dir);
    common::setup(dir);
    std::fs::create_dir_all(dir.join("mod")).unwrap();
    std::fs::write(dir.join("mod/old.rs"), "old\n").unwrap();
    git_stdout(dir, &["add", "-A"]);
    git_stdout(dir, &["commit", "-q", "-m", "initial"]);

    let t1 = common::start_task(dir, "rework mod");
    common::coord(dir).args(["claim", "-t", &t1, "mod"]).assert().success();
    std::fs::remove_file(dir.join("mod/old.rs")).unwrap();
    std::fs::write(dir.join("mod/new.rs"), "new\n").unwrap();

    common::coord(dir).args(["commit", "-t", &t1, "-m", "refactor: rework mod"]).assert().success();
    let files = git_stdout(dir, &["show", "--name-only", "--format=", "HEAD"]);
    assert!(files.contains("mod/new.rs"));
    assert!(files.contains("mod/old.rs")); // shown as deleted
}

#[test]
fn commit_errors_without_git_claims_or_changes() {
    // No git repo.
    let tmp = tempfile::tempdir().unwrap();
    common::setup(tmp.path());
    let t1 = common::start_task(tmp.path(), "x");
    common::coord(tmp.path())
        .args(["commit", "-t", &t1, "-m", "m"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not a git repository"));

    // Git repo, but no claims.
    let tmp = tempfile::tempdir().unwrap();
    common::git_repo(tmp.path());
    common::setup(tmp.path());
    let t1 = common::start_task(tmp.path(), "x");
    common::coord(tmp.path())
        .args(["commit", "-t", &t1, "-m", "m"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("no claimed files"));

    // Claims exist but nothing on disk / tracked.
    common::coord(tmp.path()).args(["claim", "-t", &t1, "ghost.rs"]).assert().success();
    common::coord(tmp.path())
        .args(["commit", "-t", &t1, "-m", "m"])
        .assert()
        .code(1);
}
