//! κῆρυξ (keryx, herald/messenger) — HTTP client base, SSE streaming,
//! mDNS discovery for Dioxus + Blitz fleet desktop apps.
//!
//! ## Modules
//!
//! - [`sse`] — owned SSE wire-protocol parser. Wraps any
//!   `Stream<Item = Result<Bytes, _>>` and yields parsed
//!   [`SseEvent`]s. Used by chalkeion + future fleet
//!   desktop surfaces to consume kanon-server SSE feeds without
//!   parser duplication.
//! - [`error`] — generic API-layer error type ([`ApiError`]) covering
//!   transport, non-2xx, auth, and invalid-token failure modes for
//!   any fleet HTTP client built on keryx.

#![deny(missing_docs, clippy::all, clippy::pedantic)]

pub mod error;
pub mod sse;

pub use error::{ApiError, Result};
pub use sse::{SseEvent, SseStream};

#[cfg(test)]
mod smoke_tests {
    /// Smoke test: crate compiles and the test module runs.
    #[test]
    fn crate_smoke() {
        assert_eq!(2 + 2, 4);
    }
}
