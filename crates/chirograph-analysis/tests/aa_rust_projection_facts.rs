use chirograph_core::model::{Revision, SourceId};
use chirograph_rust::{RustFactKind, extract_rust_facts};
use chirograph_tree_sitter::SourceProvenance;

const SOURCE: &str = r#"
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

#[test]
fn source_facts_keep_leading_attributes_in_the_parent_container() {
    let extraction = extract_rust_facts(
        SOURCE.as_bytes(),
        SourceProvenance {
            source: SourceId::new("github:acme/fixture-project").unwrap(),
            revision: Revision::Exact("0123456789abcdef0123456789abcdef01234567".into()),
            locator: "src/lib.rs".into(),
            path: "src/lib.rs".into(),
        },
    )
    .unwrap();
    let facts = &extraction.facts;

    for (kind, name, container) in [
        (RustFactKind::Struct, Some("Manifest"), Vec::<&str>::new()),
        (RustFactKind::Field, Some("profile"), vec!["Manifest"]),
        (RustFactKind::Struct, Some("Profile"), Vec::<&str>::new()),
        (RustFactKind::Field, Some("debug_info"), vec!["Profile"]),
        (RustFactKind::Enum, Some("DebugInfo"), Vec::<&str>::new()),
        (
            RustFactKind::Variant,
            Some("LineTablesOnly"),
            vec!["DebugInfo"],
        ),
    ] {
        assert!(
            facts.iter().any(|fact| {
                fact.kind == kind && fact.name.as_deref() == name && fact.container == container
            }),
            "missing {kind:?} {name:?} in {container:?}; facts={facts:#?}"
        );
    }

    let attributes = facts
        .iter()
        .filter(|fact| fact.kind == RustFactKind::Attribute && fact.text.contains("rename_all"))
        .collect::<Vec<_>>();
    assert_eq!(attributes.len(), 3);
    assert!(
        attributes
            .iter()
            .all(|attribute| attribute.container.is_empty())
    );

    for declaration in facts
        .iter()
        .filter(|fact| matches!(fact.kind, RustFactKind::Struct | RustFactKind::Enum))
    {
        assert!(attributes.iter().any(|attribute| {
            attribute.span.end_byte <= declaration.span.start_byte
                && SOURCE.as_bytes()[attribute.span.end_byte..declaration.span.start_byte]
                    .iter()
                    .all(|byte| byte.is_ascii_whitespace())
        }));
    }

    for (container, text) in [
        (vec!["Manifest"], "Profile"),
        (vec!["Profile"], "DebugInfo"),
    ] {
        assert!(
            facts.iter().any(|fact| {
                fact.kind == RustFactKind::TypeExpression
                    && fact.container == container
                    && fact.text == text
            }),
            "missing type edge {container:?} -> {text}; facts={facts:#?}"
        );
    }
}
