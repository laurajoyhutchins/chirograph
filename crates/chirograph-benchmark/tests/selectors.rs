use std::path::PathBuf;

use chirograph_benchmark::model::{
    BenchmarkCase, GOLDEN_SCHEMA, GoldenV1, SPECIMEN_SCHEMA, SpecimenV1, UpstreamV1,
};
use chirograph_benchmark::selector::select_cases;

fn case(id: &str, repository: &str, scenario: &str) -> BenchmarkCase {
    let root = PathBuf::from(id);
    BenchmarkCase {
        id: id.to_owned(),
        repository: repository.to_owned(),
        scenario: scenario.to_owned(),
        root: root.clone(),
        fixture_dir: root.join("fixture"),
        specimen_path: root.join("specimen.yaml"),
        golden_path: root.join("golden.yaml"),
        specimen: SpecimenV1 {
            schema: SPECIMEN_SCHEMA.to_owned(),
            id: id.to_owned(),
            repository: repository.to_owned(),
            scenario: scenario.to_owned(),
            upstream: UpstreamV1 {
                repository: format!("example/{repository}"),
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

fn ids(cases: Vec<&BenchmarkCase>) -> Vec<&str> {
    cases.into_iter().map(|case| case.id.as_str()).collect()
}

fn fixture_cases() -> Vec<BenchmarkCase> {
    vec![
        case(
            "kafka/message-spec-generation/case-b",
            "kafka",
            "message-spec-generation",
        ),
        case(
            "cargo/schema-enum-drift/case-b",
            "cargo",
            "schema-enum-drift",
        ),
        case(
            "cargo/schema-enum-drift/case-a",
            "cargo",
            "schema-enum-drift",
        ),
    ]
}

#[test]
fn selects_supported_dimensions_in_lexical_order() {
    let cases = fixture_cases();

    assert_eq!(
        ids(select_cases(&cases, "all").expect("all cases")),
        vec![
            "cargo/schema-enum-drift/case-a",
            "cargo/schema-enum-drift/case-b",
            "kafka/message-spec-generation/case-b",
        ]
    );
    assert_eq!(
        ids(select_cases(&cases, "cargo").expect("repository")),
        vec![
            "cargo/schema-enum-drift/case-a",
            "cargo/schema-enum-drift/case-b",
        ]
    );
    assert_eq!(
        ids(select_cases(&cases, "scenario:schema-enum-drift").expect("scenario")),
        vec![
            "cargo/schema-enum-drift/case-a",
            "cargo/schema-enum-drift/case-b",
        ]
    );
    assert_eq!(
        ids(select_cases(&cases, "cargo/schema-enum-drift").expect("repository/scenario")),
        vec![
            "cargo/schema-enum-drift/case-a",
            "cargo/schema-enum-drift/case-b",
        ]
    );
    assert_eq!(
        ids(select_cases(&cases, "cargo/schema-enum-drift/case-a").expect("exact case")),
        vec!["cargo/schema-enum-drift/case-a"]
    );
}

#[test]
fn rejects_unknown_zero_match_and_invalid_selectors() {
    let cases = fixture_cases();

    for selector in [
        "unknown",
        "scenario:unknown",
        "cargo/unknown",
        "cargo/schema-enum-drift/unknown",
        "cargo/schema-enum-drift/case-a/extra",
        "",
    ] {
        assert!(
            select_cases(&cases, selector).is_err(),
            "selector should fail: {selector:?}"
        );
    }
}
