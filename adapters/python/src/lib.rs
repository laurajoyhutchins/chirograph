#![forbid(unsafe_code)]

use chirograph_core::model::{Observation, ObservationId, Revision, SourceId};
use serde::{Deserialize, Serialize};
use std::fmt;
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PythonFactKind {
    Module,
    ClassDefinition,
    FunctionDefinition,
    AnnotatedAssignment,
    Decorator,
    Call,
    Return,
    Raise,
    Conditional,
    Comment,
    Docstring,
    TestAssertion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PythonFact {
    pub kind: PythonFactKind,
    pub path: String,
    pub span: SourceSpan,
    pub name: Option<String>,
    pub annotation: Option<String>,
    pub condition: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonAcquisition {
    pub facts: Vec<PythonFact>,
    pub observations: Vec<Observation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PythonAdapterError {
    Language(String),
    ParseFailed,
    SyntaxError,
}

impl fmt::Display for PythonAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Language(message) => write!(formatter, "cannot load Python grammar: {message}"),
            Self::ParseFailed => {
                formatter.write_str("Tree-sitter did not produce a Python syntax tree")
            }
            Self::SyntaxError => formatter.write_str("Python source contains syntax errors"),
        }
    }
}

impl std::error::Error for PythonAdapterError {}

pub fn observe_python_source(
    source_id: SourceId,
    revision: Revision,
    path: &str,
    source: &str,
) -> Result<PythonAcquisition, PythonAdapterError> {
    let facts = extract_python_facts(path, source)?;
    let observations = facts
        .iter()
        .map(|fact| Observation {
            id: observation_id(&source_id, fact),
            source: source_id.clone(),
            revision: revision.clone(),
            locator: locator(fact),
            fact: observation_fact(fact),
        })
        .collect();

    Ok(PythonAcquisition {
        facts,
        observations,
    })
}

pub fn extract_python_facts(
    path: &str,
    source: &str,
) -> Result<Vec<PythonFact>, PythonAdapterError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .map_err(|error| PythonAdapterError::Language(error.to_string()))?;
    let tree = parser
        .parse(source, None)
        .ok_or(PythonAdapterError::ParseFailed)?;
    let root = tree.root_node();
    if root.has_error() {
        return Err(PythonAdapterError::SyntaxError);
    }

    let mut facts = vec![PythonFact {
        kind: PythonFactKind::Module,
        path: path.into(),
        span: span(root),
        name: Some(path.into()),
        annotation: None,
        condition: None,
        text: source.into(),
    }];
    collect_facts(root, source.as_bytes(), path, &mut facts);
    facts.sort_by(|left, right| {
        left.span
            .start_byte
            .cmp(&right.span.start_byte)
            .then_with(|| fact_kind_order(left.kind).cmp(&fact_kind_order(right.kind)))
            .then_with(|| left.span.end_byte.cmp(&right.span.end_byte))
    });
    Ok(facts)
}

fn collect_facts(node: Node<'_>, source: &[u8], path: &str, facts: &mut Vec<PythonFact>) {
    match node.kind() {
        "class_definition" => facts.push(PythonFact {
            kind: PythonFactKind::ClassDefinition,
            path: path.into(),
            span: span(node),
            name: node.child_by_field_name("name").map(|value| text(value, source)),
            annotation: None,
            condition: None,
            text: text(node, source),
        }),
        "function_definition" => facts.push(PythonFact {
            kind: PythonFactKind::FunctionDefinition,
            path: path.into(),
            span: span(node),
            name: node.child_by_field_name("name").map(|value| text(value, source)),
            annotation: node
                .child_by_field_name("return_type")
                .map(|value| text(value, source)),
            condition: None,
            text: text(node, source),
        }),
        "assignment" if node.child_by_field_name("type").is_some() => facts.push(PythonFact {
            kind: PythonFactKind::AnnotatedAssignment,
            path: path.into(),
            span: span(node),
            name: node.child_by_field_name("left").map(|value| text(value, source)),
            annotation: node
                .child_by_field_name("type")
                .map(|value| text(value, source)),
            condition: None,
            text: text(node, source),
        }),
        "decorator" => facts.push(PythonFact {
            kind: PythonFactKind::Decorator,
            path: path.into(),
            span: span(node),
            name: decorator_name(node, source),
            annotation: None,
            condition: None,
            text: text(node, source),
        }),
        "call" => facts.push(PythonFact {
            kind: PythonFactKind::Call,
            path: path.into(),
            span: span(node),
            name: node
                .child_by_field_name("function")
                .map(|value| text(value, source)),
            annotation: None,
            condition: None,
            text: text(node, source),
        }),
        "return_statement" => facts.push(simple_fact(PythonFactKind::Return, node, source, path)),
        "raise_statement" => facts.push(simple_fact(PythonFactKind::Raise, node, source, path)),
        "if_statement" | "elif_clause" => facts.push(PythonFact {
            kind: PythonFactKind::Conditional,
            path: path.into(),
            span: span(node),
            name: None,
            annotation: None,
            condition: node
                .child_by_field_name("condition")
                .map(|value| text(value, source)),
            text: text(node, source),
        }),
        "comment" => facts.push(simple_fact(PythonFactKind::Comment, node, source, path)),
        "assert_statement" => facts.push(simple_fact(
            PythonFactKind::TestAssertion,
            node,
            source,
            path,
        )),
        "expression_statement" if is_docstring_statement(node) => facts.push(simple_fact(
            PythonFactKind::Docstring,
            node,
            source,
            path,
        )),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_facts(child, source, path, facts);
    }
}

