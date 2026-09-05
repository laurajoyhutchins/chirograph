#![forbid(unsafe_code)]

//! Deterministic production analysis between source acquisition and Chirograph graphs.

mod alignment;
mod analyze;
mod assembly;
mod candidate;
mod context;
mod discovery;
mod error;
mod json_schema;
mod rust_projection;

pub use alignment::{AlignmentDecision, CandidateKey, align_candidates};
pub use analyze::analyze_tree;
pub use assembly::{AnalysisAssembly, assemble_contract_graph};
pub use candidate::{CandidateEvidence, CandidateMechanism, RepresentationCandidate, SemanticPath};
pub use context::AnalysisSourceContext;
pub use discovery::{DiscoveredSource, SourceFileKind, discover_sources};
pub use error::AnalysisError;
pub use json_schema::extract_json_schema_candidates;
pub use rust_projection::extract_rust_candidates;
