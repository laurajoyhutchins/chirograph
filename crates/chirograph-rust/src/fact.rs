use chirograph_tree_sitter::{ParseDiagnostic, SourceProvenance, SourceSpan};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RustFactKind {
    Module,
    Struct,
    Enum,
    Variant,
    Trait,
    Impl,
    Function,
    Method,
    Field,
    Const,
    Static,
    TypeExpression,
    Attribute,
    Call,
    MacroCall,
    If,
    Match,
    MatchArm,
    Return,
    Comment,
    Assertion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustFact {
    pub kind: RustFactKind,
    pub name: Option<String>,
    pub text: String,
    pub container: Vec<String>,
    pub span: SourceSpan,
    pub provenance: SourceProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustExtraction {
    pub facts: Vec<RustFact>,
    pub diagnostics: Vec<ParseDiagnostic>,
}
