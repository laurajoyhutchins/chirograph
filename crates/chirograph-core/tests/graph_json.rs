use chirograph_core::graph_json::encode_graph_json;
use chirograph_core::model::{
    AuthorityBasis, AuthorityClaim, ClauseAssertion, ClauseId, ClauseKind, ClauseStance, Contract,
    ContractClause, ContractFacet, ContractGraph, ContractId, NodeRef, Observation, ObservationId,
    Relation, RelationKind, Representation, RepresentationId, RepresentationKind, Revision, Source,
    SourceId, SourceKind,
};

fn id<T>(
    value: &str,
    constructor: impl FnOnce(String) -> Result<T, chirograph_core::model::IdentifierError>,
) -> T {
    constructor(value.to_owned()).expect("valid test id")
}

fn fixture_graph_with_reversed_vectors() -> ContractGraph {
    let source_id = id("source.repo", SourceId::new);
    let contract_id = id("example.contract", ContractId::new);
    let representation_a = id("example.contract.a", RepresentationId::new);
    let representation_b = id("example.contract.b", RepresentationId::new);
    let observation_a = id("observation.a", ObservationId::new);
    let observation_b = id("observation.b", ObservationId::new);
    let clause_id = id("example.contract.rule", ClauseId::new);

    ContractGraph {
        sources: vec![Source {
            id: source_id.clone(),
            kind: SourceKind::Repository,
            locator: "github:example/repo".to_owned(),
        }],
        contracts: vec![Contract {
            id: contract_id.clone(),
            name: "Example contract".to_owned(),
            facets: vec![ContractFacet::Structural],
        }],
        representations: vec![
            Representation {
                id: representation_b.clone(),
                contract: contract_id.clone(),
                source: source_id.clone(),
                kind: RepresentationKind::Schema,
                locator: "schema.json".to_owned(),
                facets: vec![ContractFacet::Structural],
            },
            Representation {
                id: representation_a.clone(),
                contract: contract_id.clone(),
                source: source_id.clone(),
                kind: RepresentationKind::SourceCode,
                locator: "src/lib.rs".to_owned(),
                facets: vec![ContractFacet::Structural],
            },
        ],
        observations: vec![
            Observation {
                id: observation_b.clone(),
                source: source_id.clone(),
                revision: Revision::Exact("rev-b".to_owned()),
                locator: "schema.json:1".to_owned(),
                fact: "schema contradicts rule".to_owned(),
            },
            Observation {
                id: observation_a.clone(),
                source: source_id,
                revision: Revision::Exact("rev-a".to_owned()),
                locator: "src/lib.rs:1".to_owned(),
                fact: "source supports rule".to_owned(),
            },
        ],
        clauses: vec![ContractClause {
            id: clause_id.clone(),
            contract: contract_id.clone(),
            facet: ContractFacet::Structural,
            kind: ClauseKind::Invariant,
            statement: "Representations agree on the structural rule.".to_owned(),
        }],
        clause_assertions: vec![
            ClauseAssertion {
                clause: clause_id.clone(),
                representation: representation_b.clone(),
                stance: ClauseStance::Contradicts,
                evidence: vec![observation_b],
            },
            ClauseAssertion {
                clause: clause_id,
                representation: representation_a.clone(),
                stance: ClauseStance::Supports,
                evidence: vec![observation_a.clone()],
            },
        ],
        relations: vec![Relation {
            from: NodeRef::Representation(representation_a.clone()),
            to: NodeRef::Representation(representation_b),
            kind: RelationKind::Projects,
            basis: vec![observation_a.clone()],
        }],
        authority_claims: vec![AuthorityClaim {
            contract: contract_id,
            representation: representation_a,
            facet: ContractFacet::Structural,
            basis: AuthorityBasis::MechanicalEnforcement,
            evidence: vec![observation_a],
        }],
    }
}

#[test]
fn encodes_valid_graph_in_canonical_order() {
    let graph = fixture_graph_with_reversed_vectors();
    let json = encode_graph_json(&graph).expect("valid graph");
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert_eq!(value["schema"], "chirograph-graph-v1");
    assert_eq!(value["contracts"][0]["id"], "example.contract");
    assert_eq!(value["representations"][0]["id"], "example.contract.a");
    assert_eq!(value["representations"][1]["id"], "example.contract.b");
    assert_eq!(value["relations"][0]["kind"], "projects");
    assert_eq!(
        value["authority_claims"][0]["basis"],
        "mechanical-enforcement"
    );
    assert_eq!(value["clause_assessments"][0]["status"], "contested");
    assert_eq!(value["lifecycle"], serde_json::json!([]));
}
