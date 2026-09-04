use chirograph_core::alignment::{
    AlignmentCatalog, AlignmentClaim, AlignmentError, AlignmentState, ObservedRepresentation,
};
use chirograph_core::model::{
    Contract, ContractFacet, ContractGraph, ContractId, Observation, ObservationId,
    RepresentationId, RepresentationKind, Revision, Source, SourceId, SourceKind,
};

fn contract_id(value: &str) -> ContractId {
    ContractId::new(value).expect("valid contract id")
}

fn representation_id(value: &str) -> RepresentationId {
    RepresentationId::new(value).expect("valid representation id")
}

fn source_id(value: &str) -> SourceId {
    SourceId::new(value).expect("valid source id")
}

fn observation_id(value: &str) -> ObservationId {
    ObservationId::new(value).expect("valid observation id")
}

fn graph() -> ContractGraph {
    ContractGraph {
        sources: vec![
            Source {
                id: source_id("runtime"),
                kind: SourceKind::Executable,
                locator: "runtime://service".into(),
            },
            Source {
                id: source_id("schema"),
                kind: SourceKind::FileSystem,
                locator: "openapi.yaml".into(),
            },
            Source {
                id: source_id("docs"),
                kind: SourceKind::FileSystem,
                locator: "docs/api.md".into(),
            },
        ],
        contracts: vec![
            Contract {
                id: contract_id("review-status"),
                name: "review status".into(),
                facets: vec![ContractFacet::Structural, ContractFacet::Semantic],
            },
            Contract {
                id: contract_id("other-contract"),
                name: "other".into(),
                facets: vec![ContractFacet::Semantic],
            },
        ],
        observations: vec![
            Observation {
                id: observation_id("runtime-status"),
                source: source_id("runtime"),
                revision: Revision::Exact("runtime@abc".into()),
                locator: "GET /status".into(),
                fact: "serializes pending review as pending-review".into(),
            },
            Observation {
                id: observation_id("schema-status"),
                source: source_id("schema"),
                revision: Revision::Exact("git@abc".into()),
                locator: "components.schemas.ReviewStatus".into(),
                fact: "enum contains pending-review".into(),
            },
            Observation {
                id: observation_id("docs-status"),
                source: source_id("docs"),
                revision: Revision::Exact("git@abc".into()),
                locator: "Review status".into(),
                fact: "example contains PendingReview".into(),
            },
        ],
        ..ContractGraph::default()
    }
}

fn observed(id: &str, source: &str, kind: RepresentationKind) -> ObservedRepresentation {
    ObservedRepresentation {
        id: representation_id(id),
        source: source_id(source),
        kind,
        locator: id.into(),
    }
}

fn claim(
    representation: &str,
    contract: &str,
    facet: ContractFacet,
    state: AlignmentState,
    evidence: &[&str],
) -> AlignmentClaim {
    AlignmentClaim {
        representation: representation_id(representation),
        contract: contract_id(contract),
        facet,
        state,
        evidence: evidence.iter().map(|id| observation_id(id)).collect(),
    }
}

#[test]
fn unresolved_alignment_is_not_resolved_by_confirmed_peers() {
    let catalog = AlignmentCatalog {
        representations: vec![
            observed("runtime-status", "runtime", RepresentationKind::ExecutableSurface),
            observed("schema-status", "schema", RepresentationKind::Schema),
            observed("docs-status", "docs", RepresentationKind::Documentation),
        ],
        claims: vec![
            claim(
                "runtime-status",
                "review-status",
                ContractFacet::Semantic,
                AlignmentState::Confirmed,
                &["runtime-status"],
            ),
            claim(
                "schema-status",
                "review-status",
                ContractFacet::Semantic,
                AlignmentState::Confirmed,
                &["schema-status"],
            ),
            claim(
                "docs-status",
                "review-status",
                ContractFacet::Semantic,
                AlignmentState::Unresolved,
                &["docs-status"],
            ),
        ],
    };

    catalog.validate_against(&graph()).expect("catalog is valid");
    assert_eq!(
        catalog.state_for(
            &representation_id("docs-status"),
            &contract_id("review-status"),
            ContractFacet::Semantic,
        ),
        Some(AlignmentState::Unresolved)
    );
}

