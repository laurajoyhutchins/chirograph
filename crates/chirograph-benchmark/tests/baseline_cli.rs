#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn temp_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "chirograph-benchmark-baseline-{}-{}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).expect("create temp root");
    root
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
}

fn fake_chirograph(root: &Path) -> PathBuf {
    let path = root.join("fake-chirograph");
    let graph = serde_json::json!({
        "schema": "chirograph-graph-v1",
        "contracts": [],
        "representations": [],
        "relations": [],
        "authority_claims": [],
        "clauses": [],
        "clause_assessments": [],
        "lifecycle": []
    });
    fs::write(
        &path,
        format!(
            "#!/bin/sh\nset -eu\ntest \"$1\" = analyze\ntest \"$3\" = --format\ntest \"$4\" = graph-json\ncat <<'JSON'\n{graph}\nJSON\n"
        ),
    )
    .expect("write fake analyzer");
    let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake analyzer executable");
    path
}

fn benchmark_command(analyzer: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_chirograph-benchmark"));
    command
        .current_dir(workspace_root())
        .arg("kubernetes/go-protobuf-openapi/core-v1-pod")
        .arg("--chirograph-bin")
        .arg(analyzer)
        .arg("--format")
        .arg("json");
    command
}

#[test]
fn write_baseline_records_case_result_and_corpus_digests() {
    let root = temp_root();
    let baseline = root.join("baseline.json");
    let analyzer = fake_chirograph(&root);

    let output = benchmark_command(&analyzer)
        .arg("--write-baseline")
        .arg(&baseline)
        .output()
        .expect("benchmark binary should run");

    assert!(
        output.status.success(),
        "write-baseline failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&baseline).expect("baseline should be written"))
            .expect("baseline should be JSON");
    assert_eq!(value["schema"], "chirograph-benchmark-baseline-v1");
    assert_eq!(value["cases"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        value["cases"][0]["id"],
        "kubernetes/go-protobuf-openapi/core-v1-pod"
    );
    for field in ["specimen_sha256", "golden_sha256"] {
        let digest = value["cases"][0][field]
            .as_str()
            .expect("digest should be a string");
        assert_eq!(digest.len(), 64, "{field} should be SHA-256");
        assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
    assert_eq!(value["cases"][0]["result"]["status"], "scored");

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn identical_result_and_corpus_pass_baseline_comparison() {
    let root = temp_root();
    let baseline = root.join("baseline.json");
    let analyzer = fake_chirograph(&root);

    let write = benchmark_command(&analyzer)
        .arg("--write-baseline")
        .arg(&baseline)
        .output()
        .expect("benchmark baseline should be written");
    assert!(write.status.success());

    let compare = benchmark_command(&analyzer)
        .arg("--baseline")
        .arg(&baseline)
        .output()
        .expect("benchmark baseline should be compared");

    assert!(
        compare.status.success(),
        "identical baseline comparison failed: {}",
        String::from_utf8_lossy(&compare.stderr)
    );

    fs::remove_dir_all(root).expect("remove temp root");
}
