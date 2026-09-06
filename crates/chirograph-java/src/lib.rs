#![forbid(unsafe_code)]

//! Generic Java source-fact acquisition for Chirograph.

mod extract;
mod fact;

pub use extract::{JavaAdapterError, extract_java_facts};
pub use fact::{JavaExtraction, JavaFact, JavaFactKind};

/// Semantics intentionally outside this source-local adapter.
pub const UNSUPPORTED_SEMANTICS: &[&str] = &[
    "whole-program symbol resolution",
    "classpath or build execution",
    "annotation processor execution",
    "runtime or reflection semantics",
];