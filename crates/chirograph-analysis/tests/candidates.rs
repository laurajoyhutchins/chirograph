use std::collections::{BTreeSet, HashSet};

use chirograph_analysis::{
    CandidateEvidence, CandidateMechanism, RepresentationCandidate, SemanticPath,
};
use chirograph_core::model::{ContractFacet, RepresentationKind, Revision, SourceId};

fn evidence(locator: &str, fact: &str) -> CandidateEvidence {
    CandidateEvidence {
        source: SourceId::new("github:acme/fixture-project").unwrap(),
        revision: Revision::Exact("0123456789abcdef0123456789abcdef01234567".into()),
        locator: locator.into(),
        fact: fact.into(),
    }
}

#[test]
fn semantic_paths_require_nonempty_consumer_segments() {
    assert!(SemanticPath::new(["profile", "debug-info"]).is_ok());
    assert!(SemanticPath::new(["profile", ""]).is_err());
    assert!(SemanticPath::new([" profile "]).is_err());
}

#[test]
fn candidate_preserves_identity_facets_revision_and_stable_evidence() {
    let path = SemanticPath::new(["profile", "debug-info"]).unwrap();
    let candidate = RepresentationCandidate::new(
        RepresentationKind::SourceCode,
        "fixture::DebugInfo",
        "src/lib.rs",
        BTreeSet::from([ContractFacet::Structural]),
        path,
        Some(BTreeSet::from(["full".into(), "none".into()])),
        BTreeSet::from([
            CandidateMechanism::RustSerializedField,
            CandidateMechanism::RustClosedValueSet,
        ]),
        vec![
            evidence("src/lib.rs#L20", "enum values"),
            evidence("src/lib.rs#L10", "field path"),
        ],
    )
    .unwrap();

    assert_eq!(candidate.qualified_local_identity, "fixture::DebugInfo");
    assert_eq!(
        candidate.facets,
        BTreeSet::from([ContractFacet::Structural])
    );
    assert_eq!(candidate.evidence[0].locator, "src/lib.rs#L10");
    assert_eq!(candidate.evidence[1].locator, "src/lib.rs#L20");
    assert!(matches!(candidate.evidence[0].revision, Revision::Exact(_)));
}

#[test]
fn closed_values_require_matching_mechanism_evidence() {
    let path = SemanticPath::new(["profile", "debug-info"]).unwrap();
    let result = RepresentationCandidate::new(
        RepresentationKind::Schema,
        "fixture.schema.DebugInfo",
        "schema.json",
        BTreeSet::from([ContractFacet::Structural]),
        path,
        Some(BTreeSet::from(["Full".into(), "None".into()])),
        BTreeSet::from([CandidateMechanism::JsonSchemaProperty]),
        vec![evidence("schema.json#/$defs/DebugInfo", "closed enum")],
    );
    assert!(result.is_err());
}

#[test]
fn candidate_mechanisms_are_set_like_and_deterministic() {
    let mechanisms = BTreeSet::from([
        CandidateMechanism::JsonSchemaClosedValueSet,
        CandidateMechanism::JsonSchemaProperty,
    ]);
    assert_eq!(mechanisms.len(), 2);
    let hash = mechanisms.iter().collect::<HashSet<_>>();
    assert_eq!(hash.len(), 2);
}
