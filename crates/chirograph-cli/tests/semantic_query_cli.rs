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

#[test]
fn evidence_command_returns_only_explicit_provenance_closure() {
    let output = Command::new(env!("CARGO_BIN_EXE_chirograph"))
        .arg("evidence")
        .arg(fixture("semantic-query.json"))
        .arg("review-status")
        .output()
        .expect("chirograph should execute");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    for expected in ["obs-docs", "obs-runtime", "obs-schema"] {
        assert!(
            stdout.contains(expected),
            "expected {expected:?} in output:\n{stdout}"
        );
    }
    assert!(
        !stdout.contains("obs-unlinked"),
        "unlinked source observation leaked into evidence closure:\n{stdout}"
    );
}

#[test]
fn authority_command_preserves_multiple_claims_without_selecting_a_winner() {
    let output = Command::new(env!("CARGO_BIN_EXE_chirograph"))
        .arg("authority")
        .arg(fixture("semantic-query.json"))
        .arg("review-status")
        .arg("semantic")
        .output()
        .expect("chirograph should execute");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    for expected in [
        "review-status",
        "semantic",
        "runtime-status",
        "observed_behavior",
        "schema-status",
        "explicit_declaration",
    ] {
        assert!(
            stdout.contains(expected),
            "expected {expected:?} in output:\n{stdout}"
        );
    }
}

#[test]
fn alignment_command_reports_recorded_state_without_resolving_it() {
    let output = Command::new(env!("CARGO_BIN_EXE_chirograph"))
        .arg("alignment")
        .arg(fixture("semantic-query.json"))
        .arg(fixture("alignments.json"))
        .arg("candidate-example")
        .output()
        .expect("chirograph should execute");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    for expected in [
        "candidate-example",
        "review-status",
        "structural",
        "REJECTED",
        "semantic",
        "UNRESOLVED",
        "obs-docs",
    ] {
        assert!(
            stdout.contains(expected),
            "expected {expected:?} in output:\n{stdout}"
        );
    }
}
