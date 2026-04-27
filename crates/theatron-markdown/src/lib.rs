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
//! - [`diff`] — unified-diff parsing and structured representation
//!   ([`DiffFile`], [`DiffHunk`], [`DiffLine`], [`ChangeType`],
//!   [`WordSpan`], [`DiffViewMode`], plus side-by-side alignment +
//!   word-level diffing). Pure logic; the Dioxus components that render
//!   diffs live in `theatron_components::{diff_hunk, diff_line}`.

#![warn(missing_docs, clippy::all, clippy::pedantic)]

pub mod diff;
pub mod highlight;

pub use diff::{
    ChangeType, DiffFile, DiffHunk, DiffLine, DiffViewMode, SideBySideRow, WordSpan,
    align_side_by_side, parse_unified_diff,
};
pub use highlight::{HighlightedSpan, detect_language, highlight_code};
