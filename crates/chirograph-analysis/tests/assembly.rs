use std::collections::BTreeSet;

use chirograph_analysis::{
    AnalysisSourceContext, CandidateEvidence, CandidateMechanism, RepresentationCandidate,
    SemanticPath, assemble_contract_graph,
};
use chirograph_core::alignment::AlignmentState;
use chirograph_core::model::{
    ContractFacet, ContractId, RepresentationId, RepresentationKind, Revision,
};

fn context() -> AnalysisSourceContext {
    AnalysisSourceContext::github(
        "acme/fixture-project",
        Revision::Exact("0123456789abcdef0123456789abcdef01234567".into()),
    )
    .unwrap()
}

fn candidate(
    kind: RepresentationKind,
    identity: &str,
    locator: &str,
    path: &[&str],
    values: &[&str],
) -> RepresentationCandidate {
    let context = context();
    let mechanisms = match kind {
        RepresentationKind::SourceCode => BTreeSet::from([
            CandidateMechanism::RustSerializedField,
            CandidateMechanism::RustTypeReference,
            CandidateMechanism::RustClosedValueSet,
        ]),
        RepresentationKind::Schema => BTreeSet::from([
            CandidateMechanism::JsonSchemaProperty,
            CandidateMechanism::JsonSchemaReference,
            CandidateMechanism::JsonSchemaClosedValueSet,
        ]),
        _ => panic!("test candidate kind must be source or schema"),
    };
    RepresentationCandidate::new(
        kind,
        identity,
        locator,
        BTreeSet::from([ContractFacet::Structural]),
        SemanticPath::new(path.iter().copied()).unwrap(),
        Some(values.iter().map(|value| (*value).to_owned()).collect()),
        mechanisms,
        vec![CandidateEvidence {
            source: context.source,
            revision: context.revision,
            locator: format!("{locator}#evidence"),
            fact: format!("closed values for {}", path.join(".")),
        }],
    )
    .unwrap()
}

#[test]
fn unique_differing_pair_promotes_one_contract_and_confirmed_alignments() {
    let context = context();
    let rust = candidate(
        RepresentationKind::SourceCode,
        "fixture::DebugInfo",
        "src/lib.rs",
        &["profile", "debug-info"],
        &["none", "full"],
    );
    let schema = candidate(
        RepresentationKind::Schema,
        "#/$defs/DebugInfo",
        "schema.json",
        &["profile", "debug-info"],
        &["None", "Full"],
    );

    let assembly = assemble_contract_graph(&context, &[schema, rust]).unwrap();
    assembly.graph.validate().unwrap();
    assembly
        .alignments
        .validate_against(&assembly.graph)
        .unwrap();

    assert_eq!(assembly.graph.contracts.len(), 1);
    assert_eq!(assembly.graph.representations.len(), 2);
    assert_eq!(assembly.decisions.len(), 1);
    assert_eq!(assembly.decisions[0].state, AlignmentState::Confirmed);
    let contract = ContractId::new("fixture-project.profile.debug-info").unwrap();
    assert_eq!(assembly.graph.contracts[0].id, contract);

    for representation in [
        RepresentationId::new("fixture-project.profile.debug-info.implementation").unwrap(),
        RepresentationId::new("fixture-project.profile.debug-info.schema").unwrap(),
    ] {
        assert_eq!(
            assembly
                .alignments
                .state_for(&representation, &contract, ContractFacet::Structural),
            Some(AlignmentState::Confirmed)
        );
    }
}

#[test]
fn equal_closed_values_confirm_identity_without_promoting_drift() {
    let context = context();
    let rust = candidate(
        RepresentationKind::SourceCode,
        "fixture::DebugInfo",
        "src/lib.rs",
        &["profile", "debug-info"],
        &["none", "full"],
    );
    let schema = candidate(
        RepresentationKind::Schema,
        "#/$defs/DebugInfo",
        "schema.json",
        &["profile", "debug-info"],
        &["none", "full"],
    );

    let assembly = assemble_contract_graph(&context, &[rust, schema]).unwrap();
    assert!(assembly.graph.contracts.is_empty());
    assert!(assembly.graph.representations.is_empty());
    assert!(assembly.graph.observations.is_empty());
    assert!(assembly.alignments.claims.is_empty());
    assert_eq!(assembly.decisions.len(), 1);
    assert_eq!(assembly.decisions[0].state, AlignmentState::Confirmed);
}

#[test]
fn ambiguous_or_unrelated_candidates_remain_unpromoted() {
    let context = context();
    let source_a = candidate(
        RepresentationKind::SourceCode,
        "fixture::DebugInfoA",
        "a.rs",
        &["profile", "debug-info"],
        &["none", "full"],
    );
    let source_b = candidate(
        RepresentationKind::SourceCode,
        "fixture::DebugInfoB",
        "b.rs",
        &["profile", "debug-info"],
        &["none", "full"],
    );
    let schema = candidate(
        RepresentationKind::Schema,
        "#/$defs/DebugInfo",
        "schema.json",
        &["profile", "debug-info"],
        &["None", "Full"],
    );
    let unrelated = candidate(
        RepresentationKind::Schema,
        "#/$defs/Other",
        "other.json",
        &["profile", "other"],
        &["One", "Two"],
    );

    let assembly =
        assemble_contract_graph(&context, &[source_a, source_b, schema, unrelated]).unwrap();
    assert!(assembly.graph.contracts.is_empty());
    assert!(assembly.alignments.claims.is_empty());
    assert_eq!(assembly.decisions.len(), 2);
    assert!(
        assembly
            .decisions
            .iter()
            .all(|decision| decision.state == AlignmentState::Unresolved)
    );
}

#[test]
fn assembly_is_stable_under_candidate_input_order() {
    let context = context();
    let rust = candidate(
        RepresentationKind::SourceCode,
        "fixture::DebugInfo",
        "src/lib.rs",
        &["profile", "debug-info"],
        &["none", "full"],
    );
    let schema = candidate(
        RepresentationKind::Schema,
        "#/$defs/DebugInfo",
        "schema.json",
        &["profile", "debug-info"],
        &["None", "Full"],
    );

    let forward = assemble_contract_graph(&context, &[rust.clone(), schema.clone()]).unwrap();
    let reverse = assemble_contract_graph(&context, &[schema, rust]).unwrap();
    assert_eq!(forward, reverse);
}