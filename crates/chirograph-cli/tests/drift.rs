use std::path::PathBuf;
use std::process::Command;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
        .join("drift")
        .join("retry-safety.json")
}

fn kafka_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
        .join("kafka")
        .join("producer-idempotence.evidence.json")
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

#[test]
fn inspect_preserves_kafka_idempotence_failure_policy_drift() {
    let output = Command::new(env!("CARGO_BIN_EXE_chirograph"))
        .arg("inspect")
        .arg(kafka_fixture())
        .output()
        .expect("chirograph should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    for expected in [
        "kafka.producer.idempotence",
        "clause.max-in-flight-limit [requirement] CONSISTENT",
        "clause.explicit-max-in-flight-failure [guarantee] CONSISTENT",
        "clause.implicit-max-in-flight-fallback [guarantee] CONTESTED",
        "supports: kafka.producer.idempotence.doc",
        "contradicts: kafka.producer.idempotence.upgrade-note, kafka.producer.idempotence.validator",
        "authority failure: kafka.producer.idempotence.validator (mechanical_enforcement)",
    ] {
        assert!(
            stdout.contains(expected),
            "expected {expected:?} in output:\n{stdout}"
        );
    }
}
