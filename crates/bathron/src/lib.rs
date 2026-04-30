//! βάθρον (bathron, pedestal/base) — OS-service integration.
//!
//! Notifications, file dialogs, window state persistence, settings,
//! autoupdate, logging. Tray and hotkeys live in [`mekhane`] (the
//! windowing layer); bathron provides the higher-level OS services
//! that don't depend on the event loop.
//!
//! Phase 1+2 skeleton. See
//! `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md` for the
//! broader plan.

#![warn(missing_docs, clippy::all, clippy::pedantic)]

/// Returns the bathron crate version. Filled in iteratively as the
/// platform-services extraction progresses.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
