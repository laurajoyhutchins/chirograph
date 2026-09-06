use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chirograph_acquisition::{
    AcquiredFact, AcquisitionContext, AcquisitionError, AcquisitionRuntime, AdapterCapability,
    AdapterError, AdapterFamily, AdapterInput, DiagnosticKind, SourceAdapter,
};
use chirograph_core::model::{Revision, SourceId};

fn temp_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "chirograph-acquisition-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("temporary acquisition root should be created");
    root
}

fn context() -> AcquisitionContext {
    AcquisitionContext::new(
        SourceId::new("github:example/polyglot").expect("source identity should be valid"),
        Revision::Exact("0123456789abcdef0123456789abcdef01234567".to_owned()),
    )
}

fn write_fixture(root: &Path, rust_first: bool) {
    fs::create_dir_all(root.join("src")).expect("source directory should be created");
    if rust_first {
        fs::write(
            root.join("src/lib.rs"),
            "pub struct Widget { pub value: u32 }\n",
        )
        .expect("Rust fixture should be written");
        fs::write(
            root.join("schema.json"),
            "// source schema\n{\"kind\":\"widget\",\"values\":[1,2]}\n",
        )
        .expect("JSON fixture should be written");
    } else {
        fs::write(
            root.join("schema.json"),
            "// source schema\n{\"kind\":\"widget\",\"values\":[1,2]}\n",
        )
        .expect("JSON fixture should be written");
        fs::write(
            root.join("src/lib.rs"),
            "pub struct Widget { pub value: u32 }\n",
        )
        .expect("Rust fixture should be written");
    }
    fs::write(root.join("README.txt"), "not a supported source\n")
        .expect("unsupported fixture should be written");
}

#[derive(Debug)]
struct SyntheticAdapter {
    id: &'static str,
    extension: &'static str,
}

impl SourceAdapter for SyntheticAdapter {
    fn capability(&self) -> AdapterCapability {
        AdapterCapability {
            adapter: self.id.to_owned(),
            family: AdapterFamily::StructuredSemantic,
            extensions: vec![self.extension.to_owned()],
        }
    }

    fn acquire(&self, input: AdapterInput<'_>) -> Result<Vec<AcquiredFact>, AdapterError> {
        Ok(vec![AcquiredFact {
            adapter: self.id.to_owned(),
            kind: "synthetic".to_owned(),
            path: input.path.to_owned(),
            locator: format!("{}#synthetic", input.path),
            text: String::from_utf8_lossy(input.bytes).into_owned(),
            source: input.context.source.clone(),
            revision: input.context.revision.clone(),
            span: None,
        }])
    }
}

#[test]
fn public_registry_accepts_a_thin_custom_adapter() {
    let root = temp_root("custom-adapter");
    fs::write(root.join("contract.demo"), "contract-v1\n")
        .expect("custom fixture should be written");
    let runtime = AcquisitionRuntime::with_adapters(vec![Box::new(SyntheticAdapter {
        id: "synthetic",
        extension: "demo",
    })])
    .expect("custom adapter registry should be valid");

    let report = runtime
        .acquire_tree(&root, &context())
        .expect("custom adapter acquisition should succeed");
    fs::remove_dir_all(&root).expect("temporary acquisition root should be removed");

    assert_eq!(report.capabilities.len(), 1);
    assert_eq!(report.capabilities[0].adapter, "synthetic");
    assert_eq!(report.facts.len(), 1);
    assert_eq!(report.facts[0].path, "contract.demo");
}

#[test]
fn ambiguous_adapter_selection_fails_closed() {
    let root = temp_root("ambiguous-adapter");
    fs::write(root.join("contract.demo"), "contract-v1\n")
        .expect("ambiguous fixture should be written");
    let runtime = AcquisitionRuntime::with_adapters(vec![
        Box::new(SyntheticAdapter {
            id: "first",
            extension: "demo",
        }),
        Box::new(SyntheticAdapter {
            id: "second",
            extension: "demo",
        }),
    ])
    .expect("overlapping extensions are resolved at source selection time");

    let error = runtime
        .acquire_tree(&root, &context())
        .expect_err("ambiguous adapter selection must fail closed");
    fs::remove_dir_all(&root).expect("temporary acquisition root should be removed");

    match error {
        AcquisitionError::AmbiguousAdapter { path, adapters } => {
            assert_eq!(path, "contract.demo");
            assert_eq!(adapters, ["first", "second"]);
        }
        other => panic!("expected ambiguous adapter error, got {other}"),
    }
}

