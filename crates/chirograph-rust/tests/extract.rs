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

const BOUNDARY_SOURCE: &str = r#"
/// module docs
mod outer {
    #[derive(Debug)]
    struct Widget {
        value: u16,
    }

    trait Describe {
        fn describe(&self) -> &'static str;
    }

    impl Widget {
        fn method(&self) -> u16 {
            if self.value > 0 {
                return self.value;
            }
            helper()
        }
    }

    const LIMIT: u16 = 7;
    static ENABLED: bool = true;

    fn helper() -> u16 {
        some_macro!();
        assert_eq!(1, 1);
        match 1 {
            1 => 1,
            _ => 0,
        }
    }
}
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

#[test]
fn covers_the_documented_v0_syntax_boundary() {
    let extraction =
        extract_rust_facts(BOUNDARY_SOURCE.as_bytes(), exact_fixture_provenance()).unwrap();
    let facts = &extraction.facts;

    for kind in [
        RustFactKind::Module,
        RustFactKind::Struct,
        RustFactKind::Trait,
        RustFactKind::Impl,
        RustFactKind::Function,
        RustFactKind::Method,
        RustFactKind::Field,
        RustFactKind::Const,
        RustFactKind::Static,
        RustFactKind::TypeExpression,
        RustFactKind::Attribute,
        RustFactKind::Call,
        RustFactKind::MacroCall,
        RustFactKind::If,
        RustFactKind::Match,
        RustFactKind::MatchArm,
        RustFactKind::Return,
        RustFactKind::Comment,
        RustFactKind::Assertion,
    ] {
        assert!(
            facts.iter().any(|fact| fact.kind == kind),
            "missing {kind:?}"
        );
    }

    let method = facts
        .iter()
        .find(|fact| fact.kind == RustFactKind::Method && fact.name.as_deref() == Some("method"))
        .unwrap();
    assert_eq!(method.container, vec!["outer", "Widget"]);

    let field = facts
        .iter()
        .find(|fact| fact.kind == RustFactKind::Field && fact.name.as_deref() == Some("value"))
        .unwrap();
    assert_eq!(field.container, vec!["outer", "Widget"]);

    assert!(facts.iter().any(|fact| {
        fact.kind == RustFactKind::MacroCall && fact.name.as_deref() == Some("some_macro")
    }));
}

#[test]
fn malformed_source_is_diagnostic_and_not_semantic_certainty() {
    let extraction = extract_rust_facts(b"fn broken( {", exact_fixture_provenance()).unwrap();
    assert!(!extraction.diagnostics.is_empty());
    assert!(
        !extraction
            .facts
            .iter()
            .any(|fact| fact.kind == RustFactKind::Function
                && fact.name.as_deref() == Some("broken"))
    );
}

#[test]
fn repeated_extraction_is_deterministic() {
    let provenance = exact_fixture_provenance();
    let left = extract_rust_facts(BOUNDARY_SOURCE.as_bytes(), provenance.clone()).unwrap();
    let right = extract_rust_facts(BOUNDARY_SOURCE.as_bytes(), provenance).unwrap();
    assert_eq!(left, right);
}

#[test]
fn multibyte_source_preserves_byte_offsets_and_tree_sitter_points() {
    let source = "// é\nconst X: u8 = 1;\n";
    let extraction = extract_rust_facts(source.as_bytes(), exact_fixture_provenance()).unwrap();
    let constant = extraction
        .facts
        .iter()
        .find(|fact| fact.kind == RustFactKind::Const && fact.name.as_deref() == Some("X"))
        .unwrap();

    assert_eq!(constant.span.start_byte, 6);
    assert_eq!(constant.span.start.row, 1);
    assert_eq!(constant.span.start.column, 0);
    assert_eq!(
        &source.as_bytes()[constant.span.start_byte..constant.span.end_byte],
        constant.text.as_bytes()
    );
}
