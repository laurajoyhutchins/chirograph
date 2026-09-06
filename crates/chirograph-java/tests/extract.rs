use chirograph_core::model::{Revision, SourceId};
use chirograph_java::{JavaFactKind, UNSUPPORTED_SEMANTICS, extract_java_facts};
use chirograph_tree_sitter::SourceProvenance;

const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

fn provenance() -> SourceProvenance {
    SourceProvenance {
        source: SourceId::new("github:example/java-fixture").expect("source id should be valid"),
        revision: Revision::Exact(REVISION.to_owned()),
        locator: "Widget.java".to_owned(),
        path: "Widget.java".to_owned(),
    }
}

const SOURCE: &str = r#"package example.contract;

import java.util.List;

/** Widget carries a stable contract value. */
@Schema(version = 1)
public class Widget<T extends Comparable<T>> implements Validatable {
    public static final String KIND = "widget";
    private final T value;

    public Widget(T value) {
        this.value = value;
    }

    @Override
    public T value() throws IllegalStateException {
        if (value == null) {
            throw new IllegalStateException("missing value");
        }
        switch (KIND) {
            case "widget":
                return value;
            default:
                return value;
        }
    }

    public void verify() {
        assert value != null;
        Assertions.assertNotNull(value);
    }
}

interface Validatable {
    Object value();
}

enum Mode {
    FAST,
    SAFE
}

record Entry(String id) {}

@interface Schema {
    int version();
}
"#;

#[test]
fn extracts_generic_java_source_facts_with_exact_provenance() {
    let extraction = extract_java_facts(SOURCE.as_bytes(), provenance())
        .expect("generic Java source should parse");

    assert!(extraction.diagnostics.is_empty());
    for kind in [
        JavaFactKind::Package,
        JavaFactKind::Import,
        JavaFactKind::Class,
        JavaFactKind::Interface,
        JavaFactKind::Enum,
        JavaFactKind::EnumConstant,
        JavaFactKind::Record,
        JavaFactKind::AnnotationDeclaration,
        JavaFactKind::Field,
        JavaFactKind::Constant,
        JavaFactKind::Literal,
        JavaFactKind::TypeParameter,
        JavaFactKind::TypeExpression,
        JavaFactKind::Signature,
        JavaFactKind::Method,
        JavaFactKind::Constructor,
        JavaFactKind::Parameter,
        JavaFactKind::Annotation,
        JavaFactKind::AnnotationArgument,
        JavaFactKind::Call,
        JavaFactKind::If,
        JavaFactKind::Switch,
        JavaFactKind::Return,
        JavaFactKind::Throw,
        JavaFactKind::Comment,
        JavaFactKind::Assertion,
    ] {
        assert!(
            extraction.facts.iter().any(|fact| fact.kind == kind),
            "missing expected Java fact kind: {kind:?}"
        );
    }

    assert!(extraction.facts.iter().all(|fact| {
        fact.provenance.source.as_str() == "github:example/java-fixture"
            && fact.provenance.revision == Revision::Exact(REVISION.to_owned())
            && fact.provenance.path == "Widget.java"
            && fact.span.start_byte < fact.span.end_byte
            && fact.span.end_byte <= SOURCE.len()
    }));
}

#[test]
fn extraction_is_deterministic_for_identical_source_and_provenance() {
    let first = extract_java_facts(SOURCE.as_bytes(), provenance()).expect("first parse should work");
    let second = extract_java_facts(SOURCE.as_bytes(), provenance()).expect("second parse should work");
    assert_eq!(first, second);
}

#[test]
fn malformed_java_source_reports_parse_diagnostics() {
    let extraction = extract_java_facts(b"class Broken { void f( {", provenance())
        .expect("Tree-sitter should return a diagnostic-bearing parse");
    assert!(
        !extraction.diagnostics.is_empty(),
        "malformed Java must remain explicit rather than looking like valid partial evidence"
    );
}

#[test]
fn unsupported_java_semantics_remain_explicit() {
    assert_eq!(
        UNSUPPORTED_SEMANTICS,
        [
            "whole-program symbol resolution",
            "classpath or build execution",
            "annotation processor execution",
            "runtime or reflection semantics",
        ]
    );
}