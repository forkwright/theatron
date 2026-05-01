//! CSS source scanner.
//!
//! Extracts every `var(--token)` reference from a CSS source string and
//! produces a [`Diagnostic`] for any token that is not in the supplied
//! [`TokenRegistry`].
//!
//! ## Why we mask before regex-scanning
//!
//! A naive regex over raw CSS source flags `var(--token)` references
//! that appear inside CSS strings (`content: "var(--missing)"`) or
//! comments (`/* var(--missing) */`) — false positives. It also misses
//! `var(--token /* note */)` because the regex requires `\s*[,)]`
//! immediately after the token name — false negative.
//!
//! [`mask_strings_and_comments`] replaces the bytes inside CSS strings
//! and comments with whitespace (preserving newlines for line/col
//! accuracy), and the regex then runs over the masked source. Positions
//! reported in diagnostics still index into the original source because
//! masking preserves byte offsets exactly.

use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;

use crate::diagnostic::Diagnostic;
use crate::tokens::TokenRegistry;

/// Match `var(--token)` references with optional whitespace and fallback
/// value (`var(--foo, fallback)`). Capture group 1 is the token name.
#[expect(
    clippy::expect_used,
    reason = "hardcoded regex compilation; failure is a programming error"
)]
fn var_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"var\(\s*(--[a-z][a-z0-9-]*)\s*[,)]").expect("var regex compiles"))
}

/// Lint a CSS source string, returning one diagnostic per undocumented
/// token reference.
#[expect(
    clippy::expect_used,
    reason = "regex capture group presence is guaranteed by the hardcoded pattern"
)]
pub(crate) fn lint_css(registry: &TokenRegistry, source: &str, path: &Path) -> Vec<Diagnostic> {
    let masked = mask_strings_and_comments(source);
    let line_starts = build_line_index(source);
    let mut diagnostics = Vec::new();
    for caps in var_regex().captures_iter(&masked) {
        let m = caps.get(1).expect("capture group 1 is present in regex");
        let token = m.as_str();
        if registry.contains(token) {
            continue;
        }
        let (line, column) = locate(&line_starts, m.start());
        diagnostics.push(Diagnostic::undocumented_token(
            path.to_path_buf(),
            line,
            column,
            m.start(),
            m.len(),
            token.to_string(),
        ));
    }
    diagnostics
}

/// Replace bytes inside CSS strings (`"…"`, `'…'`) and CSS block
/// comments (`/* … */`) with spaces, preserving newlines and the source
/// length. Used to suppress false positives and avoid mid-token comment
/// false negatives in [`lint_css`].
///
/// Used by `rust.rs` too, because string-literal *contents* in Rust may
/// embed CSS strings that should not be scanned (e.g.
/// `style: "background: \"var(--ok)\""`).
#[expect(
    clippy::expect_used,
    reason = "masker only emits ASCII space/newline bytes; UTF-8 is guaranteed by construction"
)]
#[expect(
    clippy::indexing_slicing,
    reason = "indices are guarded by explicit bounds checks in the byte scanner"
)]
pub(crate) fn mask_strings_and_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        // CSS block comment: /* … */
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            out.push(b' ');
            out.push(b' ');
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                out.push(if bytes[i] == b'\n' { b'\n' } else { b' ' });
                i += 1;
            }
            // Consume closing */ if present.
            if i + 1 < bytes.len() {
                out.push(b' ');
                out.push(b' ');
                i += 2;
            } else {
                // Unterminated comment — mask the rest.
                while i < bytes.len() {
                    out.push(if bytes[i] == b'\n' { b'\n' } else { b' ' }); // kanon:ignore RUST/indexing-slicing -- bounded by i < bytes.len()
                    i += 1;
                }
            }
        // Quoted string: "…" or '…' with backslash escapes.
        } else if bytes[i] == b'"' || bytes[i] == b'\'' {
            // kanon:ignore RUST/indexing-slicing -- bounded by outer i < bytes.len()
            let quote = bytes[i];
            out.push(b' ');
            i += 1;
            while i < bytes.len() && bytes[i] != quote {
                // kanon:ignore RUST/indexing-slicing -- bounded by short-circuit i < bytes.len()
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    // kanon:ignore RUST/indexing-slicing -- bounded by outer while
                    // Mask backslash + next byte (handles \", \\, \n, etc.).
                    out.push(b' ');
                    out.push(if bytes[i + 1] == b'\n' { b'\n' } else { b' ' });
                    i += 2;
                } else {
                    out.push(if bytes[i] == b'\n' { b'\n' } else { b' ' }); // kanon:ignore RUST/indexing-slicing -- bounded by outer while
                    i += 1;
                }
            }
            // Consume closing quote if present (or run off end).
            if i < bytes.len() {
                out.push(b' ');
                i += 1;
            }
        } else {
            out.push(bytes[i]); // kanon:ignore RUST/indexing-slicing -- bounded by outer while i < bytes.len()
            i += 1;
        }
    }
    debug_assert_eq!(out.len(), bytes.len(), "mask must preserve source length");
    String::from_utf8(out).expect("mask only emits ASCII bytes (space/newline) in masked regions") // kanon:ignore RUST/expect -- mask only emits ASCII bytes; UTF-8 invariant is structural
}

/// Precompute a vector of byte offsets where each line starts.
///
/// Line `n` (1-indexed) begins at `line_starts[n - 1]`. Always includes
/// `0` as the first entry so empty input still has line 1.
pub(crate) fn build_line_index(source: &str) -> Vec<usize> {
    let mut idx = vec![0_usize];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            idx.push(i + 1);
        }
    }
    idx
}

