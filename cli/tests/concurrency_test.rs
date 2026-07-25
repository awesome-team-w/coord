mod common;

#[test]
fn exactly_one_winner_under_contention() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    common::setup(dir);
    let n = 8;
    for i in 0..n {
        common::start_task(dir, &format!("contender {i}"));
    }
    let bin = assert_cmd::cargo::cargo_bin("coord");
    let children: Vec<_> = (1..=n)
        .map(|i| {
            std::process::Command::new(&bin)
                .current_dir(dir)
                .args(["claim", "-t", &format!("T{i}"), "hot.rs"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap()
        })
        .collect();
    let codes: Vec<i32> = children
        .into_iter()
        .map(|mut c| c.wait().unwrap().code().unwrap())
        .collect();
    assert_eq!(
        codes.iter().filter(|&&c| c == 0).count(),
        1,
        "codes: {codes:?}"
    );
    assert_eq!(
        codes.iter().filter(|&&c| c == 2).count(),
        n - 1,
        "codes: {codes:?}"
    );
}
