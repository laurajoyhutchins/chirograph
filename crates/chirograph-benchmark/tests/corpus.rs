use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chirograph_benchmark::corpus::discover_corpus;
use chirograph_benchmark::model::{parse_golden_yaml, parse_specimen_yaml};
use sha2::{Digest, Sha256};

fn temp_root(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("chirograph-benchmark-{name}-{nonce}"));
    fs::create_dir_all(&root).expect("create temp root");
    root
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_case(root: &Path, extra_file: bool) {
    let case = root.join("cargo/schema-enum-drift/case-a");
    let fixture = case.join("fixture/src");
    fs::create_dir_all(&fixture).expect("create fixture directory");

    let fixture_bytes = b"pub fn example() {}\n";
    fs::write(fixture.join("lib.rs"), fixture_bytes).expect("write fixture");

    let specimen = format!(
        r#"schema: chirograph-benchmark-specimen-v1
id: cargo/schema-enum-drift/case-a
repository: cargo
scenario: schema-enum-drift
upstream:
  repository: rust-lang/cargo
  revision: 2ceefa0090080354b80cc2f5415039bdb0d2bf0b
files:
  - fixture_path: fixture/src/lib.rs
    upstream_path: src/lib.rs
    sha256: {}
"#,
        sha256(fixture_bytes)
    );
    fs::write(case.join("specimen.yaml"), specimen).expect("write specimen");

    let golden = r#"schema: chirograph-benchmark-golden-v1
contracts:
  - id: example.contract
    facets: [structural]
representations: []
authority_claims: []
relationships: []
clauses: []
lifecycle: []
expected_findings: []
non_contracts: []
"#;
    fs::write(case.join("golden.yaml"), golden).expect("write golden");

    if extra_file {
        fs::write(case.join("observe.py"), "print('not allowed')\n").expect("write glue");
    }
}

#[test]
fn parses_strict_typed_benchmark_files() {
    let specimen = r#"schema: chirograph-benchmark-specimen-v1
id: cargo/schema-enum-drift/case-a
repository: cargo
scenario: schema-enum-drift
upstream:
  repository: rust-lang/cargo
  revision: 2ceefa0090080354b80cc2f5415039bdb0d2bf0b
files:
  - fixture_path: fixture/src/lib.rs
    upstream_path: src/lib.rs
    sha256: fc503d6532663f2b0f3217b53f235b6c24690e9c85116f1364ec134ca78cd92c
"#;
    let parsed = parse_specimen_yaml(specimen).expect("valid specimen");
    assert_eq!(parsed.id, "cargo/schema-enum-drift/case-a");
    assert_eq!(parsed.files.len(), 1);

    let golden = r#"schema: chirograph-benchmark-golden-v1
contracts:
  - id: example.contract
    facets: [structural]
representations: []
authority_claims: []
relationships: []
clauses: []
lifecycle: []
expected_findings: []
non_contracts: []
"#;
    let parsed = parse_golden_yaml(golden).expect("valid golden");
    assert_eq!(parsed.contracts[0].id, "example.contract");
}

#[test]
fn rejects_unknown_fields_and_non_exact_revision() {
    let unknown_field = "schema: chirograph-benchmark-specimen-v1\n\
id: cargo/schema-enum-drift/case-a\n\
repository: cargo\n\
scenario: schema-enum-drift\n\
upstream: {repository: rust-lang/cargo, revision: 2ceefa0090080354b80cc2f5415039bdb0d2bf0b}\n\
files: []\n\
surprise: true\n";
    assert!(parse_specimen_yaml(unknown_field).is_err());

    let branch_revision = unknown_field
        .replace("\nsurprise: true", "")
        .replace("2ceefa0090080354b80cc2f5415039bdb0d2bf0b", "main");
    assert!(parse_specimen_yaml(&branch_revision).is_err());
}

#[test]
fn discovers_only_valid_fixed_depth_data_cases() {
    let root = temp_root("discover");
    write_case(&root, false);

    let cases = discover_corpus(&root).expect("valid corpus");
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].id, "cargo/schema-enum-drift/case-a");
    assert_eq!(cases[0].repository, "cargo");
    assert_eq!(cases[0].scenario, "schema-enum-drift");

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn permits_non_semantic_provenance_metadata() {
    let root = temp_root("provenance");
    write_case(&root, false);
    let case = root.join("cargo/schema-enum-drift/case-a");
    fs::write(
        case.join("provenance.json"),
        "{\"schema\":\"chirograph-benchmark-provenance-v1\"}\n",
    )
    .expect("write provenance metadata");

    let cases = discover_corpus(&root).expect("provenance metadata is allowed");
    assert_eq!(cases.len(), 1);

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn rejects_executable_case_glue() {
    let root = temp_root("glue");
    write_case(&root, true);

    assert!(discover_corpus(&root).is_err());

    fs::remove_dir_all(root).expect("remove temp root");
}
