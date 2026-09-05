use chirograph_core::model::{Revision, SourceId};
use chirograph_rust::{RustFactKind, extract_rust_facts};
use chirograph_tree_sitter::SourceProvenance;

const SOURCE: &str = r#"
#[derive(Debug)]
pub enum Mode { None, Full }

impl Mode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Full => "full",
        }
    }
}

#[test]
fn labels() { assert_eq!(Mode::Full.label(), "full"); }
"#;

fn exact_fixture_provenance() -> SourceProvenance {
    SourceProvenance {
        source: SourceId::new("fixture.repo").unwrap(),
        revision: Revision::Exact("0123456789abcdef0123456789abcdef01234567".into()),
        locator: "github:fixture/repo".into(),
        path: "src/lib.rs".into(),
    }
}

#[test]
fn extracts_declarations_variants_match_arms_and_assertions() {
    let p = exact_fixture_provenance();
    let extraction = extract_rust_facts(SOURCE.as_bytes(), p.clone()).unwrap();

    assert!(
        extraction
            .facts
            .iter()
            .any(|fact| fact.kind == RustFactKind::Enum && fact.name.as_deref() == Some("Mode"))
    );
    assert!(
        extraction
            .facts
            .iter()
            .any(|fact| fact.kind == RustFactKind::Variant && fact.name.as_deref() == Some("None"))
    );
    assert!(
        extraction
            .facts
            .iter()
            .any(|fact| fact.kind == RustFactKind::MatchArm
                && fact.text.contains("Self::None => \"none\""))
    );
    assert!(
        extraction
            .facts
            .iter()
            .any(|fact| fact.kind == RustFactKind::Assertion && fact.text.starts_with("assert_eq!"))
    );
    assert!(extraction.facts.iter().all(|fact| fact.provenance == p));
}
