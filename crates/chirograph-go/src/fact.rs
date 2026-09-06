use chirograph_tree_sitter::{ParseDiagnostic, SourceProvenance, SourceSpan};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GoFactKind {
    Package,
    Type,
    Struct,
    Interface,
    Field,
    Tag,
    Const,
    Var,
    Function,
    Method,
    Receiver,
    Parameter,
    TypeExpression,
    Call,
    If,
    Switch,
    For,
    Return,
    Panic,
    Comment,
    Assertion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoFact {
    pub kind: GoFactKind,
    pub name: Option<String>,
    pub text: String,
    pub container: Vec<String>,
    pub span: SourceSpan,
    pub provenance: SourceProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoExtraction {
    pub facts: Vec<GoFact>,
    pub diagnostics: Vec<ParseDiagnostic>,
}