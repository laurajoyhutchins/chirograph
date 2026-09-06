#![forbid(unsafe_code)]

//! Generic Go source-fact acquisition for Chirograph.

mod extract;
mod fact;

pub use extract::{GoAdapterError, extract_go_facts};
pub use fact::{GoExtraction, GoFact, GoFactKind};
