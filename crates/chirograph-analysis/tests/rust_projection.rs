use std::collections::BTreeSet;

use chirograph_analysis::{AnalysisSourceContext, CandidateMechanism, extract_rust_candidates};
use chirograph_core::model::Revision;

fn context() -> AnalysisSourceContext {
    AnalysisSourceContext::github(
        "acme/fixture-project",
        Revision::Exact("0123456789abcdef0123456789abcdef01234567".into()),
    )
    .unwrap()
}

#[test]
fn projects_unique_serde_field_chain_and_closed_enum() {
    let source = br#"
#[serde(rename_all = "kebab-case")]
struct Manifest {
    profile: Profile,
}

#[serde(rename_all = "kebab-case")]
struct Profile {
    debug_info: DebugInfo,
}

#[serde(rename_all = "kebab-case")]
enum DebugInfo {
    None,
    LineTablesOnly,
    Full,
}
"#;

    let candidates = extract_rust_candidates(&context(), "src/lib.rs", source).unwrap();
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.semantic_path.dotted() == "profile.debug-info")
        .expect("serialized nested path should be mechanically derivable");

    assert_eq!(
        candidate.closed_values,
        Some(BTreeSet::from([
            "full".to_owned(),
            "line-tables-only".to_owned(),
            "none".to_owned(),
        ]))
    );
    assert!(
        candidate
            .mechanisms
            .contains(&CandidateMechanism::RustSerializedField)
    );
    assert!(
        candidate
            .mechanisms
            .contains(&CandidateMechanism::RustTypeReference)
    );
    assert!(
        candidate
            .mechanisms
            .contains(&CandidateMechanism::RustClosedValueSet)
    );
    assert!(matches!(candidate.evidence[0].revision, Revision::Exact(_)));
}

#[test]
fn traverses_unique_transparent_newtype_without_inventing_a_path_segment() {
    let source = br#"
#[serde(rename_all = "kebab-case")]
struct Root {
    profiles: Profiles,
}

struct Profiles(BTreeMap<ProfileName, Profile>);

#[serde(rename_all = "kebab-case")]
struct Profile {
    debug: DebugInfo,
}

#[serde(rename_all = "kebab-case")]
enum DebugInfo {
    None,
    Full,
}
"#;

    let candidates = extract_rust_candidates(&context(), "src/lib.rs", source).unwrap();
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.semantic_path.dotted() == "profiles.debug")
        .expect("unique newtype wrapper should preserve the consumer path while traversing its inner local type");

    assert_eq!(
        candidate.closed_values,
        Some(BTreeSet::from(["full".to_owned(), "none".to_owned()]))
    );
    assert!(candidate.evidence.iter().any(|evidence| {
        evidence.fact.contains("transparent wrapper") && evidence.fact.contains("Profile")
    }));
}

#[test]
fn ambiguous_same_named_type_edges_do_not_resolve() {
    let source = br#"
mod left {
    #[serde(rename_all = "kebab-case")]
    struct Leaf { value: Value }
    #[serde(rename_all = "kebab-case")]
    enum Value { One, Two }
}
mod right {
    #[serde(rename_all = "kebab-case")]
    struct Leaf { value: Value }
    #[serde(rename_all = "kebab-case")]
    enum Value { One, Two }
}
#[serde(rename_all = "kebab-case")]
struct Root { leaf: Leaf }
"#;

    assert!(
        extract_rust_candidates(&context(), "src/lib.rs", source)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn field_without_explicit_or_derivable_serialization_rule_is_not_a_path() {
    let source = br#"
struct Root { leaf: Leaf }
#[serde(rename_all = "kebab-case")]
enum Leaf { One, Two }
"#;

    assert!(
        extract_rust_candidates(&context(), "src/lib.rs", source)
            .unwrap()
            .is_empty()
    );
}
