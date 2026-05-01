//! θεμέλιον (themelion, foundation) — theme provider, window lifecycle,
//! routing scaffolding, error boundary, settings persistence, logging
//! setup for any Dioxus + Blitz fleet desktop app.
//!
//! Seed crate for the theatron repo. Consumers (chalkeion, proskenion,
//! harmonia-desktop, akroasis-desktop) take dependencies on themelion
//! for the application shell.
//!
//! See `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md` for
//! the broader plan.
//!
//! ## Modules seeded
//!
//! - [`theme`] — `ThemeMode` enum (Dark/Light/System), `ThemeProvider`
//!   component with `data-theme` attribute binding, OS preference
//!   detection (`GTK_THEME` + `COLORFGBG` heuristics). Extracted verbatim
//!   from `aletheia/proskenion/src/theme.rs`.

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
    /// Smoke test: crate compiles and the test module runs.
    #[test]
    fn crate_smoke() {
        assert_eq!(2 + 2, 4);
    }
}
