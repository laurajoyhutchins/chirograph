use chirograph_benchmark::model::{
    GOLDEN_SCHEMA, GoldenAuthorityClaimV1, GoldenClauseV1, GoldenContractV1, GoldenFindingV1,
    GoldenLifecycleV1, GoldenNonContractV1, GoldenRelationshipV1, GoldenRepresentationV1, GoldenV1,
};
use chirograph_benchmark::score::score_case;
use chirograph_core::graph_json::{
    GRAPH_JSON_SCHEMA, GraphAuthorityClaimV1, GraphClauseAssessmentV1, GraphClauseV1,
    GraphContractV1, GraphJsonV1, GraphLifecycleV1, GraphNodeRefV1, GraphRelationV1,
    GraphRepresentationV1,
};

fn node(kind: &str, id: &str) -> GraphNodeRefV1 {
    GraphNodeRefV1 {
        kind: kind.to_owned(),
        id: id.to_owned(),
    }
}

fn contract(id: &str) -> GoldenContractV1 {
    GoldenContractV1 {
        id: id.to_owned(),
        facets: vec!["structural".to_owned()],
    }
}

fn observed_contract(id: &str) -> GraphContractV1 {
    GraphContractV1 {
        id: id.to_owned(),
        name: id.to_owned(),
        facets: vec!["structural".to_owned()],
    }
}

fn golden() -> GoldenV1 {
    GoldenV1 {
        schema: GOLDEN_SCHEMA.to_owned(),
        contracts: Vec::new(),
        representations: Vec::new(),
        authority_claims: Vec::new(),
        relationships: Vec::new(),
        clauses: Vec::new(),
        lifecycle: Vec::new(),
        expected_findings: Vec::new(),
        non_contracts: Vec::new(),
    }
}

fn observed() -> GraphJsonV1 {
    GraphJsonV1 {
        schema: GRAPH_JSON_SCHEMA.to_owned(),
        contracts: Vec::new(),
        representations: Vec::new(),
        relations: Vec::new(),
        authority_claims: Vec::new(),
        clauses: Vec::new(),
        clause_assessments: Vec::new(),
        lifecycle: Vec::new(),
    }
}

#[test]
fn scores_perfect_missing_false_and_zero_contract_emission() {
    let mut expected = golden();
    expected.contracts = vec![contract("contract.a"), contract("contract.b")];

    let mut actual = observed();
    actual.contracts = vec![
        observed_contract("contract.a"),
        observed_contract("contract.b"),
    ];
    let perfect = score_case(&expected, &actual);
    assert_eq!(perfect.contract_precision.ratio, Some(1.0));
    assert_eq!(perfect.contract_recall.ratio, Some(1.0));
    assert_eq!(perfect.contract_f1, Some(1.0));
    assert_eq!(perfect.false_contract_rate.ratio, Some(0.0));
    assert_eq!(perfect.contract_inflation, 1.0);

    actual.contracts = vec![
        observed_contract("contract.a"),
        observed_contract("contract.false"),
    ];
    let mixed = score_case(&expected, &actual);
    assert_eq!(mixed.contract_precision.ratio, Some(0.5));
    assert_eq!(mixed.contract_recall.ratio, Some(0.5));
    assert_eq!(mixed.contract_f1, Some(0.5));
    assert_eq!(mixed.false_contract_rate.ratio, Some(0.5));
    assert_eq!(mixed.contract_inflation, 1.0);

    actual.contracts.clear();
    let zero = score_case(&expected, &actual);
    assert_eq!(zero.contract_precision.ratio, None);
    assert_eq!(zero.contract_recall.ratio, Some(0.0));
    assert_eq!(zero.contract_f1, None);
    assert_eq!(zero.false_contract_rate.ratio, None);
    assert_eq!(zero.contract_inflation, 0.0);
}

