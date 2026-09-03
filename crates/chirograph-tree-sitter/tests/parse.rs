use chirograph_core::model::{Revision, SourceId};
use chirograph_tree_sitter::{ParseError, SourceProvenance, parse_utf8};

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

#[test]
fn malformed_regions_are_reported_without_inventing_clean_parse_state() {
    let language = tree_sitter_rust::LANGUAGE.into();
    let parsed = parse_utf8(&language, b"fn broken( {", provenance(Revision::Unknown)).unwrap();

    assert!(!parsed.diagnostics().is_empty());
}

#[test]
fn unknown_revision_stays_unknown() {
    let language = tree_sitter_rust::LANGUAGE.into();
    let parsed = parse_utf8(
        &language,
        b"const X: u8 = 1;",
        provenance(Revision::Unknown),
    )
    .unwrap();

    assert_eq!(parsed.provenance().revision, Revision::Unknown);
}

#[test]
fn preorder_is_deterministic() {
    let language = tree_sitter_rust::LANGUAGE.into();
    let parsed = parse_utf8(
        &language,
        b"fn b() {}\nfn a() {}\n",
        provenance(Revision::Unknown),
    )
    .unwrap();

    let left: Vec<_> = parsed
        .preorder()
        .into_iter()
        .map(|node| (node.kind().to_owned(), node.start_byte(), node.end_byte()))
        .collect();
    let right: Vec<_> = parsed
        .preorder()
        .into_iter()
        .map(|node| (node.kind().to_owned(), node.start_byte(), node.end_byte()))
        .collect();

    assert_eq!(left, right);
}

#[test]
fn invalid_utf8_is_rejected_before_parsing() {
    let language = tree_sitter_rust::LANGUAGE.into();
    let result = parse_utf8(&language, &[0xff], provenance(Revision::Unknown));

    assert!(matches!(result, Err(ParseError::InvalidUtf8(0))));
}
