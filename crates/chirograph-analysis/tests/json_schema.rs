use std::collections::BTreeSet;

use chirograph_analysis::{AnalysisSourceContext, CandidateMechanism, extract_json_schema_candidates};
use chirograph_core::model::{RepresentationKind, Revision};

fn context() -> AnalysisSourceContext {
    AnalysisSourceContext::github(
        "acme/fixture-project",
        Revision::Exact("0123456789abcdef0123456789abcdef01234567".into()),
    )
    .unwrap()
}

#[test]
fn extracts_explicit_property_path_and_closed_value_set() {
    let schema = br#"{
        "type": "object",
        "properties": {
            "profile": {
                "type": "object",
                "properties": {
                    "debug-info": {
                        "type": "string",
                        "enum": ["none", "line-tables-only", "full"]
                    }
                }
            }
        }
    }"#;

    let candidates = extract_json_schema_candidates(&context(), "schema.json", schema).unwrap();
    assert_eq!(candidates.len(), 1);
    let candidate = &candidates[0];
    assert_eq!(candidate.kind, RepresentationKind::Schema);
    assert_eq!(candidate.semantic_path.dotted(), "profile.debug-info");
    assert_eq!(
        candidate.closed_values,
        Some(BTreeSet::from([
            "full".to_owned(),
            "line-tables-only".to_owned(),
            "none".to_owned(),
        ]))
    );
    assert!(candidate.mechanisms.contains(&CandidateMechanism::JsonSchemaProperty));
    assert!(
        candidate
            .mechanisms
            .contains(&CandidateMechanism::JsonSchemaClosedValueSet)
    );
    assert!(matches!(candidate.evidence[0].revision, Revision::Exact(_)));
}

#[test]
fn omits_open_properties_from_first_slice() {
    let schema = br#"{
        "type": "object",
        "properties": {
            "profile": {
                "type": "object",
                "properties": { "name": { "type": "string" } }
            }
        }
    }"#;

    assert!(
        extract_json_schema_candidates(&context(), "schema.json", schema)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn broken_local_reference_fails_closed() {
    let schema = br#"{
        "type": "object",
        "properties": { "profile": { "$ref": "#/$defs/Missing" } },
        "$defs": {}
    }"#;

    assert!(extract_json_schema_candidates(&context(), "schema.json", schema).is_err());
}
