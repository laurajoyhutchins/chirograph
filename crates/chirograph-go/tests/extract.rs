use chirograph_core::model::{Revision, SourceId};
use chirograph_go::{GoFactKind, extract_go_facts};
use chirograph_tree_sitter::SourceProvenance;

const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

fn provenance() -> SourceProvenance {
    SourceProvenance {
        source: SourceId::new("github:example/go-fixture").expect("source id should be valid"),
        revision: Revision::Exact(REVISION.to_owned()),
        locator: "fixture.go".to_owned(),
        path: "fixture.go".to_owned(),
    }
}

const SOURCE: &str = r#"package widget

// Widget carries a stable identifier.
const Kind = "widget"
var DefaultID = "default"

type Validator interface {
    Valid() bool
}

type Widget struct {
    ID string `json:"id"`
}

func NewWidget(id string) Widget {
    return Widget{ID: id}
}

func (w Widget) Valid() bool {
    if w.ID == "" {
        panic("missing id")
    }
    for i := 0; i < 1; i++ {
    }
    switch w.ID {
    case "x":
        return true
    }
    return false
}

func TestWidget(t *testing.T) {
    candidate := Widget{ID: "x"}
    if !candidate.Valid() {
        t.Fatalf("expected valid widget")
    }
}
"#;

#[test]
fn extracts_generic_go_source_facts_with_exact_provenance() {
    let extraction = extract_go_facts(SOURCE.as_bytes(), provenance())
        .expect("generic Go source should parse");

    assert!(extraction.diagnostics.is_empty());
    for kind in [
        GoFactKind::Package,
        GoFactKind::Type,
        GoFactKind::Struct,
        GoFactKind::Interface,
        GoFactKind::Field,
        GoFactKind::Tag,
        GoFactKind::Const,
        GoFactKind::Var,
        GoFactKind::Function,
        GoFactKind::Method,
        GoFactKind::Receiver,
        GoFactKind::Parameter,
        GoFactKind::TypeExpression,
        GoFactKind::Call,
        GoFactKind::If,
        GoFactKind::Switch,
        GoFactKind::For,
        GoFactKind::Return,
        GoFactKind::Panic,
        GoFactKind::Comment,
        GoFactKind::Assertion,
    ] {
        assert!(
            extraction.facts.iter().any(|fact| fact.kind == kind),
            "missing expected Go fact kind: {kind:?}"
        );
    }

    assert!(extraction.facts.iter().all(|fact| {
        fact.provenance.source.as_str() == "github:example/go-fixture"
            && fact.provenance.revision == Revision::Exact(REVISION.to_owned())
            && fact.provenance.path == "fixture.go"
            && fact.span.start_byte < fact.span.end_byte
            && fact.span.end_byte <= SOURCE.len()
    }));
}

#[test]
fn extraction_is_deterministic_for_identical_source_and_provenance() {
    let first = extract_go_facts(SOURCE.as_bytes(), provenance()).expect("first parse should work");
    let second =
        extract_go_facts(SOURCE.as_bytes(), provenance()).expect("second parse should work");

    assert_eq!(first, second);
}

#[test]
fn malformed_go_source_reports_parse_diagnostics() {
    let extraction = extract_go_facts(b"package broken\nfunc Broken(\n", provenance())
        .expect("Tree-sitter should return a diagnostic-bearing parse");

    assert!(
        !extraction.diagnostics.is_empty(),
        "malformed Go must remain explicit rather than looking like valid partial evidence"
    );
}
