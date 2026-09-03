#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chirograph_benchmark::model::{
    GOLDEN_SCHEMA, SPECIMEN_SCHEMA, BenchmarkCase, GoldenV1, SpecimenV1, UpstreamV1,
};
use chirograph_benchmark::runner::run_case;
use chirograph_benchmark::score::CaseStatus;

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn temp_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "chirograph-benchmark-runner-{}-{}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).expect("create temp root");
    root
}

fn case(root: &Path) -> BenchmarkCase {
    let fixture_dir = root.join("fixture");
    fs::create_dir_all(&fixture_dir).expect("create fixture dir");
    BenchmarkCase {
        id: "repo/scenario/case".to_owned(),
        repository: "repo".to_owned(),
        scenario: "scenario".to_owned(),
        root: root.to_path_buf(),
        fixture_dir,
        specimen_path: root.join("specimen.yaml"),
        golden_path: root.join("golden.yaml"),
        specimen: SpecimenV1 {
            schema: SPECIMEN_SCHEMA.to_owned(),
            id: "repo/scenario/case".to_owned(),
            repository: "repo".to_owned(),
            scenario: "scenario".to_owned(),
            upstream: UpstreamV1 {
                repository: "owner/repo".to_owned(),
                revision: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            },
            files: Vec::new(),
        },
        golden: GoldenV1 {
            schema: GOLDEN_SCHEMA.to_owned(),
            contracts: Vec::new(),
            representations: Vec::new(),
            authority_claims: Vec::new(),
            relationships: Vec::new(),
            clauses: Vec::new(),
            lifecycle: Vec::new(),
            expected_findings: Vec::new(),
            non_contracts: Vec::new(),
        },
    }
}

fn executable(root: &Path, body: &str) -> PathBuf {
    let path = root.join("fake-chirograph");
    fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}\n")).expect("write fake binary");
    let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake executable");
    path
}

fn empty_graph(schema: &str) -> String {
    serde_json::json!({
        "schema": schema,
        "contracts": [],
        "representations": [],
        "relations": [],
        "authority_claims": [],
        "clauses": [],
        "clause_assessments": [],
        "lifecycle": []
    })
    .to_string()
}

#[test]
fn classifies_public_process_boundary_fail_closed() {
    let root = temp_root();
    let benchmark_case = case(&root);

    let nonzero = executable(&root, "printf 'provider exploded\\n' >&2\nexit 7");
    let result = run_case(&benchmark_case, &nonzero);
    assert_eq!(result.status, CaseStatus::ExecutionFailure);
    assert!(result.score.is_none());
    assert!(result.diagnostics.iter().any(|item| item.contains("process_exit")));

    let malformed = executable(&root, "printf 'not-json\\n'");
    let result = run_case(&benchmark_case, &malformed);
    assert_eq!(result.status, CaseStatus::InvalidOutput);
    assert!(result.score.is_none());

    let wrong_schema = empty_graph("wrong-schema");
    let wrong_schema_bin = executable(&root, &format!("cat <<'JSON'\n{wrong_schema}\nJSON"));
    let result = run_case(&benchmark_case, &wrong_schema_bin);
    assert_eq!(result.status, CaseStatus::InvalidOutput);
    assert!(result.score.is_none());

    let fixture = benchmark_case.fixture_dir.display();
    let valid_graph = empty_graph("chirograph-graph-v1");
    let valid = executable(
        &root,
        &format!(
            "test \"$#\" -eq 4\ntest \"$1\" = analyze\ntest \"$2\" = \"{fixture}\"\ntest \"$3\" = --format\ntest \"$4\" = graph-json\ncat <<'JSON'\n{valid_graph}\nJSON"
        ),
    );
    let result = run_case(&benchmark_case, &valid);
    assert_eq!(result.status, CaseStatus::Scored);
    assert!(result.score.is_some());

    fs::remove_dir_all(root).expect("remove temp root");
}
