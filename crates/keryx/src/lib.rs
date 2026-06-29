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
//! - [`response`] — response classification ([`ensure_success`](response::ensure_success))
//!   and decode helpers ([`decode_json`](response::decode_json)) that
//!   make the v1.1 [`ApiError`] variants reachable from `reqwest::Response`
//!   without per-consumer status-table boilerplate.
//! - [`url`] — URL helpers ([`encode_path_segment`](url::encode_path_segment))
//!   for endpoint construction. RFC 3986 unreserved-character
//!   passthrough, `%XX` uppercase-hex for everything else.

#![deny(missing_docs, clippy::all, clippy::pedantic)]

pub mod error;
pub mod response;
pub mod sse;
pub mod url;

pub use error::{ApiError, Result};
pub use sse::{SseError, SseEvent, SseStream};

#[cfg(test)]
mod smoke_tests {
    /// Smoke test: crate compiles and the test module runs.
    #[test]
    fn crate_smoke() {
        assert_eq!(2 + 2, 4);
    }
}
