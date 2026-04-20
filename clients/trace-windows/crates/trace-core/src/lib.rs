//! Cross-platform domain logic for the Trace Windows client.
//!
//! This crate intentionally avoids any Windows-specific or UI dependencies so
//! its tests can run on Linux, macOS and Windows alike. Platform integration
//! lives in `trace-platform`; the iced UI layer lives in `trace-ui`.

/// Crate version, wired up here so downstream crates can display it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_cargo_manifest() {
        assert!(
            !VERSION.is_empty(),
            "CARGO_PKG_VERSION should be populated at compile time"
        );
    }
}
