#![forbid(unsafe_code)]

//! Language-agnostic core for Chirograph.

pub mod evidence;
pub mod graph_json;
pub mod model;

/// Returns the Chirograph core package version.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn exposes_package_version() {
        assert!(!version().is_empty());
    }
}
