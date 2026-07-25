mod common;
use predicates::prelude::*;

#[test]
fn start_prints_id_and_reminder() {
    let tmp = tempfile::tempdir().unwrap();
    common::setup(tmp.path());
    common::coord(tmp.path())
        .args(["task", "start", "refactor login flow"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Started T1: refactor login flow")
                .and(predicate::str::contains("-t T1")),
        );
    // Ids increment.
    assert_eq!(common::start_task(tmp.path(), "second task"), "T2");
}

#[test]
fn start_rejects_empty_description() {
    let tmp = tempfile::tempdir().unwrap();
    common::setup(tmp.path());
    common::coord(tmp.path())
        .args(["task", "start", "   "])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("description"));
}

#[test]
fn done_releases_and_reports() {
    let tmp = tempfile::tempdir().unwrap();
    common::setup(tmp.path());
    let t1 = common::start_task(tmp.path(), "some work");
    common::coord(tmp.path())
        .args(["task", "done", &t1])
        .assert()
        .success()
        .stdout(predicate::str::contains("T1").and(predicate::str::contains("done")));
    // Finishing twice is an error.
    common::coord(tmp.path())
        .args(["task", "done", &t1])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("already done"));
    // Unknown id is an error.
    common::coord(tmp.path())
        .args(["task", "done", "T99"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("no such task"));
}
