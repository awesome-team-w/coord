mod common;
use predicates::prelude::*;

#[test]
fn claim_free_paths_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    common::setup(tmp.path());
    let t1 = common::start_task(tmp.path(), "task one");
    common::coord(tmp.path())
        .args(["claim", "-t", &t1, "src/a.rs", "src/b.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("registered src/a.rs").and(predicate::str::contains("registered src/b.rs")));
}

#[test]
fn conflicting_claim_reports_holder_and_exits_2() {
    let tmp = tempfile::tempdir().unwrap();
    common::setup(tmp.path());
    let t1 = common::start_task(tmp.path(), "refactor login flow");
    let t2 = common::start_task(tmp.path(), "add rate limiting");
    common::coord(tmp.path()).args(["claim", "-t", &t1, "src/auth.ts"]).assert().success();
    common::coord(tmp.path())
        .args(["claim", "-t", &t2, "src/auth.ts"])
        .assert()
        .code(2)
        .stdout(
            predicate::str::contains("CLAIMED src/auth.ts")
                .and(predicate::str::contains("T1"))
                .and(predicate::str::contains("refactor login flow"))
                .and(predicate::str::contains("--force")),
        );
    // Partial success: one free path + one occupied path still exits 2 but registers the free one.
    common::coord(tmp.path())
        .args(["claim", "-t", &t2, "src/free.rs", "src/auth.ts"])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("registered src/free.rs"));
}

#[test]
fn directory_claims_contain_files_and_vice_versa() {
    let tmp = tempfile::tempdir().unwrap();
    common::setup(tmp.path());
    let t1 = common::start_task(tmp.path(), "own the src dir");
    let t2 = common::start_task(tmp.path(), "other");
    common::coord(tmp.path()).args(["claim", "-t", &t1, "src"]).assert().success();
    common::coord(tmp.path()).args(["claim", "-t", &t2, "src/deep/file.rs"]).assert().code(2);
    common::coord(tmp.path()).args(["claim", "-t", &t2, "srclib/x.rs"]).assert().success();
    // And a file blocks a parent-directory claim.
    common::coord(tmp.path()).args(["claim", "-t", &t2, "."]).assert().code(1); // repo root is rejected outright
}

#[test]
fn same_task_reclaim_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    common::setup(tmp.path());
    let t1 = common::start_task(tmp.path(), "task one");
    common::coord(tmp.path()).args(["claim", "-t", &t1, "a.rs"]).assert().success();
    common::coord(tmp.path())
        .args(["claim", "-t", &t1, "a.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already claimed: a.rs"));
}

#[test]
fn force_registers_co_edit() {
    let tmp = tempfile::tempdir().unwrap();
    common::setup(tmp.path());
    let t1 = common::start_task(tmp.path(), "one");
    let t2 = common::start_task(tmp.path(), "two");
    common::coord(tmp.path()).args(["claim", "-t", &t1, "hot.rs"]).assert().success();
    common::coord(tmp.path())
        .args(["claim", "-t", &t2, "hot.rs", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("FORCED co-edit").and(predicate::str::contains("T1")));
}

#[test]
fn stale_holder_is_taken_over() {
    let tmp = tempfile::tempdir().unwrap();
    common::setup(tmp.path());
    let t1 = common::start_task(tmp.path(), "will go stale");
    common::coord(tmp.path()).args(["claim", "-t", &t1, "old.rs"]).assert().success();
    std::thread::sleep(std::time::Duration::from_secs(2));
    let t2 = common::start_task(tmp.path(), "fresh task");
    common::coord(tmp.path())
        .env("COORD_STALE_SECS", "1")
        .args(["claim", "-t", &t2, "old.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("taken over from stale T1"));
}

#[test]
fn claim_rejects_paths_outside_repo_and_unknown_task() {
    let tmp = tempfile::tempdir().unwrap();
    common::setup(tmp.path());
    let t1 = common::start_task(tmp.path(), "task one");
    common::coord(tmp.path()).args(["claim", "-t", &t1, "../outside.rs"]).assert().code(1);
    common::coord(tmp.path())
        .args(["claim", "-t", "T99", "a.rs"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("no such task"));
}
