use std::{error::Error, fmt};

use chirograph_tree_sitter::{ParseError, ParsedSource, SourceProvenance, parse_utf8};
use tree_sitter::Node;

use crate::fact::{GoExtraction, GoFact, GoFactKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoAdapterError {
    Parse(ParseError),
}

impl fmt::Display for GoAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "cannot parse Go source: {error}"),
        }
    }
}

impl Error for GoAdapterError {}

impl From<ParseError> for GoAdapterError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

pub fn extract_go_facts(
    source: &[u8],
    provenance: SourceProvenance,
) -> Result<GoExtraction, GoAdapterError> {
    let language = tree_sitter_go::LANGUAGE.into();
    let parsed = parse_utf8(&language, source, provenance)?;
    let diagnostics = parsed.diagnostics().to_vec();
    let mut facts = Vec::new();
    let mut container = Vec::new();

    walk(
        &parsed,
        parsed.tree().root_node(),
        &mut container,
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

    Ok(GoExtraction { facts, diagnostics })
}

fn walk(
    parsed: &ParsedSource,
    node: Node<'_>,
    container: &mut Vec<String>,
    facts: &mut Vec<GoFact>,
) {
    let malformed = node.is_error() || node.is_missing() || node.has_error();
    let kind = direct_kind(node);

    if !malformed {
        if let Some(kind) = kind {
            emit_fact(
                parsed,
                node,
                kind,
                explicit_name(parsed, node),
                container,
                facts,
            );
        }

        emit_derived_facts(parsed, node, container, facts);
    }

    let segment = container_segment(parsed, node, kind);
    if let Some(segment) = &segment {
        container.push(segment.clone());
    }

    for index in 0..node.child_count() {
        if let Some(child) = node.child(index as u32) {
            walk(parsed, child, container, facts);
        }
    }

    if segment.is_some() {
        container.pop();
    }
}

fn emit_derived_facts(
    parsed: &ParsedSource,
    node: Node<'_>,
    container: &[String],
    facts: &mut Vec<GoFact>,
) {
    if node.kind() == "type_spec"
        && let Some(type_node) = clean_field(node, "type")
    {
        emit_fact(
            parsed,
            type_node,
            GoFactKind::TypeExpression,
            None,
            container,
            facts,
        );
        match type_node.kind() {
            "struct_type" => emit_fact(
                parsed,
                type_node,
                GoFactKind::Struct,
                explicit_name(parsed, node),
                container,
                facts,
            ),
            "interface_type" => emit_fact(
                parsed,
                type_node,
                GoFactKind::Interface,
                explicit_name(parsed, node),
                container,
                facts,
            ),
            _ => {}
        }
    }

    if matches!(
        node.kind(),
        "field_declaration"
            | "parameter_declaration"
            | "variadic_parameter_declaration"
            | "const_spec"
            | "var_spec"
    ) && let Some(type_node) = clean_field(node, "type")
    {
        emit_fact(
            parsed,
            type_node,
            GoFactKind::TypeExpression,
            None,
            container,
            facts,
        );
    }

    if node.kind() == "field_declaration"
        && let Some(tag) = clean_field(node, "tag")
    {
        emit_fact(parsed, tag, GoFactKind::Tag, None, container, facts);
    }

    if node.kind() == "method_declaration"
        && let Some(receiver) = clean_field(node, "receiver")
    {
        emit_fact(
            parsed,
            receiver,
            GoFactKind::Receiver,
            None,
            container,
            facts,
        );
    }

    if node.kind() == "call_expression" {
        let function = clean_field(node, "function")
            .map(|function| parsed.text(function))
            .unwrap_or_default();
        if function == "panic" {
            emit_fact(
                parsed,
                node,
                GoFactKind::Panic,
                Some("panic".to_owned()),
                container,
                facts,
            );
        }
        if is_assertion_call(parsed, node) {
            emit_fact(
                parsed,
                node,
                GoFactKind::Assertion,
                call_name(parsed, node),
                container,
                facts,
            );
        }
    }
}

fn direct_kind(node: Node<'_>) -> Option<GoFactKind> {
    match node.kind() {
        "package_clause" => Some(GoFactKind::Package),
        "type_spec" => Some(GoFactKind::Type),
        "field_declaration" => Some(GoFactKind::Field),
        "const_spec" => Some(GoFactKind::Const),
        "var_spec" => Some(GoFactKind::Var),
        "function_declaration" => Some(GoFactKind::Function),
        "method_declaration" => Some(GoFactKind::Method),
        "parameter_declaration" | "variadic_parameter_declaration" => Some(GoFactKind::Parameter),
        "call_expression" => Some(GoFactKind::Call),
        "if_statement" => Some(GoFactKind::If),
        "expression_switch_statement" | "type_switch_statement" => Some(GoFactKind::Switch),
        "for_statement" => Some(GoFactKind::For),
        "return_statement" => Some(GoFactKind::Return),
        "comment" => Some(GoFactKind::Comment),
        _ => None,
    }
}

fn emit_fact(
    parsed: &ParsedSource,
    node: Node<'_>,
    kind: GoFactKind,
    name: Option<String>,
    container: &[String],
    facts: &mut Vec<GoFact>,
) {
    facts.push(GoFact {
        kind,
        name,
        text: parsed.text(node).to_owned(),
        container: container.to_vec(),
        span: parsed.span(node),
        provenance: parsed.provenance().clone(),
    });
}

fn clean_field<'tree>(node: Node<'tree>, field: &str) -> Option<Node<'tree>> {
    node.child_by_field_name(field)
        .filter(|child| !child.is_error() && !child.is_missing() && !child.has_error())
}

fn explicit_name(parsed: &ParsedSource, node: Node<'_>) -> Option<String> {
    match node.kind() {
        "package_clause" => node
            .named_child(0)
            .filter(|child| !child.has_error())
            .map(|child| parsed.text(child).to_owned()),
        "call_expression" => call_name(parsed, node),
        _ => clean_field(node, "name").map(|name| parsed.text(name).to_owned()),
    }
}

fn call_name(parsed: &ParsedSource, node: Node<'_>) -> Option<String> {
    clean_field(node, "function").map(|function| match function.kind() {
        "selector_expression" => clean_field(function, "field")
            .map(|field| parsed.text(field).to_owned())
            .unwrap_or_else(|| parsed.text(function).to_owned()),
        _ => parsed.text(function).to_owned(),
    })
}

fn is_assertion_call(parsed: &ParsedSource, node: Node<'_>) -> bool {
    let Some(function) = clean_field(node, "function") else {
        return false;
    };
    if function.kind() != "selector_expression" {
        return false;
    }
    let Some(field) = clean_field(function, "field") else {
        return false;
    };
    matches!(
        parsed.text(field),
        "Error" | "Errorf" | "Fail" | "FailNow" | "Fatal" | "Fatalf"
    )
}

fn container_segment(
    parsed: &ParsedSource,
    node: Node<'_>,
    kind: Option<GoFactKind>,
) -> Option<String> {
    match kind {
        Some(GoFactKind::Type | GoFactKind::Function | GoFactKind::Method) => {
            explicit_name(parsed, node)
        }
        _ => None,
    }
}
