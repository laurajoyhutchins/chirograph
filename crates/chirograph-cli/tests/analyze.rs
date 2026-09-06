use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

fn fixture_tree() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after the Unix epoch")
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("chirograph-analyze-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&root).expect("temporary source tree should be created");
    fs::write(
        root.join("example.rs"),
        "pub enum State { PendingReview, Complete }\n",
    )
    .expect("temporary source file should be written");
    root
}

fn analyze(root: &Path, repository: &str, revision: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chirograph"))
        .arg("analyze")
        .arg(root)
        .arg("--source-repository")
        .arg(repository)
        .arg("--revision")
        .arg(revision)
        .arg("--format")
        .arg("graph-json")
        .output()
        .expect("chirograph should execute")
}

#[test]
fn analyze_requires_explicit_source_provenance() {
    let root = fixture_tree();
    let output = Command::new(env!("CARGO_BIN_EXE_chirograph"))
        .arg("analyze")
        .arg(&root)
        .arg("--format")
        .arg("graph-json")
        .output()
        .expect("chirograph should execute");
    fs::remove_dir_all(&root).expect("temporary source tree should be removed");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("source-repository"),
        "expected source provenance diagnostic, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn analyze_with_explicit_context_emits_deterministic_canonical_graph_json() {
    let root = fixture_tree();

    let first = analyze(&root, "acme/fixture-project", REVISION);
    let second = analyze(&root, "acme/fixture-project", REVISION);

    fs::remove_dir_all(&root).expect("temporary source tree should be removed");

    assert!(
        first.status.success(),
        "first analyze failed: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        second.status.success(),
        "second analyze failed: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(first.stdout, second.stdout, "analysis must be deterministic");

    let stdout = String::from_utf8(first.stdout).expect("stdout should be UTF-8");
    assert!(
        stdout.starts_with("{\"schema\":\"chirograph-graph-v1\""),
        "expected canonical graph JSON, got: {stdout}"
    );
}

#[test]
fn analyze_rejects_malformed_repository_and_revision() {
    let root = fixture_tree();
    let bad_repository = analyze(&root, "not-a-repository", REVISION);
    let bad_revision = analyze(&root, "acme/fixture-project", "not-a-revision");
    fs::remove_dir_all(&root).expect("temporary source tree should be removed");

    assert!(!bad_repository.status.success());
    assert!(
        String::from_utf8_lossy(&bad_repository.stderr).contains("repository"),
        "expected repository diagnostic, got: {}",
        String::from_utf8_lossy(&bad_repository.stderr)
    );
    assert!(!bad_revision.status.success());
    assert!(
        String::from_utf8_lossy(&bad_revision.stderr).contains("revision"),
        "expected revision diagnostic, got: {}",
        String::from_utf8_lossy(&bad_revision.stderr)
    );
}

#[test]
fn analyze_rejects_missing_source_tree_after_provenance_validation() {
    let missing = std::env::temp_dir().join(format!(
        "chirograph-missing-analysis-tree-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&missing);
    let output = analyze(&missing, "acme/fixture-project", REVISION);

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("source tree"),
        "expected source-tree diagnostic, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn analyze_rejects_malformed_supported_source() {
    let root = fixture_tree();
    fs::write(root.join("broken.json"), "{\"contract\":")
        .expect("malformed JSON fixture should be written");

    let output = analyze(&root, "acme/fixture-project", REVISION);
    fs::remove_dir_all(&root).expect("temporary source tree should be removed");

    assert!(
        !output.status.success(),
        "malformed supported source must fail closed"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("broken.json") && stderr.contains("JSON"),
        "expected explicit malformed-source diagnostic, got: {stderr}"
    );
}

#[test]
fn analyze_rejects_malformed_rust_source() {
    let root = fixture_tree();
    fs::write(root.join("broken.rs"), "pub struct Broken {\n")
        .expect("malformed Rust fixture should be written");

    let output = analyze(&root, "acme/fixture-project", REVISION);
    fs::remove_dir_all(&root).expect("temporary source tree should be removed");

    assert!(
        !output.status.success(),
        "malformed Rust source must fail closed"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("broken.rs") && stderr.contains("Rust"),
        "expected explicit Rust parse diagnostic, got: {stderr}"
    );
}