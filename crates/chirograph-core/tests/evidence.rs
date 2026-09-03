use chirograph_core::evidence::{
    EVIDENCE_SCHEMA_V1, EvidenceError, parse_evidence_json, render_evidence_json_pretty,
};
use chirograph_core::model::{ContractFacet, ModelError, SourceId};

const VALID_DOCUMENT: &str = r#"
{
  "schema": "chirograph-evidence-v1",
  "sources": [
    {
      "id": "repo",
      "kind": "repository",
      "locator": "https://example.invalid/repo"
    }
  ],
  "contracts": [
    {
      "id": "example.contract",
      "name": "Example contract",
      "facets": ["semantic"]
    }
  ],
  "representations": [
    {
      "id": "example.source",
      "contract": "example.contract",
      "source": "repo",
      "kind": "source_code",
      "locator": "src/example.rs#Example",
      "facets": ["semantic"]
    }
  ],
  "observations": [
    {
      "id": "example.observation",
      "source": "repo",
      "revision": { "kind": "exact", "value": "abc123" },
      "locator": "src/example.rs#Example",
      "fact": "the example representation exists"
    }
  ],
  "clauses": [
    {
      "id": "example.requirement",
      "contract": "example.contract",
      "facet": "semantic",
      "kind": "requirement",
      "statement": "the example value must be present"
    }
  ],
  "clause_assertions": [
    {
      "clause": "example.requirement",
      "representation": "example.source",
      "stance": "supports",
      "evidence": ["example.observation"]
    }
  ],
  "relations": [],
  "authority_claims": []
}
"#;

#[test]
fn parses_v1_evidence_into_the_contract_graph() {
    let graph = parse_evidence_json(VALID_DOCUMENT).expect("v1 evidence should parse");

    assert_eq!(graph.contracts.len(), 1);
    assert_eq!(graph.contracts[0].id.as_str(), "example.contract");
    assert_eq!(graph.contracts[0].facets, vec![ContractFacet::Semantic]);
    assert_eq!(graph.clauses.len(), 1);
}

#[test]
fn rejects_an_unsupported_evidence_schema() {
    let source = VALID_DOCUMENT.replace(EVIDENCE_SCHEMA_V1, "chirograph-evidence-v999");

    assert_eq!(
        parse_evidence_json(&source),
        Err(EvidenceError::UnsupportedSchema(
            "chirograph-evidence-v999".into()
        ))
    );
}

#[test]
fn rejects_a_document_whose_graph_is_invalid() {
    let source = VALID_DOCUMENT.replace("\"source\": \"repo\"", "\"source\": \"missing\"");

    assert_eq!(
        parse_evidence_json(&source),
        Err(EvidenceError::InvalidGraph(ModelError::UnknownSource(
            SourceId::new("missing").expect("valid source id")
        )))
    );
}

#[test]
fn renders_a_v1_document_that_round_trips() {
    let graph = parse_evidence_json(VALID_DOCUMENT).expect("v1 evidence should parse");
    let rendered = render_evidence_json_pretty(&graph).expect("graph should render");
    let reparsed = parse_evidence_json(&rendered).expect("rendered evidence should parse");

    assert_eq!(reparsed, graph);
    assert!(rendered.contains("\"schema\": \"chirograph-evidence-v1\""));
}
