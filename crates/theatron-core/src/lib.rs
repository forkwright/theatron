//! theatron-core — window lifecycle, theme provider, routing scaffolding,
//! error boundary, settings persistence, logging setup for any Dioxus +
//! Blitz fleet desktop app.
//!
//! Phase 1+2 deliverable. This is the seed crate for the theatron repo;
//! consumers (chalkeion, proskenion-refactored, harmonia-desktop,
//! akroasis-desktop) take dependencies on theatron-core for the
//! application shell.
//!
//! See `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md` for
//! the broader plan.
//!
//! ## Modules seeded
//!
//! - [`theme`] — `ThemeMode` enum (Dark/Light/System), `ThemeProvider`
//!   component with `data-theme` attribute binding, OS preference
//!   detection (GTK_THEME + COLORFGBG heuristics). Extracted verbatim
//!   from aletheia/proskenion/src/theme.rs.

#![warn(clippy::all, clippy::pedantic)]

pub mod theme;

pub use theme::{ResolvedTheme, ThemeMode, ThemeProvider};

/// Crate version for telemetry / version-gate use.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