/// Convert a byte offset to (1-indexed line, 1-indexed column) using a
/// precomputed line-starts index.
#[expect(clippy::cast_possible_truncation, reason = "source files << 4 GiB")]
#[expect(
    clippy::indexing_slicing,
    reason = "line_idx comes from binary_search on line_starts, so it is always in bounds"
)]
pub(crate) fn locate(line_starts: &[usize], byte_offset: usize) -> (u32, u32) {
    let line_idx = match line_starts.binary_search(&byte_offset) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };
    let line = line_idx as u32 + 1; // WHY: source files are smaller than 4 GiB
    let col = (byte_offset - line_starts[line_idx]) as u32 + 1; // WHY: source files are smaller than 4 GiB
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> TokenRegistry {
        TokenRegistry::from_tokens(["--bg", "--accent", "--text-primary"])
    }

    #[test]
    fn line_index_for_empty_source() {
        assert_eq!(build_line_index(""), vec![0]);
    }

    #[test]
    fn locate_first_byte_is_line_one_col_one() {
        let starts = build_line_index("hello\nworld");
        assert_eq!(locate(&starts, 0), (1, 1));
    }

    #[test]
    fn locate_after_newline() {
        let starts = build_line_index("ab\ncd\nef");
        // 'c' is at byte 3 (after 'a','b','\n')
        assert_eq!(locate(&starts, 3), (2, 1));
        // 'd' is at byte 4
        assert_eq!(locate(&starts, 4), (2, 2));
        // 'e' is at byte 6
        assert_eq!(locate(&starts, 6), (3, 1));
    }

    #[test]
    fn lints_undocumented_token() {
        let src = "div { color: var(--missing); }\n";
        let diags = lint_css(&registry(), src, Path::new("a.css"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].token.as_deref(), Some("--missing"));
        assert_eq!(diags[0].line, 1);
        // Position should point at "--missing" start, not "var(" start.
        assert_eq!(
            &src[diags[0].byte_offset..diags[0].byte_offset + diags[0].byte_len],
            "--missing"
        );
    }

    #[test]
    fn skips_documented_token() {
        let src = "div { color: var(--accent); }";
        let diags = lint_css(&registry(), src, Path::new("a.css"));
        assert!(diags.is_empty());
    }

    #[test]
    fn handles_var_with_fallback() {
        let src = "div { color: var(--missing, #000); }";
        let diags = lint_css(&registry(), src, Path::new("a.css"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].token.as_deref(), Some("--missing"));
    }

    #[test]
    fn handles_whitespace_inside_var() {
        let src = "div { color: var( --missing ); }";
        let diags = lint_css(&registry(), src, Path::new("a.css"));
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn locates_across_multiple_lines() {
        let src = "div {\n  color: var(--missing);\n}\n";
        let diags = lint_css(&registry(), src, Path::new("a.css"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 2);
    }

    #[test]
    fn reports_one_diagnostic_per_reference() {
        let src = "a { color: var(--bad); border: 1px solid var(--bad); }";
        let diags = lint_css(&registry(), src, Path::new("a.css"));
        assert_eq!(diags.len(), 2);
    }

    // ---- Masker correctness (caught by QA swarm A01 critical findings) ---

    #[test]
    fn ignores_var_inside_double_quoted_string() {
        // Pre-fix: this would have flagged --missing as undocumented.
        let src = "div { content: \"var(--missing)\"; }";
        let diags = lint_css(&registry(), src, Path::new("a.css"));
        assert!(
            diags.is_empty(),
            "var() inside a string is not a token reference"
        );
    }

    #[test]
    fn ignores_var_inside_single_quoted_string() {
        let src = "div { content: 'var(--missing)'; }";
        let diags = lint_css(&registry(), src, Path::new("a.css"));
        assert!(diags.is_empty());
    }

    #[test]
    fn ignores_var_inside_block_comment() {
        let src = "div { /* var(--missing) example */ color: red; }";
        let diags = lint_css(&registry(), src, Path::new("a.css"));
        assert!(diags.is_empty());
    }

    #[test]
    fn detects_token_with_inline_comment_between_token_and_paren() {
        // Pre-fix: regex required `\s*[,)]` and missed this.
        let src = "div { color: var(--missing /* with comment */); }";
        let diags = lint_css(&registry(), src, Path::new("a.css"));
        assert_eq!(diags.len(), 1, "expected to detect through the comment");
        assert_eq!(diags[0].token.as_deref(), Some("--missing"));
    }

    #[test]
    fn handles_escaped_quote_inside_string() {
        // Backslash-escape the quote — the masker must not exit the string early.
        let src = r#"div { content: "say \"var(--missing)\""; }"#;
        let diags = lint_css(&registry(), src, Path::new("a.css"));
        assert!(diags.is_empty());
    }

    #[test]
    fn unterminated_string_is_masked_to_eof() {
        // Hostile input — we must not panic and must not flag inside the string.
        let src = "div { content: \"var(--missing)";
        let diags = lint_css(&registry(), src, Path::new("a.css"));
        assert!(diags.is_empty());
    }

    #[test]
    fn unterminated_comment_is_masked_to_eof() {
        let src = "div { /* var(--missing)";
        let diags = lint_css(&registry(), src, Path::new("a.css"));
        assert!(diags.is_empty());
    }

    #[test]
    fn masking_preserves_byte_offsets_for_line_col() {
        // The diagnostic position must still point at the real --missing
        // even though earlier bytes were masked.
        let src = "/* leading */ div { color: var(--missing); }\n";
        let diags = lint_css(&registry(), src, Path::new("a.css"));
        assert_eq!(diags.len(), 1);
        assert_eq!(
            &src[diags[0].byte_offset..diags[0].byte_offset + diags[0].byte_len],
            "--missing"
        );
    }
}
