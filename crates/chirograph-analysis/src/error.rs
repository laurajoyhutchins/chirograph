use std::fmt;

#[derive(Debug)]
pub enum AnalysisError {
    InvalidRepository(String),
    InvalidSourceRoot(String),
    Io(std::io::Error),
    InvalidSemanticPath(String),
    InvalidCandidate(String),
    InvalidSchema(String),
    InvalidRustProjection(String),
    InvalidAlignment(String),
    Graph(String),
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRepository(value) => {
                write!(formatter, "invalid repository identity: {value}")
            }
            Self::InvalidSourceRoot(value) => write!(formatter, "invalid source root: {value}"),
            Self::Io(error) => write!(formatter, "filesystem analysis failed: {error}"),
            Self::InvalidSemanticPath(value) => write!(formatter, "invalid semantic path: {value}"),
            Self::InvalidCandidate(value) => {
                write!(formatter, "invalid representation candidate: {value}")
            }
            Self::InvalidSchema(value) => {
                write!(formatter, "invalid JSON Schema evidence: {value}")
            }
            Self::InvalidRustProjection(value) => {
                write!(formatter, "invalid Rust projection: {value}")
            }
            Self::InvalidAlignment(value) => write!(formatter, "invalid alignment: {value}"),
            Self::Graph(value) => write!(formatter, "invalid contract graph: {value}"),
        }
    }
}

impl std::error::Error for AnalysisError {}

impl From<std::io::Error> for AnalysisError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
