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
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn two_sessions_full_protocol() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    common::git_repo(dir);
    common::setup(dir);
    std::fs::write(dir.join("README.md"), "hello\n").unwrap();
    git_stdout(dir, &["add", "-A"]);
    git_stdout(dir, &["commit", "-q", "-m", "initial"]);

    // Session A starts refactoring auth; session B starts docs work.
    let ta = common::start_task(dir, "refactor auth");
    let tb = common::start_task(dir, "improve docs");
    common::coord(dir)
        .args(["claim", "-t", &ta, "src/auth.rs"])
        .assert()
        .success();
    common::coord(dir)
        .args(["claim", "-t", &tb, "docs"])
        .assert()
        .success();

    // B wants auth too — refused with intel; B reorders to docs-only work.
    common::coord(dir)
        .args(["claim", "-t", &tb, "src/auth.rs"])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("refactor auth"));

    // Both write their own files.
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join("src/auth.rs"), "// new auth\n").unwrap();
    std::fs::write(dir.join("docs/guide.md"), "# guide\n").unwrap();

    // Both commit; neither sweeps the other's files.
    common::coord(dir)
        .args(["commit", "-t", &ta, "-m", "refactor: auth"])
        .assert()
        .success();
    common::coord(dir)
        .args(["commit", "-t", &tb, "-m", "docs: guide"])
        .assert()
        .success();
    let a_files = git_stdout(dir, &["show", "--name-only", "--format=", "HEAD~1"]);
    let b_files = git_stdout(dir, &["show", "--name-only", "--format=", "HEAD"]);
    assert!(a_files.contains("src/auth.rs") && !a_files.contains("docs/guide.md"));
    assert!(b_files.contains("docs/guide.md") && !b_files.contains("src/auth.rs"));

    // A finishes; B can now claim auth.
    common::coord(dir)
        .args(["task", "done", &ta])
        .assert()
        .success();
    common::coord(dir)
        .args(["claim", "-t", &tb, "src/auth.rs"])
        .assert()
        .success();
    common::coord(dir)
        .args(["task", "done", &tb])
        .assert()
        .success();
    common::coord(dir)
        .arg("status")
        .assert()
        .stdout(predicate::str::contains("No active tasks."));
}
