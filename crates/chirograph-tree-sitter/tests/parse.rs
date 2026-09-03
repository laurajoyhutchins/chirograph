use chirograph_core::model::{Revision, SourceId};
use chirograph_tree_sitter::{SourceProvenance, parse_utf8};

fn provenance(revision: Revision) -> SourceProvenance {
    SourceProvenance {
        source: SourceId::new("fixture.repo").unwrap(),
        revision,
        locator: "github:fixture/repo".into(),
        path: "src/lib.rs".into(),
    }
}

#[test]
fn preserves_exact_revision_and_source_coordinates() {
    let language = tree_sitter_rust::LANGUAGE.into();
    let parsed = parse_utf8(
        &language,
        b"fn alpha() {}\n",
        provenance(Revision::Exact(
            "0123456789abcdef0123456789abcdef01234567".into(),
        )),
    )
    .unwrap();

    assert_eq!(
        parsed.provenance().revision,
        Revision::Exact("0123456789abcdef0123456789abcdef01234567".into())
    );
    let function = parsed
        .preorder()
        .into_iter()
        .find(|node| node.kind() == "function_item")
        .unwrap();
    let span = parsed.span(function);
    assert_eq!((span.start_byte, span.end_byte), (0, 13));
    assert_eq!((span.start.row, span.start.column), (0, 0));
    assert_eq!((span.end.row, span.end.column), (0, 13));
    assert_eq!(parsed.text(function), "fn alpha() {}");
}
