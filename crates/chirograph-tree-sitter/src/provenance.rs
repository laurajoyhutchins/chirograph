use chirograph_core::model::{Revision, SourceId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProvenance {
    pub source: SourceId,
    pub revision: Revision,
    pub locator: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourcePoint {
    pub row: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start: SourcePoint,
    pub end: SourcePoint,
}

impl From<tree_sitter::Point> for SourcePoint {
    fn from(value: tree_sitter::Point) -> Self {
        Self {
            row: value.row,
            column: value.column,
        }
    }
}

pub(crate) fn span_of(node: tree_sitter::Node<'_>) -> SourceSpan {
    SourceSpan {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start: node.start_position().into(),
        end: node.end_position().into(),
    }
}
