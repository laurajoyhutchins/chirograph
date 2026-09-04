#![forbid(unsafe_code)]

//! Language-agnostic core for Chirograph.

pub mod alignment;
pub mod evidence;
pub mod model;
pub mod query;

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
