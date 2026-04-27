//! theatron-net — HTTP client base, SSE streaming, mDNS discovery for
//! Dioxus + Blitz fleet desktop apps.
//!
//! Phase 1+2 deliverable. See `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md`
//! for the broader plan.
//!
//! ## Modules
//!
//! - [`sse`] — owned SSE wire-protocol parser. Wraps any
//!   `Stream<Item = Result<Bytes, _>>` and yields parsed
//!   [`SseEvent`](sse::SseEvent)s. Used by chalkeion + future fleet
//!   desktop surfaces to consume kanon-server SSE feeds without
//!   parser duplication.
//! - [`error`] — generic API-layer error type ([`ApiError`]) covering
//!   transport, non-2xx, auth, and invalid-token failure modes for
//!   any fleet HTTP client built on theatron-net.

#![warn(missing_docs, clippy::all, clippy::pedantic)]

pub mod error;
pub mod sse;

pub use error::{ApiError, Result};
pub use sse::{SseEvent, SseStream};