#[test]
fn duplicate_adapter_identity_is_rejected_at_registration() {
    let error = AcquisitionRuntime::with_adapters(vec![
        Box::new(SyntheticAdapter {
            id: "duplicate",
            extension: "one",
        }),
        Box::new(SyntheticAdapter {
            id: "duplicate",
            extension: "two",
        }),
    ])
    .expect_err("duplicate adapter identity must be rejected");

    assert!(matches!(
        error,
        AcquisitionError::DuplicateAdapterId { ref adapter } if adapter == "duplicate"
    ));
}

#[test]
fn default_runtime_reports_tree_sitter_and_structured_capabilities() {
    let capabilities = AcquisitionRuntime::default().capabilities();

    assert!(capabilities.iter().any(|capability| {
        capability.adapter == "rust"
            && capability.family == AdapterFamily::TreeSitter
            && capability.extensions == ["rs"]
    }));
    assert!(capabilities.iter().any(|capability| {
        capability.adapter == "json"
            && capability.family == AdapterFamily::StructuredSemantic
            && capability.extensions == ["json"]
    }));
}

#[test]
fn mixed_acquisition_preserves_exact_provenance_and_spans() {
    let root = temp_root("mixed");
    write_fixture(&root, true);
    let context = context();

    let report = AcquisitionRuntime::default()
        .acquire_tree(&root, &context)
        .expect("mixed source acquisition should succeed");
    fs::remove_dir_all(&root).expect("temporary acquisition root should be removed");

    let rust_facts = report
        .facts
        .iter()
        .filter(|fact| fact.adapter == "rust")
        .collect::<Vec<_>>();
    assert!(!rust_facts.is_empty(), "Rust source must be dispatched");
    assert!(rust_facts.iter().all(|fact| {
        fact.source == context.source
            && fact.revision == context.revision
            && fact.path == "src/lib.rs"
            && fact.span.is_some()
    }));

    assert!(report.facts.iter().any(|fact| {
        fact.adapter == "json" && fact.path == "schema.json" && fact.locator == "#/kind"
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == DiagnosticKind::UnsupportedSource && diagnostic.path == "README.txt"
    }));
}

#[test]
fn acquisition_is_independent_of_root_path_and_creation_order() {
    let first = temp_root("order-a");
    let second = temp_root("order-b");
    write_fixture(&first, true);
    write_fixture(&second, false);
    let context = context();

    let first_report = AcquisitionRuntime::default()
        .acquire_tree(&first, &context)
        .expect("first acquisition should succeed");
    let second_report = AcquisitionRuntime::default()
        .acquire_tree(&second, &context)
        .expect("second acquisition should succeed");
    fs::remove_dir_all(&first).expect("first temporary root should be removed");
    fs::remove_dir_all(&second).expect("second temporary root should be removed");

    assert_eq!(first_report, second_report);
}

#[cfg(unix)]
#[test]
fn discovery_does_not_follow_symlinks_outside_the_root() {
    use std::os::unix::fs::symlink;

    let root = temp_root("symlink-root");
    let outside = temp_root("symlink-outside");
    fs::write(outside.join("escaped.json"), "{\"escaped\":true}\n")
        .expect("outside fixture should be written");
    symlink(outside.join("escaped.json"), root.join("escaped.json"))
        .expect("fixture symlink should be created");

    let report = AcquisitionRuntime::default()
        .acquire_tree(&root, &context())
        .expect("symlink-safe acquisition should succeed");

    fs::remove_dir_all(&root).expect("temporary acquisition root should be removed");
    fs::remove_dir_all(&outside).expect("temporary outside root should be removed");

    assert!(
        report.facts.iter().all(|fact| fact.path != "escaped.json"),
        "acquisition must not read a symlink target outside the supplied root"
    );
}

