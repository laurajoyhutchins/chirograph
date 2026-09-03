#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JavaFactKind {
    FieldDeclaration,
    MethodInvocation,
    ConditionalThrow,
    Comment,
    TestAssertion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JavaFact {
    pub kind: JavaFactKind,
    pub path: String,
    pub span: SourceSpan,
    pub name: Option<String>,
    pub condition: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaAdapterError {
    Language(String),
    ParseFailed,
    SyntaxError,
}

impl fmt::Display for JavaAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Language(message) => write!(formatter, "cannot load Java grammar: {message}"),
            Self::ParseFailed => {
                formatter.write_str("Tree-sitter did not produce a Java syntax tree")
            }
            Self::SyntaxError => formatter.write_str("Java source contains syntax errors"),
        }
    }
}

impl std::error::Error for JavaAdapterError {}

pub fn extract_java_facts(path: &str, source: &str) -> Result<Vec<JavaFact>, JavaAdapterError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .map_err(|error| JavaAdapterError::Language(error.to_string()))?;
    let tree = parser
        .parse(source, None)
        .ok_or(JavaAdapterError::ParseFailed)?;
    if tree.root_node().has_error() {
        return Err(JavaAdapterError::SyntaxError);
    }

    let mut facts = Vec::new();
    collect_facts(tree.root_node(), source.as_bytes(), path, &mut facts);
    facts.sort_by(|left, right| {
        left.span
            .start_line
            .cmp(&right.span.start_line)
            .then_with(|| left.span.start_column.cmp(&right.span.start_column))
            .then_with(|| fact_kind_order(left.kind).cmp(&fact_kind_order(right.kind)))
    });
    Ok(facts)
}

fn fact_kind_order(kind: JavaFactKind) -> u8 {
    match kind {
        JavaFactKind::FieldDeclaration => 0,
        JavaFactKind::MethodInvocation => 1,
        JavaFactKind::ConditionalThrow => 2,
        JavaFactKind::Comment => 3,
        JavaFactKind::TestAssertion => 4,
    }
}

fn collect_facts(node: Node<'_>, source: &[u8], path: &str, facts: &mut Vec<JavaFact>) {
    match node.kind() {
        "field_declaration" => facts.push(JavaFact {
            kind: JavaFactKind::FieldDeclaration,
            path: path.into(),
            span: span(node),
            name: field_name(node, source),
            condition: None,
            text: text(node, source),
        }),
        "method_invocation" => {
            let name = node
                .child_by_field_name("name")
                .map(|name| text(name, source));
            let kind = match name.as_deref() {
                Some(value) if value.starts_with("assert") => JavaFactKind::TestAssertion,
                _ => JavaFactKind::MethodInvocation,
            };
            facts.push(JavaFact {
                kind,
                path: path.into(),
                span: span(node),
                name,
                condition: None,
                text: text(node, source),
            });
        }
        "if_statement" => {
            if let Some(throw) = first_descendant(node, "throw_statement") {
                facts.push(JavaFact {
                    kind: JavaFactKind::ConditionalThrow,
                    path: path.into(),
                    span: span(node),
                    name: thrown_type(throw, source),
                    condition: node
                        .child_by_field_name("condition")
                        .map(|condition| text(condition, source)),
                    text: text(node, source),
                });
            }
        }
        "line_comment" | "block_comment" => facts.push(JavaFact {
            kind: JavaFactKind::Comment,
            path: path.into(),
            span: span(node),
            name: None,
            condition: None,
            text: text(node, source),
        }),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_facts(child, source, path, facts);
    }
}

fn field_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let declarator = first_descendant(node, "variable_declarator")?;
    declarator
        .child_by_field_name("name")
        .map(|name| text(name, source))
}

fn thrown_type(node: Node<'_>, source: &[u8]) -> Option<String> {
    let creation = first_descendant(node, "object_creation_expression")?;
    creation
        .child_by_field_name("type")
        .or_else(|| first_descendant(creation, "type_identifier"))
        .map(|kind| text(kind, source))
}

fn first_descendant<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
        if let Some(found) = first_descendant(child, kind) {
            return Some(found);
        }
    }
    None
}

fn text(node: Node<'_>, source: &[u8]) -> String {
    node.utf8_text(source).unwrap_or_default().to_owned()
}

fn span(node: Node<'_>) -> SourceSpan {
    let start = node.start_position();
    let end = node.end_position();
    SourceSpan {
        start_line: start.row + 1,
        start_column: start.column + 1,
        end_line: end.row + 1,
        end_column: end.column + 1,
    }
}
