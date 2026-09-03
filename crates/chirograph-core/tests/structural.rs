use chirograph_core::evidence::parse_evidence_json;
use chirograph_core::model::ContractFacet;

#[test]
fn structural_contract_authority_is_a_first_class_facet() {
    let source = r#"
{
  "schema": "chirograph-evidence-v1",
  "sources": [{"id":"catalog","kind":"repository","locator":"contract catalog"}],
  "contracts": [{"id":"example.schema","name":"Example schema","facets":["structural"]}],
  "representations": [{
    "id":"example.schema.authority",
    "contract":"example.schema",
    "source":"catalog",
    "kind":"schema",
    "locator":"schema/example.json",
    "facets":["structural"]
  }],
  "observations": [{
    "id":"example.schema.classification",
    "source":"catalog",
    "revision":{"kind":"unknown"},
    "locator":"classification",
    "fact":"example.schema.authority is explicitly classified as the structural authority"
  }],
  "clauses": [],
  "clause_assertions": [],
  "relations": [],
  "authority_claims": [{
    "contract":"example.schema",
    "representation":"example.schema.authority",
    "facet":"structural",
    "basis":"explicit_declaration",
    "evidence":["example.schema.classification"]
  }]
}
"#;

    let graph = parse_evidence_json(source).expect("structural evidence should parse");
    assert_eq!(graph.contracts[0].facets, vec![ContractFacet::Structural]);
}
