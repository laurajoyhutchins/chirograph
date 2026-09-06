use std::{error::Error, fmt};

use chirograph_tree_sitter::{ParseError, ParsedSource, SourceProvenance, parse_utf8};
use tree_sitter::Node;

use crate::fact::{JavaExtraction, JavaFact, JavaFactKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaAdapterError {
    Parse(ParseError),
}

impl fmt::Display for JavaAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "cannot parse Java source: {error}"),
        }
    }
}

impl Error for JavaAdapterError {}

impl From<ParseError> for JavaAdapterError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

pub fn extract_java_facts(
    source: &[u8],
    provenance: SourceProvenance,
) -> Result<JavaExtraction, JavaAdapterError> {
    let language = tree_sitter_java::LANGUAGE.into();
    let parsed = parse_utf8(&language, source, provenance)?;
    let mut facts = Vec::new();

    for node in parsed.preorder() {
        emit_node_facts(&parsed, node, &mut facts);
    }

    facts.sort_by(|left, right| {
        (
            left.span.start_byte,
            left.span.end_byte,
            left.kind,
            &left.name,
            &left.text,
            &left.container,
        )
            .cmp(&(
                right.span.start_byte,
                right.span.end_byte,
                right.kind,
                &right.name,
                &right.text,
                &right.container,
            ))
    });
    facts.dedup();

    Ok(JavaExtraction {
        facts,
        diagnostics: parsed.diagnostics().to_vec(),
    })
}

fn emit_node_facts(parsed: &ParsedSource, node: Node<'_>, facts: &mut Vec<JavaFact>) {
    if node.is_error() || node.is_missing() || node.start_byte() == node.end_byte() {
        return;
    }

    let kind = match node.kind() {
        "package_declaration" => Some(JavaFactKind::Package),
        "import_declaration" => Some(JavaFactKind::Import),
        "class_declaration" => Some(JavaFactKind::Class),
        "interface_declaration" => Some(JavaFactKind::Interface),
        "enum_declaration" => Some(JavaFactKind::Enum),
        "enum_constant" => Some(JavaFactKind::EnumConstant),
        "record_declaration" => Some(JavaFactKind::Record),
        "annotation_type_declaration" => Some(JavaFactKind::AnnotationDeclaration),
        "field_declaration" => Some(JavaFactKind::Field),
        "type_parameter" => Some(JavaFactKind::TypeParameter),
        "method_declaration" => Some(JavaFactKind::Method),
        "constructor_declaration" => Some(JavaFactKind::Constructor),
        "formal_parameter" | "spread_parameter" | "receiver_parameter" => {
            Some(JavaFactKind::Parameter)
        }
        "annotation" | "marker_annotation" => Some(JavaFactKind::Annotation),
        "annotation_argument_list" | "element_value_pair" => Some(JavaFactKind::AnnotationArgument),
        "method_invocation" | "object_creation_expression" => Some(JavaFactKind::Call),
        "if_statement" => Some(JavaFactKind::If),
        "switch_expression" => Some(JavaFactKind::Switch),
        "return_statement" => Some(JavaFactKind::Return),
        "throw_statement" => Some(JavaFactKind::Throw),
        "line_comment" | "block_comment" => Some(JavaFactKind::Comment),
        "assert_statement" => Some(JavaFactKind::Assertion),
        candidate if is_literal(candidate) => Some(JavaFactKind::Literal),
        _ => None,
    };

    if let Some(kind) = kind {
        push_fact(parsed, node, kind, node_name(parsed, node), facts);
    }

    match node.kind() {
        "field_declaration" => {
            if is_final_field(parsed, node) {
                push_fact(
                    parsed,
                    node,
                    JavaFactKind::Constant,
                    node_name(parsed, node),
                    facts,
                );
            }
            emit_type_field(parsed, node, facts);
        }
        "formal_parameter" | "spread_parameter" | "receiver_parameter" => {
            emit_type_field(parsed, node, facts);
        }
        "method_declaration" => {
            push_fact(
                parsed,
                node,
                JavaFactKind::Signature,
                node_name(parsed, node),
                facts,
            );
            emit_type_field(parsed, node, facts);
        }
        "constructor_declaration" => push_fact(
            parsed,
            node,
            JavaFactKind::Signature,
            node_name(parsed, node),
            facts,
        ),
        "method_invocation" if is_assertion_call(parsed, node) => push_fact(
            parsed,
            node,
            JavaFactKind::Assertion,
            node_name(parsed, node),
            facts,
        ),
        "object_creation_expression" => emit_type_field(parsed, node, facts),
        _ => {}
    }
}

fn emit_type_field(parsed: &ParsedSource, node: Node<'_>, facts: &mut Vec<JavaFact>) {
    let Some(type_node) = node.child_by_field_name("type") else {
        return;
    };
    push_fact(
        parsed,
        type_node,
        JavaFactKind::TypeExpression,
        Some(parsed.text(type_node).to_owned()),
        facts,
    );
}

fn push_fact(
    parsed: &ParsedSource,
    node: Node<'_>,
    kind: JavaFactKind,
    name: Option<String>,
    facts: &mut Vec<JavaFact>,
) {
    let text = parsed.text(node);
    if text.is_empty() {
        return;
    }
    facts.push(JavaFact {
        kind,
        name,
        text: text.to_owned(),
        container: container_path(parsed, node),
        span: parsed.span(node),
        provenance: parsed.provenance().clone(),
    });
}

fn node_name(parsed: &ParsedSource, node: Node<'_>) -> Option<String> {
    node.child_by_field_name("name")
        .or_else(|| {
            node.child_by_field_name("declarator")
                .and_then(|declarator| declarator.child_by_field_name("name"))
        })
        .map(|name| parsed.text(name).trim().to_owned())
        .filter(|name| !name.is_empty())
}

fn container_path(parsed: &ParsedSource, node: Node<'_>) -> Vec<String> {
    let mut names = Vec::new();
    let mut current = node.parent();
    while let Some(parent) = current {
        if is_container(parent.kind()) {
            if let Some(name) = node_name(parsed, parent) {
                names.push(name);
            }
        }
        current = parent.parent();
    }
    names.reverse();
    names
}

fn is_container(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "annotation_type_declaration"
            | "method_declaration"
            | "constructor_declaration"
    )
}

fn is_final_field(parsed: &ParsedSource, node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| {
        child.kind() == "modifiers"
            && parsed
                .text(child)
                .split_whitespace()
                .any(|modifier| modifier == "final")
    })
}

fn is_assertion_call(parsed: &ParsedSource, node: Node<'_>) -> bool {
    let Some(name) = node.child_by_field_name("name") else {
        return false;
    };
    let name = parsed.text(name);
    name.starts_with("assert") || matches!(name, "fail" | "failNow")
}

fn is_literal(kind: &str) -> bool {
    matches!(
        kind,
        "binary_integer_literal"
            | "character_literal"
            | "class_literal"
            | "decimal_floating_point_literal"
            | "decimal_integer_literal"
            | "false"
            | "hex_floating_point_literal"
            | "hex_integer_literal"
            | "null_literal"
            | "octal_integer_literal"
            | "string_literal"
            | "true"
    )
}
