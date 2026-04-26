//! theatron-net — HTTP client base, SSE/streaming via reqwest+tokio+eventsource-stream pattern, mDNS discovery
//!
//! Phase 1+2 deliverable. See `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md`
//! for the broader plan.

#![warn(missing_docs, clippy::all, clippy::pedantic)]

/// Placeholder. Phase 1+2 work fills this in iteratively against
/// proskenion refactor.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