#[test]
fn scores_authority_and_relationships_by_exact_typed_identity() {
    let mut expected = golden();
    expected.contracts = vec![contract("contract.a")];
    expected.representations = vec![GoldenRepresentationV1 {
        id: "rep.a".to_owned(),
        contract: "contract.a".to_owned(),
        kind: "schema".to_owned(),
        locator: "schema.json".to_owned(),
        facets: vec![
            "structural".to_owned(),
            "semantic".to_owned(),
            "verification".to_owned(),
        ],
    }];
    for facet in ["structural", "semantic", "verification"] {
        expected.authority_claims.push(GoldenAuthorityClaimV1 {
            contract: "contract.a".to_owned(),
            facet: facet.to_owned(),
            representation: "rep.a".to_owned(),
            basis: "mechanical-enforcement".to_owned(),
        });
    }
    expected.relationships = vec![GoldenRelationshipV1 {
        from: node("contract", "contract.a"),
        kind: "projects".to_owned(),
        to: node("representation", "rep.a"),
    }];

    let mut actual = observed();
    actual.contracts = vec![observed_contract("contract.a")];
    actual.authority_claims = vec![
        GraphAuthorityClaimV1 {
            contract: "contract.a".to_owned(),
            representation: "rep.a".to_owned(),
            facet: "structural".to_owned(),
            basis: "inference".to_owned(),
        },
        GraphAuthorityClaimV1 {
            contract: "contract.a".to_owned(),
            representation: "rep.a".to_owned(),
            facet: "semantic".to_owned(),
            basis: "documentation".to_owned(),
        },
        GraphAuthorityClaimV1 {
            contract: "contract.a".to_owned(),
            representation: "rep.wrong".to_owned(),
            facet: "verification".to_owned(),
            basis: "mechanical-enforcement".to_owned(),
        },
    ];
    actual.relations = vec![
        GraphRelationV1 {
            from: node("contract", "contract.a"),
            kind: "projects".to_owned(),
            to: node("representation", "rep.a"),
        },
        GraphRelationV1 {
            from: node("contract", "contract.a"),
            kind: "implements".to_owned(),
            to: node("representation", "rep.a"),
        },
    ];

    let score = score_case(&expected, &actual);
    assert_eq!(score.authority_correctness.numerator, 2);
    assert_eq!(score.authority_correctness.denominator, 3);
    assert_eq!(score.authority_correctness.ratio, Some(2.0 / 3.0));
    assert_eq!(score.relationship_precision.ratio, Some(0.5));
    assert_eq!(score.relationship_recall.ratio, Some(1.0));
}

#[test]
fn scores_lifecycle_findings_and_known_negative_diagnostics() {
    let mut expected = golden();
    expected.contracts = vec![contract("contract.a")];
    expected.clauses = vec![GoldenClauseV1 {
        id: "clause.a".to_owned(),
        contract: "contract.a".to_owned(),
        facet: "structural".to_owned(),
        kind: "invariant".to_owned(),
        statement: "A must hold.".to_owned(),
    }];
    expected.expected_findings = vec![GoldenFindingV1::ContestedClause {
        clause: "clause.a".to_owned(),
    }];
    expected.lifecycle = vec![GoldenLifecycleV1 {
        subject: node("contract", "contract.a"),
        status: "active".to_owned(),
    }];
    expected.non_contracts = vec![GoldenNonContractV1 {
        locator: "src/detail.rs".to_owned(),
        reason: "implementation-detail".to_owned(),
    }];

    let mut actual = observed();
    actual.contracts = vec![
        observed_contract("contract.a"),
        observed_contract("contract.false"),
    ];
    actual.representations = vec![GraphRepresentationV1 {
        id: "rep.false".to_owned(),
        contract: "contract.false".to_owned(),
        kind: "source-code".to_owned(),
        locator: "src/detail.rs".to_owned(),
        facets: vec!["structural".to_owned()],
    }];
    actual.clauses = vec![GraphClauseV1 {
        id: "clause.a".to_owned(),
        contract: "contract.a".to_owned(),
        facet: "structural".to_owned(),
        kind: "invariant".to_owned(),
        statement: "A must hold.".to_owned(),
    }];
    actual.clause_assessments = vec![
        GraphClauseAssessmentV1 {
            clause: "clause.a".to_owned(),
            status: "contested".to_owned(),
            supporting_representations: Vec::new(),
            contradicting_representations: vec!["rep.false".to_owned()],
        },
        GraphClauseAssessmentV1 {
            clause: "clause.extra".to_owned(),
            status: "contested".to_owned(),
            supporting_representations: Vec::new(),
            contradicting_representations: vec!["rep.false".to_owned()],
        },
    ];

    let unavailable = score_case(&expected, &actual);
    assert_eq!(unavailable.lifecycle_correctness, None);
    assert!(
        unavailable
            .diagnostics
            .contains(&"lifecycle_not_observed".to_owned())
    );
    assert_eq!(unavailable.finding_precision.ratio, Some(0.5));
    assert_eq!(unavailable.finding_recall.ratio, Some(1.0));
    assert!(
        unavailable
            .diagnostics
            .contains(&"known_non_contract_promoted".to_owned())
    );

    actual.lifecycle = vec![GraphLifecycleV1 {
        subject: node("contract", "contract.a"),
        status: "active".to_owned(),
    }];
    let observed_lifecycle = score_case(&expected, &actual);
    assert_eq!(
        observed_lifecycle
            .lifecycle_correctness
            .as_ref()
            .and_then(|metric| metric.ratio),
        Some(1.0)
    );
}
