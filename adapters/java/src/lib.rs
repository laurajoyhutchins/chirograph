#![forbid(unsafe_code)]

use chirograph_core::model::{Observation, ObservationId, Revision, SourceId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
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
pub struct JavaAcquisition {
    pub facts: Vec<JavaFact>,
    pub observations: Vec<Observation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaEvidenceCandidate {
    pub fact_index: usize,
    pub fact: JavaFact,
    pub observation: Observation,
    pub matched_terms: Vec<String>,
    pub score: usize,
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

pub fn observe_java_source(
    source_id: SourceId,
    revision: Revision,
    path: &str,
    source: &str,
) -> Result<JavaAcquisition, JavaAdapterError> {
    let facts = extract_java_facts(path, source)?;
    let observations = facts
        .iter()
        .map(|fact| Observation {
            id: observation_id(&source_id, fact),
            source: source_id.clone(),
            revision: revision.clone(),
            locator: locator(fact),
            fact: format!("Java {}: {}", fact_kind_name(fact.kind), fact.text),
        })
        .collect();

    Ok(JavaAcquisition {
        facts,
        observations,
    })
}

#[must_use]
pub fn rank_java_evidence(
    acquisition: &JavaAcquisition,
    query: &str,
    allowed_kinds: &[JavaFactKind],
) -> Vec<JavaEvidenceCandidate> {
    let query_terms = lexical_terms(query);
    if query_terms.is_empty() {
        return Vec::new();
    }

    let mut candidates = acquisition
        .facts
        .iter()
        .enumerate()
        .filter(|(_, fact)| allowed_kinds.is_empty() || allowed_kinds.contains(&fact.kind))
        .map(|(fact_index, fact)| {
            let mut fact_terms = lexical_terms(&fact.text);
            if let Some(name) = &fact.name {
                fact_terms.extend(lexical_terms(name));
            }
            if let Some(condition) = &fact.condition {
                fact_terms.extend(lexical_terms(condition));
            }
            let matched_terms = query_terms
                .intersection(&fact_terms)
                .cloned()
                .collect::<Vec<_>>();
            let score = matched_terms.len() * 100 + fact_kind_weight(fact.kind);
            JavaEvidenceCandidate {
                fact_index,
                fact: fact.clone(),
                observation: acquisition.observations[fact_index].clone(),
                matched_terms,
                score,
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| right.matched_terms.len().cmp(&left.matched_terms.len()))
            .then_with(|| left.fact_index.cmp(&right.fact_index))
    });
    candidates
}

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

fn lexical_terms(value: &str) -> BTreeSet<String> {
    let chars = value.chars().collect::<Vec<_>>();
    let mut terms = BTreeSet::new();
    let mut current = String::new();

    for (index, character) in chars.iter().copied().enumerate() {
        if !character.is_ascii_alphanumeric() {
            insert_term(&mut terms, &mut current);
            continue;
        }

        let previous = index.checked_sub(1).and_then(|position| chars.get(position));
        let next = chars.get(index + 1);
        let camel_boundary = character.is_ascii_uppercase()
            && !current.is_empty()
            && previous.is_some_and(|value| {
                value.is_ascii_lowercase()
                    || value.is_ascii_digit()
                    || (value.is_ascii_uppercase()
                        && next.is_some_and(|next| next.is_ascii_lowercase()))
            });
        if camel_boundary {
            insert_term(&mut terms, &mut current);
        }
        current.push(character.to_ascii_lowercase());
    }
    insert_term(&mut terms, &mut current);
    terms
}

fn insert_term(terms: &mut BTreeSet<String>, current: &mut String) {
    if current.is_empty() {
        return;
    }
    let term = std::mem::take(current);
    if term.len() >= 2 && !is_stop_word(&term) {
        terms.insert(term);
    }
}

fn is_stop_word(term: &str) -> bool {
    matches!(
        term,
        "a" | "an"
            | "and"
            | "are"
            | "as"
            | "at"
            | "be"
            | "by"
            | "for"
            | "from"
            | "in"
            | "into"
            | "is"
            | "of"
            | "on"
            | "or"
            | "per"
            | "the"
            | "to"
            | "with"
    )
}

const fn fact_kind_weight(kind: JavaFactKind) -> usize {
    match kind {
        JavaFactKind::ConditionalThrow => 50,
        JavaFactKind::TestAssertion => 40,
        JavaFactKind::FieldDeclaration => 30,
        JavaFactKind::Comment => 20,
        JavaFactKind::MethodInvocation => 10,
    }
}

fn observation_id(source_id: &SourceId, fact: &JavaFact) -> ObservationId {
    ObservationId::new(format!(
        "obs.java.{}.{}.{}.{}.{}",
        source_id.as_str(),
        fact.path,
        fact.span.start_line,
        fact.span.start_column,
        fact_kind_name(fact.kind),
    ))
    .expect("generated Java observation ids are non-empty and trimmed")
}

fn locator(fact: &JavaFact) -> String {
    format!(
        "{}:L{}:C{}-L{}:C{}",
        fact.path,
        fact.span.start_line,
        fact.span.start_column,
        fact.span.end_line,
        fact.span.end_column,
    )
}

const fn fact_kind_name(kind: JavaFactKind) -> &'static str {
    match kind {
        JavaFactKind::FieldDeclaration => "field declaration",
        JavaFactKind::MethodInvocation => "method invocation",
        JavaFactKind::ConditionalThrow => "conditional throw",
        JavaFactKind::Comment => "comment",
        JavaFactKind::TestAssertion => "test assertion",
    }
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
