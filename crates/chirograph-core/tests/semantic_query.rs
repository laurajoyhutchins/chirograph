use chirograph_core::alignment::{
    AlignmentCatalog, AlignmentClaim, AlignmentState, ObservedRepresentation,
};
use chirograph_core::model::{
    AuthorityBasis, AuthorityClaim, ClauseAssertion, ClauseId, ClauseKind, ClauseStance, Contract,
    ContractClause, ContractFacet, ContractGraph, ContractId, NodeRef, Observation, ObservationId,
    Relation, RelationKind, Representation, RepresentationId, RepresentationKind, Revision, Source,
    SourceId, SourceKind,
};
use chirograph_core::query::{QueryError, SemanticQuery};

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

fn clause_id(value: &str) -> ClauseId {
    ClauseId::new(value).expect("valid clause id")
}

fn graph(reverse: bool) -> ContractGraph {
    let review = contract_id("review-status");
    let other = contract_id("other-contract");
    let runtime = representation_id("runtime-status");
    let schema = representation_id("schema-status");
    let docs = representation_id("docs-status");
    let other_runtime = representation_id("other-runtime");
    let review_clause = clause_id("review-wire-value");
    let other_clause = clause_id("other-clause");

    let mut graph = ContractGraph {
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
                id: review.clone(),
                name: "review status".into(),
                facets: vec![ContractFacet::Structural, ContractFacet::Semantic],
            },
            Contract {
                id: other.clone(),
                name: "other".into(),
                facets: vec![ContractFacet::Semantic],
            },
        ],
        representations: vec![
            Representation {
                id: runtime.clone(),
                contract: review.clone(),
                source: source_id("runtime"),
                kind: RepresentationKind::ExecutableSurface,
                locator: "GET /status".into(),
                facets: vec![ContractFacet::Semantic],
            },
            Representation {
                id: schema.clone(),
                contract: review.clone(),
                source: source_id("schema"),
                kind: RepresentationKind::Schema,
                locator: "components.schemas.ReviewStatus".into(),
                facets: vec![ContractFacet::Structural, ContractFacet::Semantic],
            },
            Representation {
                id: docs.clone(),
                contract: review.clone(),
                source: source_id("docs"),
                kind: RepresentationKind::Documentation,
                locator: "Review status".into(),
                facets: vec![ContractFacet::Semantic],
            },
            Representation {
                id: other_runtime.clone(),
                contract: other.clone(),
                source: source_id("runtime"),
                kind: RepresentationKind::ExecutableSurface,
                locator: "GET /other".into(),
                facets: vec![ContractFacet::Semantic],
            },
        ],
        observations: vec![
            Observation {
                id: observation_id("obs-runtime"),
                source: source_id("runtime"),
                revision: Revision::Exact("runtime@abc".into()),
                locator: "GET /status".into(),
                fact: "pending review serializes as pending-review".into(),
            },
            Observation {
                id: observation_id("obs-schema"),
                source: source_id("schema"),
                revision: Revision::Exact("git@abc".into()),
                locator: "ReviewStatus".into(),
                fact: "schema enum contains pending-review".into(),
            },
            Observation {
                id: observation_id("obs-docs"),
                source: source_id("docs"),
                revision: Revision::Exact("git@abc".into()),
                locator: "Review status".into(),
                fact: "example contains PendingReview".into(),
            },
            Observation {
                id: observation_id("obs-unlinked"),
                source: source_id("docs"),
                revision: Revision::Exact("git@abc".into()),
                locator: "unrelated".into(),
                fact: "unrelated observation from same source".into(),
            },
        ],
        clauses: vec![
            ContractClause {
                id: review_clause.clone(),
                contract: review.clone(),
                facet: ContractFacet::Semantic,
                kind: ClauseKind::Guarantee,
                statement: "pending review serializes as pending-review".into(),
            },
            ContractClause {
                id: other_clause.clone(),
                contract: other.clone(),
                facet: ContractFacet::Semantic,
                kind: ClauseKind::Guarantee,
                statement: "other contract remains stable".into(),
            },
        ],
        clause_assertions: vec![
            ClauseAssertion {
                clause: review_clause.clone(),
                representation: runtime.clone(),
                stance: ClauseStance::Supports,
                evidence: vec![observation_id("obs-runtime")],
            },
            ClauseAssertion {
                clause: review_clause,
                representation: schema.clone(),
                stance: ClauseStance::Supports,
                evidence: vec![observation_id("obs-schema")],
            },
            ClauseAssertion {
                clause: clause_id("review-wire-value"),
                representation: docs.clone(),
                stance: ClauseStance::Contradicts,
                evidence: vec![observation_id("obs-docs")],
            },
            ClauseAssertion {
                clause: other_clause,
                representation: other_runtime,
                stance: ClauseStance::Supports,
                evidence: vec![observation_id("obs-runtime")],
            },
        ],
        relations: vec![Relation {
            from: NodeRef::Representation(schema.clone()),
            to: NodeRef::Contract(review.clone()),
            kind: RelationKind::Defines,
            basis: vec![observation_id("obs-schema")],
        }],
        authority_claims: vec![
            AuthorityClaim {
                contract: review.clone(),
                representation: schema.clone(),
                facet: ContractFacet::Structural,
                basis: AuthorityBasis::ExplicitDeclaration,
                evidence: vec![observation_id("obs-schema")],
            },
            AuthorityClaim {
                contract: review,
                representation: runtime,
                facet: ContractFacet::Semantic,
                basis: AuthorityBasis::ObservedBehavior,
                evidence: vec![observation_id("obs-runtime")],
            },
        ],
    };

    if reverse {
        graph.sources.reverse();
        graph.contracts.reverse();
        graph.representations.reverse();
        graph.observations.reverse();
        graph.clauses.reverse();
        graph.clause_assertions.reverse();
        graph.relations.reverse();
        graph.authority_claims.reverse();
    }
    graph
}

