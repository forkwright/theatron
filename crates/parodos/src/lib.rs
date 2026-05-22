//! πάροδος (parodos, chorus's stage entrance) -- terminal UI substrate.
//!
//! Ratatui shared primitives + Elm state/update/view dispatcher.
//! Extracted from aletheia/koilon during Phase 1+2 of the chalkeion
//! plan. See `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md`
//! for the broader plan.
//!
//! ## Modules
//!
//! - [`mod@env`] -- minimal environment-variable abstraction. Trait
//!   [`Env`] + production [`RealEnv`] impl,
//!   inlined here so parodos doesn't depend on aletheia's
//!   `koina::system::Environment`.
//! - [`fuzzy`] -- subsequence fuzzy matcher for command palette / slash
//!   completion. Pure-logic, no external state.
//! - [`theme`] -- terminal palette + color-depth detection. Provides
//!   [`Theme`](theme::Theme), [`ThemeMode`],
//!   [`ColorDepth`], and detection helpers that
//!   read terminal capability env vars (COLORTERM, TERM, COLORFGBG).
//! - [`highlight`] -- syntect-backed code-block syntax highlighting
//!   that returns ratatui `Line`s tinted to the active [`ThemeMode`].
//! - [`sanitize`] -- strip dangerous escape sequences (CSI/OSC/DCS/
//!   APC/SOS/PM) and replace C0/C1 control bytes with safe alternates
//!   for terminal display of untrusted text.
//! - [`clipboard`] -- read/write the system clipboard via arboard with
//!   OSC52 escape-sequence fallback for headless / SSH / tmux contexts.
//! - [`hyperlink`] -- OSC 8 hyperlink emission, terminal capability
//!   detection, URL + file-path detection regexes.
//! - [`layout`] -- shared ratatui layout helpers for common terminal
//!   view geometry.
//! - [`text`] -- Unicode-safe text truncation helpers for terminal
//!   display.
//! - [`widgets`] -- small string helpers for terminal widget assembly.

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

pub mod clipboard;
pub mod env;
pub mod fuzzy;
pub mod highlight;
pub mod hyperlink;
pub mod layout;
pub mod sanitize;
pub mod text;
pub mod theme;
pub mod widgets;

pub use env::{Env, RealEnv};
pub use fuzzy::{MatchResult, fuzzy_match};
pub use theme::{ColorDepth, ThemeMode};

/// Returns the parodos crate version. Filled in iteratively as the
/// koilon extraction progresses.
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
