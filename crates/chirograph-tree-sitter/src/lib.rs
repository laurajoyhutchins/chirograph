#![forbid(unsafe_code)]

//! Shared read-only Tree-sitter acquisition substrate for Chirograph.

mod parse;
mod provenance;

pub use parse::{ParseDiagnostic, ParseDiagnosticKind, ParseError, ParsedSource, parse_utf8};
pub use provenance::{SourcePoint, SourceProvenance, SourceSpan};