fn alignments(reverse: bool) -> AlignmentCatalog {
    let mut catalog = AlignmentCatalog {
        representations: vec![ObservedRepresentation {
            id: representation_id("candidate-example"),
            source: source_id("docs"),
            kind: RepresentationKind::Documentation,
            locator: "Candidate example".into(),
        }],
        claims: vec![
            AlignmentClaim {
                representation: representation_id("candidate-example"),
                contract: contract_id("review-status"),
                facet: ContractFacet::Semantic,
                state: AlignmentState::Unresolved,
                evidence: vec![observation_id("obs-docs")],
            },
            AlignmentClaim {
                representation: representation_id("candidate-example"),
                contract: contract_id("review-status"),
                facet: ContractFacet::Structural,
                state: AlignmentState::Rejected,
                evidence: vec![observation_id("obs-docs")],
            },
        ],
    };
    if reverse {
        catalog.representations.reverse();
        catalog.claims.reverse();
    }
    catalog
}

#[test]
fn semantic_queries_are_stable_under_input_reordering() {
    let graph_a = graph(false);
    let graph_b = graph(true);
    let alignments_a = alignments(false);
    let alignments_b = alignments(true);
    let query_a = SemanticQuery::with_alignments(&graph_a, &alignments_a).expect("valid query");
    let query_b = SemanticQuery::with_alignments(&graph_b, &alignments_b).expect("valid query");

    let contract_ids_a: Vec<_> = query_a
        .contracts()
        .into_iter()
        .map(|contract| contract.id.as_str().to_owned())
        .collect();
    let contract_ids_b: Vec<_> = query_b
        .contracts()
        .into_iter()
        .map(|contract| contract.id.as_str().to_owned())
        .collect();
    assert_eq!(contract_ids_a, contract_ids_b);

    let representation_ids_a: Vec<_> = query_a
        .representations_for(&contract_id("review-status"))
        .expect("known contract")
        .into_iter()
        .map(|representation| representation.id.as_str().to_owned())
        .collect();
    let representation_ids_b: Vec<_> = query_b
        .representations_for(&contract_id("review-status"))
        .expect("known contract")
        .into_iter()
        .map(|representation| representation.id.as_str().to_owned())
        .collect();
    assert_eq!(representation_ids_a, representation_ids_b);

    let alignment_states_a: Vec<_> = query_a
        .alignments_for(&representation_id("candidate-example"))
        .iter()
        .map(|claim| (claim.facet, claim.state))
        .collect();
    let alignment_states_b: Vec<_> = query_b
        .alignments_for(&representation_id("candidate-example"))
        .iter()
        .map(|claim| (claim.facet, claim.state))
        .collect();
    assert_eq!(alignment_states_a, alignment_states_b);
}

#[test]
fn contestations_preserve_supporting_and_contradicting_representations() {
    let graph = graph(false);
    let query = SemanticQuery::new(&graph).expect("valid query");
    let contestations = query.contestations();

    assert_eq!(contestations.len(), 1);
    assert_eq!(contestations[0].clause, clause_id("review-wire-value"));
    let supporting: Vec<_> = contestations[0]
        .supporting_representations
        .iter()
        .map(RepresentationId::as_str)
        .collect();
    assert_eq!(supporting, vec!["runtime-status", "schema-status"]);
    let contradicting: Vec<_> = contestations[0]
        .contradicting_representations
        .iter()
        .map(RepresentationId::as_str)
        .collect();
    assert_eq!(contradicting, vec!["docs-status"]);
}

#[test]
fn authority_query_returns_all_matching_claims_without_ranking() {
    let graph = graph(false);
    let query = SemanticQuery::new(&graph).expect("valid query");

    let claims = query
        .authority_for(&contract_id("review-status"), ContractFacet::Semantic)
        .expect("known contract");
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].representation.as_str(), "runtime-status");
    assert_eq!(claims[0].basis, AuthorityBasis::ObservedBehavior);
}

#[test]
fn evidence_for_contract_uses_explicit_provenance_edges_only() {
    let graph = graph(false);
    let query = SemanticQuery::new(&graph).expect("valid query");

    let evidence = query
        .evidence_for(&contract_id("review-status"))
        .expect("known contract");
    let ids: Vec<_> = evidence
        .iter()
        .map(|observation| observation.id.as_str())
        .collect();
    assert_eq!(ids, vec!["obs-docs", "obs-runtime", "obs-schema"]);
    assert!(!ids.contains(&"obs-unlinked"));
}

#[test]
fn clauses_and_representations_for_unknown_contract_fail_explicitly() {
    let graph = graph(false);
    let query = SemanticQuery::new(&graph).expect("valid query");
    let missing = contract_id("missing");

    assert_eq!(
        query.clauses_for(&missing),
        Err(QueryError::UnknownContract(missing.clone()))
    );
    assert_eq!(
        query.representations_for(&missing),
        Err(QueryError::UnknownContract(missing))
    );
}

#[test]
fn alignment_query_returns_recorded_states_without_resolving_them() {
    let graph = graph(false);
    let alignments = alignments(false);
    let query = SemanticQuery::with_alignments(&graph, &alignments).expect("valid query");

    let claims = query.alignments_for(&representation_id("candidate-example"));
    assert_eq!(claims.len(), 2);
    assert_eq!(claims[0].facet, ContractFacet::Structural);
    assert_eq!(claims[0].state, AlignmentState::Rejected);
    assert_eq!(claims[1].facet, ContractFacet::Semantic);
    assert_eq!(claims[1].state, AlignmentState::Unresolved);
}
