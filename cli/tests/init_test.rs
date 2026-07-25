mod common;
use predicates::prelude::*;

#[test]
fn help_lists_all_subcommands() {
    let tmp = tempfile::tempdir().unwrap();
    common::coord(tmp.path())
        .arg("--help")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("init")
                .and(predicate::str::contains("task"))
                .and(predicate::str::contains("claim"))
                .and(predicate::str::contains("status"))
                .and(predicate::str::contains("commit")),
        );
}

#[test]
fn commands_require_init() {
    let tmp = tempfile::tempdir().unwrap();
    common::coord(tmp.path())
        .arg("status")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("coord init"));
}
