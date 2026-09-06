use chirograph_tree_sitter::{ParseDiagnostic, SourceProvenance, SourceSpan};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JavaFactKind {
    Package,
    Import,
    Class,
    Interface,
    Enum,
    EnumConstant,
    Record,
    AnnotationDeclaration,
    Field,
    Constant,
    Literal,
    TypeParameter,
    TypeExpression,
    Signature,
    Method,
    Constructor,
    Parameter,
    Annotation,
    AnnotationArgument,
    Call,
    If,
    Switch,
    Return,
    Throw,
    Comment,
    Assertion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaFact {
    pub kind: JavaFactKind,
    pub name: Option<String>,
    pub text: String,
    pub container: Vec<String>,
    pub span: SourceSpan,
    pub provenance: SourceProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaExtraction {
    pub facts: Vec<JavaFact>,
    pub diagnostics: Vec<ParseDiagnostic>,
}
