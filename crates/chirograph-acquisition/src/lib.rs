#![forbid(unsafe_code)]

//! Deterministic source discovery and adapter dispatch for Chirograph.
//!
//! This crate owns acquisition mechanics only. It does not infer logical contract
//! identity, alignment, authority, or clause truth.

use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use chirograph_core::model::{Revision, SourceId};
use chirograph_go::{GoFactKind, extract_go_facts};
use chirograph_rust::{RustFactKind, extract_rust_facts};
use chirograph_tree_sitter::{SourceProvenance, SourceSpan};
use serde_json::Value;

const MAX_DIAGNOSTICS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquisitionContext {
    pub source: SourceId,
    pub revision: Revision,
}

impl AcquisitionContext {
    #[must_use]
    pub fn new(source: SourceId, revision: Revision) -> Self {
        Self { source, revision }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdapterFamily {
    TreeSitter,
    StructuredSemantic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterCapability {
    pub adapter: String,
    pub family: AdapterFamily,
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterError {
    message: String,
}

impl AdapterError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for AdapterError {}

#[derive(Debug, Clone, Copy)]
pub struct AdapterInput<'a> {
    pub bytes: &'a [u8],
    pub path: &'a str,
    pub context: &'a AcquisitionContext,
}

pub trait SourceAdapter: fmt::Debug + Send + Sync {
    fn capability(&self) -> AdapterCapability;

    fn acquire(&self, input: AdapterInput<'_>) -> Result<Vec<AcquiredFact>, AdapterError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticKind {
    UnsupportedSource,
    DiagnosticsTruncated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquisitionDiagnostic {
    pub kind: DiagnosticKind,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AcquiredSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_row: usize,
    pub start_column: usize,
    pub end_row: usize,
    pub end_column: usize,
}

impl From<SourceSpan> for AcquiredSpan {
    fn from(span: SourceSpan) -> Self {
        Self {
            start_byte: span.start_byte,
            end_byte: span.end_byte,
            start_row: span.start.row,
            start_column: span.start.column,
            end_row: span.end.row,
            end_column: span.end.column,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquiredFact {
    pub adapter: String,
    pub kind: String,
    pub path: String,
    pub locator: String,
    pub text: String,
    pub source: SourceId,
    pub revision: Revision,
    pub span: Option<AcquiredSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AcquisitionReport {
    pub capabilities: Vec<AdapterCapability>,
    pub facts: Vec<AcquiredFact>,
    pub diagnostics: Vec<AcquisitionDiagnostic>,
}

#[derive(Debug)]
pub enum AcquisitionError {
    InvalidRoot(PathBuf),
    Io {
        path: String,
        source: std::io::Error,
    },
    AmbiguousAdapter {
        path: String,
        adapters: Vec<String>,
    },
    InvalidJson {
        path: String,
        source: serde_json::Error,
    },
    AdapterFailure {
        adapter: String,
        path: String,
        message: String,
    },
    MalformedSyntax {
        adapter: String,
        path: String,
        diagnostic_count: usize,
    },
    DuplicateAdapterId {
        adapter: String,
    },
}

impl fmt::Display for AcquisitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRoot(path) => write!(
                formatter,
                "source tree is not a readable directory: {}",
                path.display()
            ),
            Self::Io { path, source } => write!(formatter, "cannot acquire {path}: {source}"),
            Self::AmbiguousAdapter { path, adapters } => write!(
                formatter,
                "ambiguous acquisition adapter for {path}: {}",
                adapters.join(", ")
            ),
            Self::InvalidJson { path, source } => {
                write!(formatter, "invalid JSON source {path}: {source}")
            }
            Self::AdapterFailure {
                adapter,
                path,
                message,
            } => write!(
                formatter,
                "{adapter} acquisition failed for {path}: {message}"
            ),
            Self::MalformedSyntax {
                adapter,
                path,
                diagnostic_count,
            } => write!(
                formatter,
                "malformed {adapter} source {path}: {diagnostic_count} parse diagnostics"
            ),
            Self::DuplicateAdapterId { adapter } => {
                write!(
                    formatter,
                    "duplicate acquisition adapter identity: {adapter}"
                )
            }
        }
    }
}

impl Error for AcquisitionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidJson { source, .. } => Some(source),
            Self::InvalidRoot(_)
            | Self::AmbiguousAdapter { .. }
            | Self::AdapterFailure { .. }
            | Self::MalformedSyntax { .. }
            | Self::DuplicateAdapterId { .. } => None,
        }
    }
}

#[derive(Debug)]
struct JsonAdapter;

impl SourceAdapter for JsonAdapter {
    fn capability(&self) -> AdapterCapability {
        AdapterCapability {
            adapter: "json".to_owned(),
            family: AdapterFamily::StructuredSemantic,
            extensions: vec!["json".to_owned()],
        }
    }

    fn acquire(&self, input: AdapterInput<'_>) -> Result<Vec<AcquiredFact>, AdapterError> {
        let mut facts = Vec::new();
        acquire_json(input.bytes, input.path, input.context, &mut facts)
            .map_err(|error| AdapterError::new(error.to_string()))?;
        Ok(facts)
    }
}

#[derive(Debug)]
struct GoAdapter;

impl SourceAdapter for GoAdapter {
    fn capability(&self) -> AdapterCapability {
        AdapterCapability {
            adapter: "go".to_owned(),
            family: AdapterFamily::TreeSitter,
            extensions: vec!["go".to_owned()],
        }
    }

    fn acquire(&self, input: AdapterInput<'_>) -> Result<Vec<AcquiredFact>, AdapterError> {
        let mut facts = Vec::new();
        acquire_go(input.bytes, input.path, input.context, &mut facts)
            .map_err(|error| AdapterError::new(error.to_string()))?;
        Ok(facts)
    }
}

#[derive(Debug)]
struct RustAdapter;

impl SourceAdapter for RustAdapter {
    fn capability(&self) -> AdapterCapability {
        AdapterCapability {
            adapter: "rust".to_owned(),
            family: AdapterFamily::TreeSitter,
            extensions: vec!["rs".to_owned()],
        }
    }

    fn acquire(&self, input: AdapterInput<'_>) -> Result<Vec<AcquiredFact>, AdapterError> {
        let mut facts = Vec::new();
        acquire_rust(input.bytes, input.path, input.context, &mut facts)
            .map_err(|error| AdapterError::new(error.to_string()))?;
        Ok(facts)
    }
}

#[derive(Debug)]
pub struct AcquisitionRuntime {
    adapters: Vec<Box<dyn SourceAdapter>>,
}

impl Default for AcquisitionRuntime {
    fn default() -> Self {
        Self::with_adapters(vec![
            Box::new(GoAdapter),
            Box::new(JsonAdapter),
            Box::new(RustAdapter),
        ])
        .expect("built-in acquisition adapter identities must be unique")
    }
}

impl AcquisitionRuntime {
    pub fn with_adapters(adapters: Vec<Box<dyn SourceAdapter>>) -> Result<Self, AcquisitionError> {
        let mut identities = adapters
            .iter()
            .map(|adapter| adapter.capability().adapter)
            .collect::<Vec<_>>();
        identities.sort();
        for pair in identities.windows(2) {
            if pair[0] == pair[1] {
                return Err(AcquisitionError::DuplicateAdapterId {
                    adapter: pair[0].clone(),
                });
            }
        }
        Ok(Self { adapters })
    }

    #[must_use]
    pub fn capabilities(&self) -> Vec<AdapterCapability> {
        let mut capabilities = self
            .adapters
            .iter()
            .map(|adapter| adapter.capability())
            .collect::<Vec<_>>();
        capabilities.sort_by(|left, right| left.adapter.cmp(&right.adapter));
        capabilities
    }

    pub fn acquire_tree(
        &self,
        root: &Path,
        context: &AcquisitionContext,
    ) -> Result<AcquisitionReport, AcquisitionError> {
        if !root.is_dir() {
            return Err(AcquisitionError::InvalidRoot(root.to_path_buf()));
        }

        let mut files = Vec::new();
        discover(root, root, &mut files)?;
        files.sort();

        let mut report = AcquisitionReport {
            capabilities: self.capabilities(),
            ..AcquisitionReport::default()
        };

        for relative_path in files {
            let extension = relative_path.extension().and_then(|value| value.to_str());
            let matches = self
                .adapters
                .iter()
                .filter(|adapter| {
                    adapter
                        .capability()
                        .extensions
                        .iter()
                        .any(|candidate| Some(candidate.as_str()) == extension)
                })
                .collect::<Vec<_>>();
            let path = normalized_path(&relative_path);

            match matches.as_slice() {
                [] => push_diagnostic(
                    &mut report.diagnostics,
                    AcquisitionDiagnostic {
                        kind: DiagnosticKind::UnsupportedSource,
                        path,
                        message: "no registered acquisition adapter".to_owned(),
                    },
                ),
                [adapter] => {
                    self.acquire_file(adapter.as_ref(), root, &relative_path, context, &mut report)?
                }
                _ => {
                    let mut adapters = matches
                        .iter()
                        .map(|adapter| adapter.capability().adapter)
                        .collect::<Vec<_>>();
                    adapters.sort();
                    return Err(AcquisitionError::AmbiguousAdapter { path, adapters });
                }
            }
        }

        report.facts.sort_by(|left, right| {
            (
                &left.path,
                &left.locator,
                &left.adapter,
                &left.kind,
                &left.text,
            )
                .cmp(&(
                    &right.path,
                    &right.locator,
                    &right.adapter,
                    &right.kind,
                    &right.text,
                ))
        });
        report.diagnostics.sort_by(|left, right| {
            (&left.path, left.kind, &left.message).cmp(&(&right.path, right.kind, &right.message))
        });
        Ok(report)
    }

    fn acquire_file(
        &self,
        adapter: &dyn SourceAdapter,
        root: &Path,
        relative_path: &Path,
        context: &AcquisitionContext,
        report: &mut AcquisitionReport,
    ) -> Result<(), AcquisitionError> {
        let path = normalized_path(relative_path);
        let absolute_path = root.join(relative_path);
        let bytes = fs::read(&absolute_path).map_err(|source| AcquisitionError::Io {
            path: path.clone(),
            source,
        })?;
        let capability = adapter.capability();
        let facts = adapter
            .acquire(AdapterInput {
                bytes: &bytes,
                path: &path,
                context,
            })
            .map_err(|error| AcquisitionError::AdapterFailure {
                adapter: capability.adapter,
                path: path.clone(),
                message: error.to_string(),
            })?;
        report.facts.extend(facts);
        Ok(())
    }
}

fn discover(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> Result<(), AcquisitionError> {
    let mut entries = fs::read_dir(current)
        .map_err(|source| AcquisitionError::Io {
            path: display_relative(root, current),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| AcquisitionError::Io {
            path: display_relative(root, current),
            source,
        })?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let file_type = entry.file_type().map_err(|source| AcquisitionError::Io {
            path: display_relative(root, &entry.path()),
            source,
        })?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if matches!(
                name.as_ref(),
                ".git" | "target" | "node_modules" | ".venv" | "__pycache__"
            ) {
                continue;
            }
            discover(root, &entry.path(), files)?;
        } else if file_type.is_file() {
            let relative = entry
                .path()
                .strip_prefix(root)
                .expect("discovered entry must remain under acquisition root")
                .to_path_buf();
            files.push(relative);
        }
    }
    Ok(())
}

fn acquire_go(
    bytes: &[u8],
    path: &str,
    context: &AcquisitionContext,
    facts: &mut Vec<AcquiredFact>,
) -> Result<(), AcquisitionError> {
    let provenance = SourceProvenance {
        source: context.source.clone(),
        revision: context.revision.clone(),
        locator: path.to_owned(),
        path: path.to_owned(),
    };
    let extraction =
        extract_go_facts(bytes, provenance).map_err(|error| AcquisitionError::AdapterFailure {
            adapter: "Go".to_owned(),
            path: path.to_owned(),
            message: error.to_string(),
        })?;
    if !extraction.diagnostics.is_empty() {
        return Err(AcquisitionError::MalformedSyntax {
            adapter: "Go".to_owned(),
            path: path.to_owned(),
            diagnostic_count: extraction.diagnostics.len(),
        });
    }

    for fact in extraction.facts {
        let span = fact.span;
        facts.push(AcquiredFact {
            adapter: "go".to_owned(),
            kind: go_fact_kind(fact.kind).to_owned(),
            path: fact.provenance.path,
            locator: format!("{}#bytes={}-{}", path, span.start_byte, span.end_byte),
            text: fact.text,
            source: fact.provenance.source,
            revision: fact.provenance.revision,
            span: Some(span.into()),
        });
    }
    Ok(())
}

const fn go_fact_kind(kind: GoFactKind) -> &'static str {
    match kind {
        GoFactKind::Package => "package",
        GoFactKind::Type => "type",
        GoFactKind::Struct => "struct",
        GoFactKind::Interface => "interface",
        GoFactKind::Field => "field",
        GoFactKind::Tag => "tag",
        GoFactKind::Const => "const",
        GoFactKind::Var => "var",
        GoFactKind::Function => "function",
        GoFactKind::Method => "method",
        GoFactKind::Receiver => "receiver",
        GoFactKind::Parameter => "parameter",
        GoFactKind::TypeExpression => "type_expression",
        GoFactKind::Call => "call",
        GoFactKind::If => "if",
        GoFactKind::Switch => "switch",
        GoFactKind::For => "for",
        GoFactKind::Return => "return",
        GoFactKind::Panic => "panic",
        GoFactKind::Comment => "comment",
        GoFactKind::Assertion => "assertion",
    }
}

fn acquire_rust(
    bytes: &[u8],
    path: &str,
    context: &AcquisitionContext,
    facts: &mut Vec<AcquiredFact>,
) -> Result<(), AcquisitionError> {
    let provenance = SourceProvenance {
        source: context.source.clone(),
        revision: context.revision.clone(),
        locator: path.to_owned(),
        path: path.to_owned(),
    };
    let extraction = extract_rust_facts(bytes, provenance).map_err(|error| {
        AcquisitionError::AdapterFailure {
            adapter: "Rust".to_owned(),
            path: path.to_owned(),
            message: error.to_string(),
        }
    })?;
    if !extraction.diagnostics.is_empty() {
        return Err(AcquisitionError::MalformedSyntax {
            adapter: "Rust".to_owned(),
            path: path.to_owned(),
            diagnostic_count: extraction.diagnostics.len(),
        });
    }

    for fact in extraction.facts {
        let span = fact.span;
        facts.push(AcquiredFact {
            adapter: "rust".to_owned(),
            kind: rust_fact_kind(fact.kind).to_owned(),
            path: fact.provenance.path,
            locator: format!("{}#bytes={}-{}", path, span.start_byte, span.end_byte),
            text: fact.text,
            source: fact.provenance.source,
            revision: fact.provenance.revision,
            span: Some(span.into()),
        });
    }
    Ok(())
}

const fn rust_fact_kind(kind: RustFactKind) -> &'static str {
    match kind {
        RustFactKind::Module => "module",
        RustFactKind::Struct => "struct",
        RustFactKind::Enum => "enum",
        RustFactKind::Variant => "variant",
        RustFactKind::Trait => "trait",
        RustFactKind::Impl => "impl",
        RustFactKind::Function => "function",
        RustFactKind::Method => "method",
        RustFactKind::Field => "field",
        RustFactKind::Const => "const",
        RustFactKind::Static => "static",
        RustFactKind::TypeExpression => "type_expression",
        RustFactKind::Attribute => "attribute",
        RustFactKind::Call => "call",
        RustFactKind::MacroCall => "macro_call",
        RustFactKind::If => "if",
        RustFactKind::Match => "match",
        RustFactKind::MatchArm => "match_arm",
        RustFactKind::Return => "return",
        RustFactKind::Comment => "comment",
        RustFactKind::Assertion => "assertion",
    }
}

fn acquire_json(
    bytes: &[u8],
    path: &str,
    context: &AcquisitionContext,
    facts: &mut Vec<AcquiredFact>,
) -> Result<(), AcquisitionError> {
    let compatible = strip_json_comments(bytes);
    let value: Value =
        serde_json::from_slice(&compatible).map_err(|source| AcquisitionError::InvalidJson {
            path: path.to_owned(),
            source,
        })?;
    walk_json(&value, "", path, context, facts);
    Ok(())
}

fn strip_json_comments(bytes: &[u8]) -> Vec<u8> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Normal,
        String,
        LineComment,
        BlockComment,
    }

    let mut output = bytes.to_vec();
    let mut state = State::Normal;
    let mut index = 0;
    while index < bytes.len() {
        match state {
            State::Normal => match bytes[index] {
                b'\"' => {
                    state = State::String;
                    index += 1;
                }
                b'/' if bytes.get(index + 1) == Some(&b'/') => {
                    output[index] = b' ';
                    output[index + 1] = b' ';
                    state = State::LineComment;
                    index += 2;
                }
                b'/' if bytes.get(index + 1) == Some(&b'*') => {
                    output[index] = b' ';
                    output[index + 1] = b' ';
                    state = State::BlockComment;
                    index += 2;
                }
                _ => index += 1,
            },
            State::String => match bytes[index] {
                b'\\' => {
                    index += 1;
                    if index < bytes.len() {
                        index += 1;
                    }
                }
                b'\"' => {
                    state = State::Normal;
                    index += 1;
                }
                _ => index += 1,
            },
            State::LineComment => {
                if matches!(bytes[index], b'\n' | b'\r') {
                    state = State::Normal;
                } else {
                    output[index] = b' ';
                }
                index += 1;
            }
            State::BlockComment => {
                if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    output[index] = b' ';
                    output[index + 1] = b' ';
                    state = State::Normal;
                    index += 2;
                } else {
                    if !matches!(bytes[index], b'\n' | b'\r') {
                        output[index] = b' ';
                    }
                    index += 1;
                }
            }
        }
    }
    output
}