fn simple_fact(kind: PythonFactKind, node: Node<'_>, source: &[u8], path: &str) -> PythonFact {
    PythonFact {
        kind,
        path: path.into(),
        span: span(node),
        name: None,
        annotation: None,
        condition: None,
        text: text(node, source),
    }
}

fn decorator_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let expression = node.named_child(0)?;
    if expression.kind() == "call" {
        return expression
            .child_by_field_name("function")
            .map(|value| text(value, source));
    }
    Some(text(expression, source))
}

fn is_docstring_statement(node: Node<'_>) -> bool {
    let Some(value) = node.named_child(0) else {
        return false;
    };
    if !matches!(value.kind(), "string" | "concatenated_string") {
        return false;
    }

    let Some(parent) = node.parent() else {
        return false;
    };
    let belongs_to_documentable_scope = match parent.kind() {
        "module" => true,
        "block" => parent.parent().is_some_and(|owner| {
            matches!(owner.kind(), "class_definition" | "function_definition")
        }),
        _ => false,
    };
    if !belongs_to_documentable_scope {
        return false;
    }

    let mut cursor = parent.walk();
    parent
        .named_children(&mut cursor)
        .filter(|child| child.kind() != "comment")
        .next()
        .is_some_and(|first| {
            first.kind() == node.kind()
                && first.start_byte() == node.start_byte()
                && first.end_byte() == node.end_byte()
        })
}

fn observation_id(source_id: &SourceId, fact: &PythonFact) -> ObservationId {
    ObservationId::new(format!(
        "obs.python.{}.{}.{}.{}.{}",
        source_id.as_str(),
        fact.path,
        fact.span.start_byte,
        fact.span.end_byte,
        fact_kind_name(fact.kind),
    ))
    .expect("generated Python observation ids are non-empty and trimmed")
}

fn locator(fact: &PythonFact) -> String {
    format!(
        "{}:B{}-B{}:L{}:C{}-L{}:C{}",
        fact.path,
        fact.span.start_byte,
        fact.span.end_byte,
        fact.span.start_line,
        fact.span.start_column,
        fact.span.end_line,
        fact.span.end_column,
    )
}

fn observation_fact(fact: &PythonFact) -> String {
    if fact.kind == PythonFactKind::Module {
        return format!("Python module: {}", fact.path);
    }
    format!("Python {}: {}", fact_kind_name(fact.kind), fact.text)
}

const fn fact_kind_name(kind: PythonFactKind) -> &'static str {
    match kind {
        PythonFactKind::Module => "module",
        PythonFactKind::ClassDefinition => "class definition",
        PythonFactKind::FunctionDefinition => "function definition",
        PythonFactKind::AnnotatedAssignment => "annotated assignment",
        PythonFactKind::Decorator => "decorator",
        PythonFactKind::Call => "call",
        PythonFactKind::Return => "return",
        PythonFactKind::Raise => "raise",
        PythonFactKind::Conditional => "conditional",
        PythonFactKind::Comment => "comment",
        PythonFactKind::Docstring => "docstring",
        PythonFactKind::TestAssertion => "test assertion",
    }
}

const fn fact_kind_order(kind: PythonFactKind) -> u8 {
    match kind {
        PythonFactKind::Module => 0,
        PythonFactKind::ClassDefinition => 1,
        PythonFactKind::FunctionDefinition => 2,
        PythonFactKind::AnnotatedAssignment => 3,
        PythonFactKind::Decorator => 4,
        PythonFactKind::Call => 5,
        PythonFactKind::Return => 6,
        PythonFactKind::Raise => 7,
        PythonFactKind::Conditional => 8,
        PythonFactKind::Comment => 9,
        PythonFactKind::Docstring => 10,
        PythonFactKind::TestAssertion => 11,
    }
}

fn text(node: Node<'_>, source: &[u8]) -> String {
    node.utf8_text(source).unwrap_or_default().to_owned()
}

fn span(node: Node<'_>) -> SourceSpan {
    let start = node.start_position();
    let end = node.end_position();
    SourceSpan {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_line: start.row + 1,
        start_column: start.column + 1,
        end_line: end.row + 1,
        end_column: end.column + 1,
    }
}
