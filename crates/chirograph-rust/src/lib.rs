#![forbid(unsafe_code)]

//! Generic Rust source-fact acquisition for Chirograph.

mod extract;
mod fact;

pub use extract::{RustAdapterError, extract_rust_facts};
pub use fact::{RustExtraction, RustFact, RustFactKind};
