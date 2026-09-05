use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

fn analyze(root: &PathBuf) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_chirograph"))
        .arg("analyze")
        .arg(root)
        .arg("--format")
        .arg("graph-json")
        .output()
        .expect("chirograph should execute")
}

#[test]
fn analyze_source_tree_emits_deterministic_canonical_graph_json() {
    let root = fixture_tree();

    let first = analyze(&root);
    let second = analyze(&root);

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
    assert_eq!(
        first.stdout, second.stdout,
        "analysis must be deterministic"
    );

    let stdout = String::from_utf8(first.stdout).expect("stdout should be UTF-8");
    assert!(
        stdout.starts_with("{\"schema\":\"chirograph-graph-v1\""),
        "expected canonical graph JSON, got: {stdout}"
    );
}

#[test]
fn analyze_rejects_missing_source_tree() {
    let missing = std::env::temp_dir().join(format!(
        "chirograph-missing-analysis-tree-{}",
        std::process::id()
    ));
    let output = analyze(&missing);

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

    let output = analyze(&root);
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

    let output = analyze(&root);
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
