//! βάθρον (bathron, pedestal/base) — OS-service integration.
//!
//! Notifications, file dialogs, settings persistence, and logging.
//! Tray and hotkeys live in [`mekhane`] (the windowing layer); bathron
//! provides the higher-level OS services that don't depend on the event
//! loop.
//!
//! ## Feature gates
//!
//! Each service module is gated behind a cargo feature so consumers
//! only pull the OS-integration deps they actually use:
//!
//! - `notifications` — desktop-notification dispatch via `notify-rust`.
//! - `dialogs` — file open / save dialogs via `rfd`.
//! - `atomic-write` — durable whole-file replacement ([`atomic`]).
//! - `settings` — TOML-backed operator-tier KV store, built on `atomic-write`.
//! - `logging` — `tracing-subscriber` init with daily-rotated file
//!   appender via `tracing-appender`.
//!
//! [`version`] is unconditional.
//!
//! See the [theatron repository](https://forge.forkwright.com/forkwright/theatron)
//! for the broader roadmap.
//!
//! [`mekhane`]: https://forge.forkwright.com/forkwright/theatron/src/branch/main/crates/mekhane

#![deny(missing_docs, clippy::all, clippy::pedantic)]

#[cfg(feature = "notifications")]
pub mod notifications;

#[cfg(feature = "dialogs")]
pub mod dialogs;

#[cfg(feature = "atomic-write")]
pub mod atomic;

#[cfg(feature = "settings")]
pub mod settings;

#[cfg(feature = "logging")]
pub mod logging;

/// Returns the crate version string from Cargo metadata.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_cargo_metadata() {
        let v = version();
        assert!(!v.is_empty(), "version() must return a non-empty string");
        assert!(
            v.contains('.'),
            "version() should be semver-shaped, got {v}"
        );
    }
}
