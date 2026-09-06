use std::{fs, path::PathBuf};

use chirograph_core::model::{Revision, SourceId};
use chirograph_java::{JavaFactKind, extract_java_facts};
use chirograph_tree_sitter::SourceProvenance;

const KAFKA_REVISION: &str = "b57cf6e56eb59a952db7236b4da67cc2fdbb8cdf";
const UPSTREAM_PATH: &str =
    "clients/src/main/java/org/apache/kafka/common/requests/ProduceRequest.java";

#[test]
fn acquires_pinned_kafka_produce_request_without_repository_specific_logic() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../benchmark/kafka/message-spec-generation/produce-request-data/fixture/clients/src/main/java/org/apache/kafka/common/requests/ProduceRequest.java",
    );
    let source = fs::read(&fixture).expect("pinned Kafka Java fixture should be readable");
    let provenance = SourceProvenance {
        source: SourceId::new("github:apache/kafka")
            .expect("Kafka source identity should be valid"),
        revision: Revision::Exact(KAFKA_REVISION.to_owned()),
        locator: UPSTREAM_PATH.to_owned(),
        path: UPSTREAM_PATH.to_owned(),
    };

    let extraction = extract_java_facts(&source, provenance)
        .expect("generic Java acquisition should parse the pinned Kafka fixture");

    assert!(
        extraction.diagnostics.is_empty(),
        "pinned Kafka fixture must not require permissive partial parsing"
    );
    assert!(
        extraction.facts.iter().any(|fact| {
            fact.kind == JavaFactKind::Class && fact.name.as_deref() == Some("ProduceRequest")
        }),
        "generic acquisition should discover the ProduceRequest class structurally"
    );
    for kind in [
        JavaFactKind::Package,
        JavaFactKind::Import,
        JavaFactKind::Class,
        JavaFactKind::Field,
        JavaFactKind::Constant,
        JavaFactKind::Literal,
        JavaFactKind::Signature,
        JavaFactKind::Method,
        JavaFactKind::Constructor,
        JavaFactKind::Parameter,
        JavaFactKind::Annotation,
        JavaFactKind::Call,
        JavaFactKind::If,
        JavaFactKind::Return,
        JavaFactKind::Throw,
        JavaFactKind::Comment,
    ] {
        assert!(
            extraction.facts.iter().any(|fact| fact.kind == kind),
            "pinned Kafka fixture is missing generic Java fact kind: {kind:?}"
        );
    }

    let source_text =
        std::str::from_utf8(&source).expect("Kafka fixture should be UTF-8 Java source");
    assert!(extraction.facts.iter().all(|fact| {
        fact.provenance.source.as_str() == "github:apache/kafka"
            && fact.provenance.revision == Revision::Exact(KAFKA_REVISION.to_owned())
            && fact.provenance.path == UPSTREAM_PATH
            && fact.span.start_byte < fact.span.end_byte
            && fact.span.end_byte <= source.len()
            && source_text.get(fact.span.start_byte..fact.span.end_byte) == Some(fact.text.as_str())
    }));
}
