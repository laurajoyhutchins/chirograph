use std::{error::Error, fmt};

use chirograph_tree_sitter::{ParseError, ParsedSource, SourceProvenance, parse_utf8};
use tree_sitter::Node;

use crate::fact::{RustExtraction, RustFact, RustFactKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustAdapterError {
    Parse(ParseError),
}

impl fmt::Display for RustAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "cannot parse Rust source: {error}"),
        }
    }
}

impl Error for RustAdapterError {}

impl From<ParseError> for RustAdapterError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

pub fn extract_rust_facts(
    source: &[u8],
    provenance: SourceProvenance,
) -> Result<RustExtraction, RustAdapterError> {
    let language = tree_sitter_rust::LANGUAGE.into();
    let parsed = parse_utf8(&language, source, provenance)?;
    let diagnostics = parsed.diagnostics().to_vec();
    let mut facts = Vec::new();
    let mut container = Vec::new();

    walk(
        &parsed,
        parsed.tree().root_node(),
        &mut container,
        false,
        &mut facts,
    );

    facts.sort_by(|left, right| {
        (
            left.span.start_byte,
            left.span.end_byte,
            left.kind,
            &left.name,
            &left.text,
        )
            .cmp(&(
                right.span.start_byte,
                right.span.end_byte,
                right.kind,
                &right.name,
                &right.text,
            ))
    });
    facts.dedup_by(|left, right| {
        left.span.start_byte == right.span.start_byte
            && left.span.end_byte == right.span.end_byte
            && left.kind == right.kind
            && left.name == right.name
            && left.text == right.text
    });

    Ok(RustExtraction { facts, diagnostics })
}

fn walk(
    parsed: &ParsedSource,
    node: Node<'_>,
    container: &mut Vec<String>,
    in_impl: bool,
    facts: &mut Vec<RustFact>,
) {
    let malformed = node.is_error() || node.is_missing() || node.has_error();
    let kind = direct_kind(node, in_impl);

    if !malformed {
        if let Some(kind) = kind {
            emit_fact(parsed, node, kind, explicit_name(parsed, node), container, facts);
            if kind == RustFactKind::MacroCall && is_assertion_macro(parsed, node) {
                emit_fact(
                    parsed,
                    node,
                    RustFactKind::Assertion,
                    explicit_macro_name(parsed, node),
                    container,
                    facts,
                );
            }
        }

        if let Some(type_node) = node.child_by_field_name("type")
            && !type_node.is_error()
            && !type_node.is_missing()
            && !type_node.has_error()
        {
            emit_fact(
                parsed,
                type_node,
                RustFactKind::TypeExpression,
                None,
                container,
                facts,
            );
        }
    }

    let segment = container_segment(parsed, node, kind, in_impl);
    if let Some(segment) = &segment {
        container.push(segment.clone());
    }

    let child_in_impl = in_impl || node.kind() == "impl_item";
    for index in 0..node.child_count() {
        if let Some(child) = node.child(index as u32) {
            walk(parsed, child, container, child_in_impl, facts);
        }
    }

    if segment.is_some() {
        container.pop();
    }
}

fn emit_fact(
    parsed: &ParsedSource,
    node: Node<'_>,
    kind: RustFactKind,
    name: Option<String>,
    container: &[String],
    facts: &mut Vec<RustFact>,
) {
    facts.push(RustFact {
        kind,
        name,
        text: parsed.text(node).to_owned(),
        container: container.to_vec(),
        span: parsed.span(node),
        provenance: parsed.provenance().clone(),
    });
}

fn direct_kind(node: Node<'_>, in_impl: bool) -> Option<RustFactKind> {
    match node.kind() {
        "mod_item" => Some(RustFactKind::Module),
        "struct_item" => Some(RustFactKind::Struct),
        "enum_item" => Some(RustFactKind::Enum),
        "enum_variant" => Some(RustFactKind::Variant),
        "trait_item" => Some(RustFactKind::Trait),
        "impl_item" => Some(RustFactKind::Impl),
        "function_item" if in_impl => Some(RustFactKind::Method),
        "function_item" => Some(RustFactKind::Function),
        "field_declaration" => Some(RustFactKind::Field),
        "const_item" => Some(RustFactKind::Const),
        "static_item" => Some(RustFactKind::Static),
        "attribute_item" => Some(RustFactKind::Attribute),
        "call_expression" => Some(RustFactKind::Call),
        "macro_invocation" => Some(RustFactKind::MacroCall),
        "if_expression" => Some(RustFactKind::If),
        "match_expression" => Some(RustFactKind::Match),
        "match_arm" => Some(RustFactKind::MatchArm),
        "return_expression" => Some(RustFactKind::Return),
        "line_comment" | "block_comment" => Some(RustFactKind::Comment),
        _ => None,
    }
}

fn explicit_name(parsed: &ParsedSource, node: Node<'_>) -> Option<String> {
    if node.kind() == "macro_invocation" {
        return explicit_macro_name(parsed, node);
    }
    node.child_by_field_name("name")
        .filter(|name| !name.is_error() && !name.is_missing() && !name.has_error())
        .map(|name| parsed.text(name).to_owned())
}

fn explicit_macro_name(parsed: &ParsedSource, node: Node<'_>) -> Option<String> {
    node.child_by_field_name("macro")
        .filter(|name| !name.is_error() && !name.is_missing() && !name.has_error())
        .map(|name| parsed.text(name).to_owned())
}

fn is_assertion_macro(parsed: &ParsedSource, node: Node<'_>) -> bool {
    let Some(name) = explicit_macro_name(parsed, node) else {
        return false;
    };
    let base = name.rsplit("::").next().unwrap_or(name.as_str());
    matches!(
        base,
        "assert"
            | "assert_eq"
            | "assert_ne"
            | "debug_assert"
            | "debug_assert_eq"
            | "debug_assert_ne"
    )
}

fn container_segment(
    parsed: &ParsedSource,
    node: Node<'_>,
    kind: Option<RustFactKind>,
    in_impl: bool,
) -> Option<String> {
    match kind {
        Some(
            RustFactKind::Module
            | RustFactKind::Struct
            | RustFactKind::Enum
            | RustFactKind::Trait
            | RustFactKind::Function
            | RustFactKind::Method,
        ) => explicit_name(parsed, node),
        Some(RustFactKind::Impl) => node
            .child_by_field_name("type")
            .filter(|target| !target.is_error() && !target.is_missing() && !target.has_error())
            .map(|target| parsed.text(target).to_owned()),
        _ if node.kind() == "function_item" && in_impl => explicit_name(parsed, node),
        _ => None,
    }
}
