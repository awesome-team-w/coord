mod common;
use predicates::prelude::*;

#[test]
fn status_empty() {
    let tmp = tempfile::tempdir().unwrap();
    common::setup(tmp.path());
    common::coord(tmp.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("No active tasks."));
}

#[test]
fn status_shows_tasks_claims_and_staleness() {
    let tmp = tempfile::tempdir().unwrap();
    common::setup(tmp.path());
    let t1 = common::start_task(tmp.path(), "refactor login flow");
    common::coord(tmp.path())
        .args(["claim", "-t", &t1, "src/auth.ts"])
        .assert()
        .success();
    let t2 = common::start_task(tmp.path(), "hotfix");
    common::coord(tmp.path())
        .args(["claim", "-t", &t2, "src/auth.ts", "--force"])
        .assert()
        .success();

    common::coord(tmp.path())
        .arg("status")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("T1")
                .and(predicate::str::contains("refactor login flow"))
                .and(predicate::str::contains("src/auth.ts"))
                .and(predicate::str::contains("[forced co-edit]")),
        );

    // Finished tasks disappear.
    common::coord(tmp.path())
        .args(["task", "done", &t2])
        .assert()
        .success();
    common::coord(tmp.path())
        .arg("status")
        .assert()
        .stdout(predicate::str::contains("hotfix").not());

    // Staleness is flagged.
    std::thread::sleep(std::time::Duration::from_secs(2));
    common::coord(tmp.path())
        .env("COORD_STALE_SECS", "1")
        .arg("status")
        .assert()
        .stdout(predicate::str::contains("STALE"));
}
