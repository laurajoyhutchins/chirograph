use std::path::PathBuf;
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn contestations_command_preserves_both_sides_of_a_disagreement() {
    let output = Command::new(env!("CARGO_BIN_EXE_chirograph"))
        .arg("contestations")
        .arg(fixture("semantic-query.json"))
        .output()
        .expect("chirograph should execute");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    for expected in [
        "review-wire-value",
        "CONTESTED",
        "runtime-status",
        "schema-status",
        "docs-status",
    ] {
        assert!(
            stdout.contains(expected),
            "expected {expected:?} in output:\n{stdout}"
        );
    }
}
