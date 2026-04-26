//! CSS source scanner.
//!
//! Extracts every `var(--token)` reference from a CSS source string and
//! produces a [`Diagnostic`] for any token that is not in the supplied
//! [`TokenRegistry`].
//!
//! Position tracking precomputes line-start byte offsets in a single pass,
//! then locates each match in O(log lines) via binary search.

use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;

use crate::diagnostic::Diagnostic;
use crate::tokens::TokenRegistry;

/// Match `var(--token)` references with optional whitespace and fallback
/// value (`var(--foo, fallback)`). Capture group 1 is the token name.
fn var_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"var\(\s*(--[a-z][a-z0-9-]*)\s*[,)]").expect("var regex compiles"))
}

/// Lint a CSS source string, returning one diagnostic per undocumented
/// token reference.
pub(crate) fn lint_css(registry: &TokenRegistry, source: &str, path: &Path) -> Vec<Diagnostic> {
    let line_starts = build_line_index(source);
    let mut diagnostics = Vec::new();
    for caps in var_regex().captures_iter(source) {
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
pub(crate) fn locate(line_starts: &[usize], byte_offset: usize) -> (u32, u32) {
    let line_idx = match line_starts.binary_search(&byte_offset) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };
    let line = line_idx as u32 + 1;
    let col = (byte_offset - line_starts[line_idx]) as u32 + 1;
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
}
