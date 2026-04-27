//! theatron-markdown — pulldown-cmark + syntect for native rendering.
//!
//! Phase 1+2 deliverable. See `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md`
//! for the broader plan.
//!
//! ## Modules
//!
//! - [`highlight`] — source-code syntax highlighting via syntect.
//!   Returns line-by-line styled spans (no Dioxus dependency). The
//!   Dioxus component that renders these spans lives in
//!   `theatron_components::CodeBlock`.

#![warn(missing_docs, clippy::all, clippy::pedantic)]

pub mod highlight;

pub use highlight::{HighlightedSpan, detect_language, highlight_code};
