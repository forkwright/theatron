//! θεμέλιον (themelion, foundation) — theme provider and OS preference
//! detection for Dioxus + Blitz fleet desktop apps.
//!
//! Consumers (chalkeion, proskenion, harmonia-desktop, akroasis-desktop)
//! take dependencies on themelion for the application shell.
//!
//! ## Modules
//!
//! - [`theme`] — canonical fleet theme vocabulary: `ThemeMode`
//!   (Dark/Light/System preference), `ResolvedTheme` (concrete
//!   brightness), OS preference detection (`GTK_THEME` + `COLORFGBG`
//!   heuristics).
//! - `provider` — `ThemeProvider` component with `data-theme`
//!   attribute binding + `ThemeToggle`, behind the `dioxus` feature
//!   (default).
//!
//! ## Features
//!
//! - `dioxus` (default) — `ThemeProvider` / `ThemeToggle` components.
//!   Non-GUI consumers (`parodos`) disable default features to take
//!   only the theme vocabulary.

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

#[cfg(feature = "dioxus")]
pub mod provider;
pub mod theme;

#[cfg(feature = "dioxus")]
pub use provider::{ThemeProvider, ThemeToggle};
pub use theme::{ResolvedTheme, ThemeMode};

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
