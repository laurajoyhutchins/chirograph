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

fn write_current_baseline(analyzer: &Path, baseline: &Path) {
    let output = benchmark_command(analyzer)
        .arg("--write-baseline")
        .arg(baseline)
        .output()
        .expect("benchmark baseline should be written");
    assert!(
        output.status.success(),
        "write-baseline failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn write_baseline_records_case_result_and_corpus_digests() {
    let root = temp_root();
    let baseline = root.join("baseline.json");
    let analyzer = fake_chirograph(&root);

    write_current_baseline(&analyzer, &baseline);

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

    write_current_baseline(&analyzer, &baseline);

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

#[test]
fn execution_failure_to_scored_is_an_improvement() {
    let root = temp_root();
    let baseline = root.join("baseline.json");
    let analyzer = fake_chirograph(&root);

    write_current_baseline(&analyzer, &baseline);
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&baseline).expect("baseline should be readable"))
            .expect("baseline should be JSON");
    value["cases"][0]["result"]["status"] = serde_json::json!("execution-failure");
    value["cases"][0]["result"]["score"] = serde_json::Value::Null;
    value["cases"][0]["result"]["diagnostics"] =
        serde_json::json!(["simulated baseline execution failure"]);
    fs::write(
        &baseline,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&value).expect("encode edited baseline")
        ),
    )
    .expect("write edited baseline");

    let compare = benchmark_command(&analyzer)
        .arg("--baseline")
        .arg(&baseline)
        .output()
        .expect("benchmark baseline should be compared");

    assert!(
        compare.status.success(),
        "execution-failure to scored should improve: {}",
        String::from_utf8_lossy(&compare.stderr)
    );

    fs::remove_dir_all(root).expect("remove temp root");
}
