use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

fn fixture_tree(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "chirograph-cli-analysis-{}-{label}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("temporary source tree should be created");
    fs::write(
        root.join("manifest.rs"),
        r#"
#[serde(rename_all = "kebab-case")]
struct Manifest {
    profile: Profile,
}

#[serde(rename_all = "kebab-case")]
struct Profile {
    debug_info: DebugInfo,
}

#[serde(rename_all = "kebab-case")]
enum DebugInfo {
    None,
    Full,
}
"#,
    )
    .expect("temporary Rust source should be written");
    fs::write(
        root.join("manifest.schema.json"),
        r#"{"properties":{"profile":{"properties":{"debug-info":{"enum":["None","Full"]}}}}}"#,
    )
    .expect("temporary schema should be written");
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
    let root = fixture_tree("missing-provenance");
    let output = Command::new(env!("CARGO_BIN_EXE_chirograph"))
        .arg("analyze")
        .arg(&root)
        .arg("--format")
        .arg("graph-json")
        .output()
        .expect("chirograph should execute");
    fs::remove_dir_all(&root).unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("source-repository"),
        "expected source provenance diagnostic, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn analyze_with_explicit_context_emits_canonical_contract_graph() {
    let root = fixture_tree("explicit-context");
    let output = analyze(&root, "acme/fixture-project", REVISION);
    fs::remove_dir_all(&root).unwrap();

    assert!(
        output.status.success(),
        "analyze failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("{\"schema\":\"chirograph-graph-v1\""));
    assert!(stdout.contains("fixture-project.profile.debug-info"));
}

#[test]
fn analyze_rejects_malformed_repository_and_revision() {
    let root = fixture_tree("bad-provenance");
    let bad_repository = analyze(&root, "not-a-repository", REVISION);
    let bad_revision = analyze(&root, "acme/fixture-project", "not-a-revision");
    fs::remove_dir_all(&root).unwrap();

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
fn analyze_output_does_not_depend_on_fixture_directory_name() {
    let first_root = fixture_tree("path-a");
    let second_root = fixture_tree("completely-unrelated-path-b");

    let first = analyze(&first_root, "acme/fixture-project", REVISION);
    let second = analyze(&second_root, "acme/fixture-project", REVISION);

    fs::remove_dir_all(&first_root).unwrap();
    fs::remove_dir_all(&second_root).unwrap();

    assert!(first.status.success());
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);
    let stdout = String::from_utf8(first.stdout).unwrap();
    assert!(!stdout.contains("path-a"));
    assert!(!stdout.contains("completely-unrelated-path-b"));
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
