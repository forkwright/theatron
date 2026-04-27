//! πάροδος (parodos, chorus's stage entrance) — terminal UI substrate.
//!
//! Ratatui shared primitives + Elm state/update/view dispatcher.
//! Extracted from aletheia/koilon during Phase 1+2 of the chalkeion
//! plan. See `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md`
//! for the broader plan.

#![warn(missing_docs, clippy::all, clippy::pedantic)]

/// Returns the parodos crate version. Filled in iteratively as the
/// koilon extraction progresses.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
