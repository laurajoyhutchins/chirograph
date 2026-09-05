#![forbid(unsafe_code)]

//! Deterministic production analysis between source acquisition and Chirograph graphs.

mod candidate;
mod context;
mod discovery;
mod error;

pub use candidate::{CandidateEvidence, CandidateMechanism, RepresentationCandidate, SemanticPath};
pub use context::AnalysisSourceContext;
pub use discovery::{DiscoveredSource, SourceFileKind, discover_sources};
pub use error::AnalysisError;
