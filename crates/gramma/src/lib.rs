//! γράμμα (gramma, written character) — syntax highlighting + diff data.
//!
//! syntect-backed highlighting spans plus structured unified-diff
//! parsing. Pure data structures — no HTML output; rendering lives in
//! `skeue` components that consume these types.
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
    /// Round-trip smoke: the public parse API turns a minimal unified
    /// diff into structured hunks with correct content and stats.
    #[test]
    fn crate_smoke() {
        let diff = crate::parse_unified_diff("smoke.rs", "@@ -1,2 +1,2 @@\n a\n-old\n+new\n");
        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.additions, 1);
        assert_eq!(diff.deletions, 1);
        let lines = diff
            .hunks
            .first()
            .map(|hunk| hunk.lines.as_slice())
            .unwrap_or_default();
        assert_eq!(lines.len(), 3);
        assert_eq!(
            lines.get(1).map(|l| (l.change_type, l.content.as_str())),
            Some((crate::ChangeType::Remove, "old"))
        );
        assert_eq!(
            lines.get(2).map(|l| (l.change_type, l.content.as_str())),
            Some((crate::ChangeType::Add, "new"))
        );
    }
}
