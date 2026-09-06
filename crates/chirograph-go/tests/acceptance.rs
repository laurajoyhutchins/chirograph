use std::fs;
use std::path::{Path, PathBuf};

use chirograph_core::model::{Revision, SourceId};
use chirograph_go::{GoFactKind, extract_go_facts};
use chirograph_tree_sitter::SourceProvenance;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn acquire_pinned(
    fixture_path: &str,
    source: &str,
    revision: &str,
    upstream_path: &str,
) -> chirograph_go::GoExtraction {
    let bytes = fs::read(repository_root().join(fixture_path)).expect("pinned fixture should exist");
    let provenance = SourceProvenance {
        source: SourceId::new(source).expect("source id should be valid"),
        revision: Revision::Exact(revision.to_owned()),
        locator: upstream_path.to_owned(),
        path: upstream_path.to_owned(),
    };

    extract_go_facts(&bytes, provenance).expect("pinned Go specimen should parse")
}

#[test]
fn acquires_pinned_kubernetes_go_source_without_repository_specific_logic() {
    let revision = "82ca8014fabed9e61cf6c14560cdb9f1e4e1d067";
    let path = "staging/src/k8s.io/api/core/v1/types.go";
    let extraction = acquire_pinned(
        "benchmark/kubernetes/go-protobuf-openapi/core-v1-pod/fixture/staging/src/k8s.io/api/core/v1/types.go",
        "github:kubernetes/kubernetes",
        revision,
        path,
    );

    assert!(
        extraction.diagnostics.is_empty(),
        "pinned Kubernetes Go source must parse cleanly"
    );
    for kind in [
        GoFactKind::Package,
        GoFactKind::Type,
        GoFactKind::Struct,
        GoFactKind::Field,
        GoFactKind::Tag,
        GoFactKind::Comment,
    ] {
        assert!(
            extraction.facts.iter().any(|fact| fact.kind == kind),
            "Kubernetes acceptance source is missing generic fact kind {kind:?}"
        );
    }
    assert!(extraction.facts.iter().all(|fact| {
        fact.provenance.source.as_str() == "github:kubernetes/kubernetes"
            && fact.provenance.revision == Revision::Exact(revision.to_owned())
            && fact.provenance.path == path
            && fact.span.start_byte < fact.span.end_byte
    }));
}

#[test]
fn acquires_pinned_temporal_go_source_with_distinct_exact_provenance() {
    let revision = "cd667daadb88d189df0302d8f473858ee7168ce5";
    let path = "tools/tests/test_data.go";
    let extraction = acquire_pinned(
        "benchmark/temporal/multi-dialect-persistence/executions-table/fixture/tools/tests/test_data.go",
        "github:temporalio/temporal",
        revision,
        path,
    );

    assert!(
        extraction.diagnostics.is_empty(),
        "pinned Temporal Go source must parse cleanly"
    );
    assert!(
        extraction
            .facts
            .iter()
            .any(|fact| fact.kind == GoFactKind::Package)
    );
    assert!(
        extraction
            .facts
            .iter()
            .any(|fact| fact.kind == GoFactKind::Const)
    );
    assert!(extraction.facts.iter().all(|fact| {
        fact.provenance.source.as_str() == "github:temporalio/temporal"
            && fact.provenance.revision == Revision::Exact(revision.to_owned())
            && fact.provenance.path == path
            && fact.span.start_byte < fact.span.end_byte
    }));
}
