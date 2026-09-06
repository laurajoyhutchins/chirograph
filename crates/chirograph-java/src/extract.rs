use std::{error::Error, fmt};

use chirograph_tree_sitter::{ParseError, SourceProvenance, parse_utf8};

use crate::fact::JavaExtraction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaAdapterError {
    Parse(ParseError),
}

impl fmt::Display for JavaAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "cannot parse Java source: {error}"),
        }
    }
}

impl Error for JavaAdapterError {}

impl From<ParseError> for JavaAdapterError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

pub fn extract_java_facts(
    source: &[u8],
    provenance: SourceProvenance,
) -> Result<JavaExtraction, JavaAdapterError> {
    let language = tree_sitter_java::LANGUAGE.into();
    let parsed = parse_utf8(&language, source, provenance)?;

    Ok(JavaExtraction {
        facts: Vec::new(),
        diagnostics: parsed.diagnostics().to_vec(),
    })
}
