//! πάροδος (parodos, chorus's stage entrance) — terminal UI substrate.
//!
//! Ratatui shared primitives + Elm state/update/view dispatcher.
//! Extracted from aletheia/koilon during Phase 1+2 of the chalkeion
//! plan. See `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md`
//! for the broader plan.
//!
//! ## Modules seeded
//!
//! - [`fuzzy`] — subsequence fuzzy matcher for command palette / slash
//!   completion. First beat of the koilon → parodos extraction wave
//!   (kanon Task #82). Pure-logic, no external state.

#![warn(missing_docs, clippy::all, clippy::pedantic)]

pub mod fuzzy;

pub use fuzzy::{MatchResult, fuzzy_match};

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
