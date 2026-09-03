use std::{error::Error, fmt};

use tree_sitter::{Language, Node, Parser, Tree};

use crate::provenance::{SourceProvenance, SourceSpan, span_of};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseDiagnosticKind {
    ErrorNode,
    MissingNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnostic {
    pub kind: ParseDiagnosticKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidUtf8(usize),
    Language(String),
    NoTree,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8(offset) => {
                write!(formatter, "source is not valid UTF-8 at byte {offset}")
            }
            Self::Language(message) => {
                write!(formatter, "Tree-sitter language is incompatible: {message}")
            }
            Self::NoTree => formatter.write_str("Tree-sitter parser returned no tree"),
        }
    }
}

impl Error for ParseError {}

pub struct ParsedSource {
    source: String,
    tree: Tree,
    provenance: SourceProvenance,
    diagnostics: Vec<ParseDiagnostic>,
}

impl ParsedSource {
    #[must_use]
    pub fn provenance(&self) -> &SourceProvenance {
        &self.provenance
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[ParseDiagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub fn span(&self, node: Node<'_>) -> SourceSpan {
        span_of(node)
    }

    #[must_use]
    pub fn text(&self, node: Node<'_>) -> &str {
        node.utf8_text(self.source.as_bytes())
            .expect("Tree-sitter node boundaries from validated UTF-8 source must remain UTF-8")
    }

    #[must_use]
    pub fn preorder(&self) -> Vec<Node<'_>> {
        let mut nodes = Vec::new();
        collect_preorder(self.tree.root_node(), &mut nodes);
        nodes
    }
}

pub fn parse_utf8(
    language: &Language,
    source: &[u8],
    provenance: SourceProvenance,
) -> Result<ParsedSource, ParseError> {
    let source = std::str::from_utf8(source)
        .map_err(|error| ParseError::InvalidUtf8(error.valid_up_to()))?
        .to_owned();

    let mut parser = Parser::new();
    parser
        .set_language(language)
        .map_err(|error| ParseError::Language(error.to_string()))?;
    let tree = parser.parse(&source, None).ok_or(ParseError::NoTree)?;
    let diagnostics = collect_diagnostics(tree.root_node());

    Ok(ParsedSource {
        source,
        tree,
        provenance,
        diagnostics,
    })
}

fn collect_preorder<'tree>(node: Node<'tree>, nodes: &mut Vec<Node<'tree>>) {
    nodes.push(node);
    for index in 0..node.child_count() {
        if let Some(child) = node.child(index as u32) {
            collect_preorder(child, nodes);
        }
    }
}

fn collect_diagnostics(root: Node<'_>) -> Vec<ParseDiagnostic> {
    let mut nodes = Vec::new();
    collect_preorder(root, &mut nodes);

    let mut diagnostics = Vec::new();
    for node in nodes {
        if node.is_error() {
            diagnostics.push(ParseDiagnostic {
                kind: ParseDiagnosticKind::ErrorNode,
                span: span_of(node),
            });
        }
        if node.is_missing() {
            diagnostics.push(ParseDiagnostic {
                kind: ParseDiagnosticKind::MissingNode,
                span: span_of(node),
            });
        }
    }

    diagnostics.sort_by_key(|diagnostic| {
        (
            diagnostic.span.start_byte,
            diagnostic.span.end_byte,
            diagnostic.kind,
        )
    });
    diagnostics.dedup();
    diagnostics
}
