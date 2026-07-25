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

#[test]
fn init_creates_state_gitignore_and_agents_block() {
    let tmp = tempfile::tempdir().unwrap();
    common::coord(tmp.path()).arg("init").assert().success();
    assert!(tmp.path().join(".agentcoord/state.db").exists());
    let gitignore = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
    assert!(gitignore.lines().any(|l| l == ".agentcoord/"));
    let agents = std::fs::read_to_string(tmp.path().join("AGENTS.md")).unwrap();
    assert!(agents.contains("<!-- coord:begin -->"));
    assert!(agents.contains("<!-- coord:end -->"));
    assert!(agents.contains("coord task start"));
}

#[test]
fn init_is_idempotent_and_preserves_user_content() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("AGENTS.md"), "# My rules\n\nBe nice.\n").unwrap();
    common::coord(tmp.path()).arg("init").assert().success();
    common::coord(tmp.path()).arg("init").assert().success();
    let agents = std::fs::read_to_string(tmp.path().join("AGENTS.md")).unwrap();
    assert!(agents.contains("# My rules"));
    assert!(agents.contains("Be nice."));
    assert_eq!(agents.matches("<!-- coord:begin -->").count(), 1);
    let gitignore = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
    assert_eq!(gitignore.matches(".agentcoord/").count(), 1);
}

#[test]
fn init_updates_existing_block_in_place() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("AGENTS.md"),
        "before\n\n<!-- coord:begin -->\nOLD CONTENT\n<!-- coord:end -->\n\nafter\n",
    )
    .unwrap();
    common::coord(tmp.path()).arg("init").assert().success();
    let agents = std::fs::read_to_string(tmp.path().join("AGENTS.md")).unwrap();
    assert!(agents.contains("before"));
    assert!(agents.contains("after"));
    assert!(!agents.contains("OLD CONTENT"));
    assert!(agents.contains("coord task start"));
}
