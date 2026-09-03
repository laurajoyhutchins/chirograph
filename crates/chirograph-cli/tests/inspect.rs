use std::path::PathBuf;
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn inspect_reports_a_consistent_contract_clause() {
    let output = Command::new(env!("CARGO_BIN_EXE_chirograph"))
        .arg("inspect")
        .arg(fixture("consistent.json"))
        .output()
        .expect("chirograph should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    for expected in [
        "example.contract",
        "semantic",
        "example.requirement",
        "CONSISTENT",
        "example.source",
    ] {
        assert!(
            stdout.contains(expected),
            "expected {expected:?} in output:\n{stdout}"
        );
    }
}
