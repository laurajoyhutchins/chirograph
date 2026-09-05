use std::collections::BTreeSet;

use chirograph_analysis::{
    AnalysisSourceContext, CandidateEvidence, CandidateMechanism, RepresentationCandidate,
    SemanticPath, align_candidates,
};
use chirograph_core::alignment::AlignmentState;
use chirograph_core::model::{ContractFacet, RepresentationKind, Revision};

fn context() -> AnalysisSourceContext {
    AnalysisSourceContext::github(
        "acme/fixture-project",
        Revision::Exact("0123456789abcdef0123456789abcdef01234567".into()),
    )
    .unwrap()
}

fn candidate(kind: RepresentationKind, identity: &str, path: &[&str], values: &[&str]) -> RepresentationCandidate {
    let context = context();
    let mechanisms = match kind {
        RepresentationKind::SourceCode => BTreeSet::from([
            CandidateMechanism::RustSerializedField,
            CandidateMechanism::RustTypeReference,
            CandidateMechanism::RustClosedValueSet,
        ]),
        RepresentationKind::Schema => BTreeSet::from([
            CandidateMechanism::JsonSchemaProperty,
            CandidateMechanism::JsonSchemaClosedValueSet,
        ]),
        _ => panic!("test candidate kind must be source or schema"),
    };
    RepresentationCandidate::new(
        kind,
        identity,
        format!("{identity}.fixture"),
        BTreeSet::from([ContractFacet::Structural]),
        SemanticPath::new(path.iter().copied()).unwrap(),
        Some(values.iter().map(|value| (*value).to_owned()).collect()),
        mechanisms,
        vec![CandidateEvidence {
            source: context.source,
            revision: context.revision,
            locator: format!("{identity}.fixture#evidence"),
            fact: format!("serialized path {}", path.join(".")),
        }],
    )
    .unwrap()
}

#[test]
fn unique_exact_path_pair_is_confirmed_even_when_values_agree() {
    let rust = candidate(
        RepresentationKind::SourceCode,
        "fixture::DebugInfo",
        &["profile", "debug-info"],
        &["none", "full"],
    );
    let schema = candidate(
        RepresentationKind::Schema,
        "#/$defs/DebugInfo",
        &["profile", "debug-info"],
        &["none", "full"],
    );

    let forward = align_candidates(&[rust.clone(), schema.clone()]).unwrap();
    let reverse = align_candidates(&[schema, rust]).unwrap();
    assert_eq!(forward, reverse);
    assert_eq!(forward.len(), 1);
    assert_eq!(forward[0].state, AlignmentState::Confirmed);
    assert_eq!(forward[0].semantic_path.dotted(), "profile.debug-info");
    assert_eq!(forward[0].candidates.len(), 2);
    assert!(!forward[0].evidence.is_empty());
}

#[test]
fn one_to_many_and_one_sided_paths_stay_unresolved() {
    let source_a = candidate(
        RepresentationKind::SourceCode,
        "fixture::A",
        &["profile", "debug-info"],
        &["none", "full"],
    );
    let source_b = candidate(
        RepresentationKind::SourceCode,
        "fixture::B",
        &["profile", "debug-info"],
        &["none", "full"],
    );
    let schema = candidate(
        RepresentationKind::Schema,
        "#/$defs/DebugInfo",
        &["profile", "debug-info"],
        &["none", "full"],
    );
    let schema_only = candidate(
        RepresentationKind::Schema,
        "#/$defs/Other",
        &["profile", "other"],
        &["one", "two"],
    );

    let decisions = align_candidates(&[source_a, source_b, schema, schema_only]).unwrap();
    assert_eq!(decisions.len(), 2);
    assert!(decisions.iter().all(|decision| decision.state == AlignmentState::Unresolved));
}