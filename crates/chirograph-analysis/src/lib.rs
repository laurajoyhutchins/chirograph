#![forbid(unsafe_code)]

//! Deterministic production analysis between source acquisition and Chirograph graphs.

mod candidate;
mod context;
mod discovery;
mod error;
mod json_schema;

pub use candidate::{CandidateEvidence, CandidateMechanism, RepresentationCandidate, SemanticPath};
pub use context::AnalysisSourceContext;
pub use discovery::{DiscoveredSource, SourceFileKind, discover_sources};
pub use error::AnalysisError;
pub use json_schema::extract_json_schema_candidates;