fn walk_json(
    value: &Value,
    pointer: &str,
    path: &str,
    context: &AcquisitionContext,
    facts: &mut Vec<AcquiredFact>,
) {
    let (kind, text) = match value {
        Value::Null => ("null", "null".to_owned()),
        Value::Bool(value) => ("boolean", value.to_string()),
        Value::Number(value) => ("number", value.to_string()),
        Value::String(value) => ("string", value.clone()),
        Value::Array(values) => ("array", format!("{} items", values.len())),
        Value::Object(values) => ("object", format!("{} properties", values.len())),
    };
    facts.push(AcquiredFact {
        adapter: "json".to_owned(),
        kind: kind.to_owned(),
        path: path.to_owned(),
        locator: if pointer.is_empty() {
            "#".to_owned()
        } else {
            format!("#{pointer}")
        },
        text,
        source: context.source.clone(),
        revision: context.revision.clone(),
        span: None,
    });

    match value {
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                walk_json(value, &format!("{pointer}/{index}"), path, context, facts);
            }
        }
        Value::Object(values) => {
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort();
            for key in keys {
                let escaped = key.replace('~', "~0").replace('/', "~1");
                walk_json(
                    &values[key],
                    &format!("{pointer}/{escaped}"),
                    path,
                    context,
                    facts,
                );
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn push_diagnostic(
    diagnostics: &mut Vec<AcquisitionDiagnostic>,
    diagnostic: AcquisitionDiagnostic,
) {
    if diagnostics.len() < MAX_DIAGNOSTICS {
        diagnostics.push(diagnostic);
    } else if diagnostics.len() == MAX_DIAGNOSTICS {
        diagnostics.push(AcquisitionDiagnostic {
            kind: DiagnosticKind::DiagnosticsTruncated,
            path: String::new(),
            message: format!("diagnostics truncated after {MAX_DIAGNOSTICS} entries"),
        });
    }
}

fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(normalized_path)
        .unwrap_or_else(|_| normalized_path(path))
}

fn normalized_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
