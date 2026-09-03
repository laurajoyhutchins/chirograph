use std::path::PathBuf;
use std::process::Command;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
        .join("drift")
        .join("retry-safety.json")
}

#[test]
fn inspect_exposes_cross_representation_retry_safety_drift() {
    let output = Command::new(env!("CARGO_BIN_EXE_chirograph"))
        .arg("inspect")
        .arg(fixture())
        .output()
        .expect("chirograph should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    for expected in [
        "mutation.retry-safety",
        "recovery",
        "retry-after-indeterminate-does-not-duplicate",
        "CONTESTED",
        "retry.docs",
        "retry.test",
        "retry.runtime",
    ] {
        assert!(
            stdout.contains(expected),
            "expected {expected:?} in output:\n{stdout}"
        );
    }
}
