//! θεμέλιον (themelion, foundation) — theme provider and OS preference
//! detection for Dioxus + Blitz fleet desktop apps.
//!
//! Consumers (chalkeion, proskenion, harmonia-desktop, akroasis-desktop)
//! take dependencies on themelion for the application shell.
//!
//! ## Modules
//!
//! - [`theme`] — `ThemeMode` enum (Dark/Light/System), `ThemeProvider`
//!   component with `data-theme` attribute binding, OS preference
//!   detection (`GTK_THEME` + `COLORFGBG` heuristics).

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

pub mod theme;

pub use theme::{ResolvedTheme, ThemeMode, ThemeProvider, ThemeToggle};

/// Crate version for telemetry / version-gate use.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod smoke_tests {
    /// `version()` returns the compiled Cargo package version:
    /// non-empty and semver-shaped.
    #[test]
    fn version_is_semver_like() {
        let v = super::version();
        assert!(!v.is_empty(), "version() must return a non-empty string");
        assert!(v.contains('.'), "version {v:?} should be semver-like");
    }
}