#[test]
fn rejected_alignment_is_not_overridden_by_confirmed_peers() {
    let catalog = AlignmentCatalog {
        representations: vec![
            observed("runtime-status", "runtime", RepresentationKind::ExecutableSurface),
            observed("schema-status", "schema", RepresentationKind::Schema),
            observed("docs-status", "docs", RepresentationKind::Documentation),
        ],
        claims: vec![
            claim(
                "runtime-status",
                "review-status",
                ContractFacet::Semantic,
                AlignmentState::Confirmed,
                &["runtime-status"],
            ),
            claim(
                "schema-status",
                "review-status",
                ContractFacet::Semantic,
                AlignmentState::Confirmed,
                &["schema-status"],
            ),
            claim(
                "docs-status",
                "review-status",
                ContractFacet::Semantic,
                AlignmentState::Rejected,
                &["docs-status"],
            ),
        ],
    };

    catalog.validate_against(&graph()).expect("catalog is valid");
    assert_eq!(
        catalog.state_for(
            &representation_id("docs-status"),
            &contract_id("review-status"),
            ContractFacet::Semantic,
        ),
        Some(AlignmentState::Rejected)
    );
}

#[test]
fn claims_for_representation_are_deterministically_ordered() {
    let catalog = AlignmentCatalog {
        representations: vec![observed(
            "docs-status",
            "docs",
            RepresentationKind::Documentation,
        )],
        claims: vec![
            claim(
                "docs-status",
                "review-status",
                ContractFacet::Semantic,
                AlignmentState::Unresolved,
                &["docs-status"],
            ),
            claim(
                "docs-status",
                "other-contract",
                ContractFacet::Semantic,
                AlignmentState::Rejected,
                &["docs-status"],
            ),
            claim(
                "docs-status",
                "review-status",
                ContractFacet::Structural,
                AlignmentState::Confirmed,
                &["docs-status"],
            ),
        ],
    };

    catalog.validate_against(&graph()).expect("catalog is valid");
    let ordered = catalog.claims_for(&representation_id("docs-status"));
    let keys: Vec<_> = ordered
        .iter()
        .map(|claim| (claim.contract.as_str(), claim.facet))
        .collect();
    assert_eq!(
        keys,
        vec![
            ("other-contract", ContractFacet::Semantic),
            ("review-status", ContractFacet::Structural),
            ("review-status", ContractFacet::Semantic),
        ]
    );
}

#[test]
fn alignment_claim_requires_evidence() {
    let catalog = AlignmentCatalog {
        representations: vec![observed(
            "docs-status",
            "docs",
            RepresentationKind::Documentation,
        )],
        claims: vec![claim(
            "docs-status",
            "review-status",
            ContractFacet::Semantic,
            AlignmentState::Unresolved,
            &[],
        )],
    };

    assert_eq!(
        catalog.validate_against(&graph()),
        Err(AlignmentError::EvidenceRequired {
            representation: representation_id("docs-status"),
            contract: contract_id("review-status"),
            facet: ContractFacet::Semantic,
        })
    );
}

#[test]
fn alignment_claim_rejects_unknown_observation() {
    let catalog = AlignmentCatalog {
        representations: vec![observed(
            "docs-status",
            "docs",
            RepresentationKind::Documentation,
        )],
        claims: vec![claim(
            "docs-status",
            "review-status",
            ContractFacet::Semantic,
            AlignmentState::Unresolved,
            &["missing"],
        )],
    };

    assert_eq!(
        catalog.validate_against(&graph()),
        Err(AlignmentError::UnknownObservation(observation_id("missing")))
    );
}

#[test]
fn duplicate_alignment_identity_is_rejected_instead_of_counted() {
    let duplicate = claim(
        "docs-status",
        "review-status",
        ContractFacet::Semantic,
        AlignmentState::Unresolved,
        &["docs-status"],
    );
    let catalog = AlignmentCatalog {
        representations: vec![observed(
            "docs-status",
            "docs",
            RepresentationKind::Documentation,
        )],
        claims: vec![duplicate.clone(), duplicate],
    };

    assert_eq!(
        catalog.validate_against(&graph()),
        Err(AlignmentError::DuplicateClaim {
            representation: representation_id("docs-status"),
            contract: contract_id("review-status"),
            facet: ContractFacet::Semantic,
        })
    );
}
