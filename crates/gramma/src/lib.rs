//! γράμμα (gramma, written character) — markdown + syntax highlighting + diff.
//!
//! pulldown-cmark + syntect for native rendering, plus structured
//! unified-diff parsing. Sandbox-safe HTML output.
//!
//! ## Modules
//!
//! - [`highlight`] — source-code syntax highlighting via syntect.
//!   Returns line-by-line styled spans (no Dioxus dependency). The
//!   Dioxus component that renders these spans lives in
//!   `skeue::CodeBlock`.
//! - [`diff`] — unified-diff parsing and structured representation
//!   ([`DiffFile`], [`DiffHunk`], [`DiffLine`], [`ChangeType`],
//!   [`WordSpan`], [`DiffViewMode`], plus side-by-side alignment +
//!   word-level diffing). Pure logic; the Dioxus components that render
//!   diffs live in `skeue::{diff_hunk, diff_line}`.
//! - [`syntax`] — file-path-to-syntect-language resolution
//!   ([`language_from_path`](syntax::language_from_path),
//!   [`language_from_extension`](syntax::language_from_extension))
//!   for file viewers + diff views that need a syntect token from a
//!   path without re-implementing the extension table.

#![deny(missing_docs, clippy::all, clippy::pedantic)]

pub mod diff;
pub mod highlight;
pub mod syntax;

pub use diff::{
    ChangeType, DiffFile, DiffHunk, DiffLine, DiffViewMode, SideBySideRow, WordSpan,
    align_side_by_side, parse_git_diff, parse_unified_diff,
};
pub use highlight::{HighlightedSpan, detect_language, highlight_code};

#[cfg(test)]
mod smoke_tests {
    /// Smoke test: crate compiles and the test module runs.
    #[test]
    fn crate_smoke() {
        assert_eq!(2 + 2, 4);
    }
}