#[test]
fn default_runtime_acquires_go_source_with_exact_spans() {
    let root = temp_root("go-runtime");
    fs::write(
        root.join("widget.go"),
        r#"package widget

// Widget carries a stable identifier.
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
    return true
}

func TestWidget(t *testing.T) {
    candidate := Widget{ID: "x"}
    if !candidate.Valid() {
        t.Fatalf("expected valid widget")
    }
}
"#,
    )
    .expect("Go fixture should be written");
    let context = context();

    let runtime = AcquisitionRuntime::default();
    let capabilities = runtime.capabilities();
    assert!(capabilities.iter().any(|capability| {
        capability.adapter == "go"
            && capability.family == AdapterFamily::TreeSitter
            && capability.extensions == ["go"]
    }));

    let report = runtime
        .acquire_tree(&root, &context)
        .expect("Go acquisition should succeed");
    fs::remove_dir_all(&root).expect("temporary acquisition root should be removed");

    let go_facts = report
        .facts
        .iter()
        .filter(|fact| fact.adapter == "go")
        .collect::<Vec<_>>();
    assert!(!go_facts.is_empty(), "Go source must be dispatched");
    assert!(go_facts.iter().all(|fact| {
        fact.source == context.source
            && fact.revision == context.revision
            && fact.path == "widget.go"
            && fact.span.is_some()
    }));
    for kind in [
        "package",
        "type",
        "struct",
        "field",
        "tag",
        "method",
        "receiver",
        "function",
        "call",
        "if",
        "panic",
        "return",
        "comment",
        "assertion",
    ] {
        assert!(
            go_facts.iter().any(|fact| fact.kind == kind),
            "missing expected Go fact kind: {kind}"
        );
    }
}

#[test]
fn default_runtime_acquires_java_source_with_exact_spans() {
    let root = temp_root("java-runtime");
    fs::write(
        root.join("Widget.java"),
        r#"package example.contract;

/** Widget carries a stable identifier. */
@Deprecated
public final class Widget {
    public static final String KIND = "widget";
    private final String value;

    public Widget(String value) {
        this.value = value;
    }

    public String value() {
        if (value == null) {
            throw new IllegalStateException("missing value");
        }
        return value;
    }

    public void verify() {
        assert value != null;
        Assertions.assertNotNull(value);
    }
}
"#,
    )
    .expect("Java fixture should be written");
    let context = context();

    let runtime = AcquisitionRuntime::default();
    let capabilities = runtime.capabilities();
    assert!(capabilities.iter().any(|capability| {
        capability.adapter == "java"
            && capability.family == AdapterFamily::TreeSitter
            && capability.extensions == ["java"]
    }));

    let report = runtime
        .acquire_tree(&root, &context)
        .expect("Java acquisition should succeed");
    fs::remove_dir_all(&root).expect("temporary acquisition root should be removed");

    let java_facts = report
        .facts
        .iter()
        .filter(|fact| fact.adapter == "java")
        .collect::<Vec<_>>();
    assert!(!java_facts.is_empty(), "Java source must be dispatched");
    assert!(java_facts.iter().all(|fact| {
        fact.source == context.source
            && fact.revision == context.revision
            && fact.path == "Widget.java"
            && fact.span.is_some()
    }));
    for kind in [
        "package",
        "class",
        "field",
        "constant",
        "literal",
        "signature",
        "method",
        "constructor",
        "parameter",
        "annotation",
        "call",
        "if",
        "return",
        "throw",
        "comment",
        "assertion",
    ] {
        assert!(
            java_facts.iter().any(|fact| fact.kind == kind),
            "missing expected Java runtime fact kind: {kind}"
        );
    }
}

#[test]
fn default_runtime_rejects_malformed_java_source() {
    let root = temp_root("java-malformed");
    fs::write(root.join("Broken.java"), "class Broken { void f( {")
        .expect("malformed Java fixture should be written");

    let error = AcquisitionRuntime::default()
        .acquire_tree(&root, &context())
        .expect_err("malformed Java must fail closed");
    fs::remove_dir_all(&root).expect("temporary acquisition root should be removed");

    assert!(matches!(
        error,
        AcquisitionError::MalformedSyntax {
            ref adapter,
            ref path,
            diagnostic_count,
        } if adapter == "java" && path == "Broken.java" && diagnostic_count > 0
    ));
}
