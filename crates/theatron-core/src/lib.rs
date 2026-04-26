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

#![warn(missing_docs, clippy::all, clippy::pedantic)]

/// Placeholder. Phase 1+2 work fills this in iteratively against
/// proskenion refactor.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
