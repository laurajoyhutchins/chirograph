use chirograph_core::model::*;

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

fn clause_graph() -> ContractGraph {
    let contract = contract_id("git.update-ref");
    let help_source = source_id("git-help");
    let implementation_source = source_id("git-source");
    let help_representation = representation_id("git-update-ref-help");
    let implementation_representation = representation_id("git-update-ref-source");
    let help_observation = observation_id("obs-help-old-oid");
    let implementation_observation = observation_id("obs-source-old-oid");
    let clause = clause_id("old-oid-must-match");

    let mut graph = ContractGraph::default();
    graph.sources = vec![
        Source {
            id: help_source.clone(),
            kind: SourceKind::Executable,
            locator: "/usr/bin/git".into(),
        },
        Source {
            id: implementation_source.clone(),
            kind: SourceKind::Repository,
            locator: "refs.c".into(),
        },
    ];
    graph.contracts = vec![Contract {
        id: contract.clone(),
        name: "git update-ref".into(),
        facets: vec![ContractFacet::Concurrency],
    }];
    graph.representations = vec![
        Representation {
            id: help_representation.clone(),
            contract: contract.clone(),
            source: help_source.clone(),
            kind: RepresentationKind::ExecutableSurface,
            locator: "git update-ref --help".into(),
            facets: vec![ContractFacet::Concurrency],
        },
        Representation {
            id: implementation_representation.clone(),
            contract: contract.clone(),
            source: implementation_source.clone(),
            kind: RepresentationKind::SourceCode,
            locator: "refs.c:update_ref".into(),
            facets: vec![ContractFacet::Concurrency],
        },
    ];
    graph.observations = vec![
        Observation {
            id: help_observation.clone(),
            source: help_source,
            revision: Revision::Exact("git 2.47.3".into()),
            locator: "update-ref help".into(),
            fact: "old object id must match the current ref value".into(),
        },
        Observation {
            id: implementation_observation.clone(),
            source: implementation_source,
            revision: Revision::Exact("git 2.47.3".into()),
            locator: "refs.c:update_ref".into(),
            fact: "implementation rejects an old object id mismatch".into(),
        },
    ];
    graph.clauses = vec![ContractClause {
        id: clause.clone(),
        contract: contract.clone(),
        facet: ContractFacet::Concurrency,
        kind: ClauseKind::Requirement,
        statement: "the supplied old object id must match the current ref value".into(),
    }];
    graph.clause_assertions = vec![ClauseAssertion {
        clause,
        representation: help_representation,
        stance: ClauseStance::Supports,
        evidence: vec![help_observation],
    }];
    graph
}

#[test]
fn accepts_clause_with_source_backed_support() {
    clause_graph().validate().expect("clause graph should be valid");
}

#[test]
fn rejects_clause_without_supporting_representation() {
    let mut graph = clause_graph();
    let clause = graph.clauses[0].id.clone();
    graph.clause_assertions.clear();

    assert_eq!(
        graph.validate(),
        Err(ModelError::ClauseSupportRequired(clause))
    );
}

#[test]
fn assessment_exposes_cross_representation_disagreement() {
    let mut graph = clause_graph();
    let clause = graph.clauses[0].id.clone();
    let implementation = representation_id("git-update-ref-source");
    graph.clause_assertions.push(ClauseAssertion {
        clause: clause.clone(),
        representation: implementation.clone(),
        stance: ClauseStance::Contradicts,
        evidence: vec![observation_id("obs-source-old-oid")],
    });
    graph.validate().expect("contested clause should still be valid evidence");

    assert_eq!(
        graph.assess_clause(&clause),
        Ok(ClauseAssessment {
            clause,
            status: ClauseStatus::Contested,
            supporting_representations: vec![representation_id("git-update-ref-help")],
            contradicting_representations: vec![implementation],
        })
    );
}

#[test]
fn rejects_assertion_from_representation_that_does_not_cover_clause_facet() {
    let mut graph = clause_graph();
    let representation = representation_id("git-update-ref-source");
    graph.representations[1].facets.clear();
    graph.clause_assertions.push(ClauseAssertion {
        clause: clause_id("old-oid-must-match"),
        representation: representation.clone(),
        stance: ClauseStance::Contradicts,
        evidence: vec![observation_id("obs-source-old-oid")],
    });

    assert_eq!(
        graph.validate(),
        Err(ModelError::ClauseFacetNotRepresented {
            clause: clause_id("old-oid-must-match"),
            representation,
            facet: ContractFacet::Concurrency,
        })
    );
}
