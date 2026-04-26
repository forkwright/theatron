//! theatron-lint — Design-token enforcement linter — parses CSS + Rust component code; fails on undocumented tokens. Folds in #41.
//!
//! Phase 1+2 deliverable. See `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md`
//! for the broader plan.

#![warn(missing_docs, clippy::all, clippy::pedantic)]

/// Placeholder. Phase 1+2 work fills this in iteratively against
/// proskenion refactor.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
