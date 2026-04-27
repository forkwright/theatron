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
//!   [`SseEvent`](sse::SseEvent)s. Extracted from aletheia/skene
//!   (W4). Used by chalkeion + future fleet desktop surfaces to
//!   consume kanon-server SSE feeds without parser duplication.

#![warn(missing_docs, clippy::all, clippy::pedantic)]

pub mod sse;

pub use sse::{SseEvent, SseStream};
