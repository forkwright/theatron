//! Rust source scanner.
//!
//! Parses a Rust file via `syn`, walks the AST visiting every string literal
//! (including those nested inside macro invocations such as `rsx!`), and
//! extracts `var(--token)` patterns from each literal's content.
//!
//! Diagnostics are reported at the literal's source position spanning the
//! entire literal token. Mapping a position *inside* a literal back to a
//! source byte offset is not generally possible (escape sequences, raw
//! strings, etc.), and a literal-spanning span is precise enough for the
//! reader to find the offending token.

use std::path::Path;
use std::sync::OnceLock;

use proc_macro2::{LineColumn, TokenStream, TokenTree};
use regex::Regex;
use syn::visit::Visit;

use crate::css::{build_line_index, locate};
use crate::diagnostic::Diagnostic;
use crate::tokens::TokenRegistry;

/// Match `var(--token)` within string literal *contents* (not source).
fn var_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"var\(\s*(--[a-z][a-z0-9-]*)\s*[,)]").expect("var regex compiles"))
}

/// Lint a Rust source string. Returns one diagnostic per undocumented
/// token reference found in any string literal.
///
/// If the source fails to parse as a Rust file, returns an empty
/// diagnostic list — parse errors aren't lint findings, and surface
/// elsewhere (`cargo check`).
pub(crate) fn lint_rust(registry: &TokenRegistry, source: &str, path: &Path) -> Vec<Diagnostic> {
    let Ok(file) = syn::parse_file(source) else {
        return Vec::new();
    };
    let line_starts = build_line_index(source);
    let mut visitor = LitVisitor {
        source,
        line_starts: &line_starts,
        registry,
        path,
        diagnostics: Vec::new(),
    };
    visitor.visit_file(&file);
    visitor.diagnostics
}

struct LitVisitor<'a> {
    source: &'a str,
    line_starts: &'a [usize],
    registry: &'a TokenRegistry,
    path: &'a Path,
    diagnostics: Vec<Diagnostic>,
}

impl<'ast> Visit<'ast> for LitVisitor<'_> {
    fn visit_lit_str(&mut self, lit: &'ast syn::LitStr) {
        let value = lit.value();
        let span = lit.span();
        self.scan_literal(&value, span.start(), span.end());
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        // syn::visit's default impl does not recurse into macro contents.
        // Walk the token stream manually to catch literals inside `rsx!`,
        // `format!`, etc. — that's where Dioxus components live.
        self.walk_tokens(mac.tokens.clone());
    }

    fn visit_item_mod(&mut self, m: &'ast syn::ItemMod) {
        // Skip `#[cfg(test)]` modules. Test fixtures intentionally include
        // bogus tokens (`var(--missing)`) and would otherwise produce
        // false positives whenever the linter is run against a workspace
        // that includes its own tests.
        if has_cfg_test(&m.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, m);
    }

    fn visit_attribute(&mut self, _attr: &'ast syn::Attribute) {
        // Don't descend into attributes. Doc comments (`/// …`) desugar to
        // `#[doc = "…"]` and would otherwise be scanned as string literals,
        // producing false positives whenever a doc comment cites an example
        // like `var(--token)`. Other attributes (`#[deprecated = "…"]`,
        // `#[link_name = "…"]`, etc.) are metadata, not source CSS, so the
        // same blanket skip applies.
    }
}

/// Detect `#[cfg(test)]` (or `#[cfg(any(test, …))]`) on a module.
fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("test") {
                found = true;
            }
            Ok(())
        });
        found
    })
}

impl LitVisitor<'_> {
    fn walk_tokens(&mut self, stream: TokenStream) {
        for tt in stream {
            match tt {
                TokenTree::Group(g) => self.walk_tokens(g.stream()),
                TokenTree::Literal(lit) => {
                    if let syn::Lit::Str(s) = syn::Lit::new(lit) {
                        let span = s.span();
                        self.scan_literal(&s.value(), span.start(), span.end());
                    }
                }
                _ => {}
            }
        }
    }

    fn scan_literal(&mut self, content: &str, span_start: LineColumn, span_end: LineColumn) {
        let Some((byte_offset, byte_len)) = self.span_to_byte_range(span_start, span_end) else {
            return;
        };
        let line = u32::try_from(span_start.line).unwrap_or(0);
        // proc-macro2 uses 0-indexed columns; we use 1-indexed.
        let column = u32::try_from(span_start.column.saturating_add(1)).unwrap_or(0);

        for caps in var_regex().captures_iter(content) {
            let token = caps.get(1).expect("regex always captures group 1").as_str();
            if self.registry.contains(token) {
                continue;
            }
            self.diagnostics.push(Diagnostic::undocumented_token(
                self.path.to_path_buf(),
                line,
                column,
                byte_offset,
                byte_len,
                token.to_string(),
            ));
        }
    }

    /// Convert a (start, end) `LineColumn` span pair into a byte range in
    /// the original source. Returns `None` if either endpoint is out of
    /// bounds (defensive against proc-macro2 returning columns past EOL).
    fn span_to_byte_range(&self, start: LineColumn, end: LineColumn) -> Option<(usize, usize)> {
        let start_offset = lc_to_offset(self.source, self.line_starts, start)?;
        let end_offset = lc_to_offset(self.source, self.line_starts, end)?;
        if end_offset < start_offset {
            return None;
        }
        Some((start_offset, end_offset - start_offset))
    }
}

/// Map a proc-macro2 `LineColumn` (1-indexed line, 0-indexed column) to a
/// byte offset using the source's line index.
fn lc_to_offset(source: &str, line_starts: &[usize], lc: LineColumn) -> Option<usize> {
    let line_idx = lc.line.checked_sub(1)?;
    let line_start = *line_starts.get(line_idx)?;
    let offset = line_start.checked_add(lc.column)?;
    if offset > source.len() {
        return None;
    }
    Some(offset)
}

// `locate` is re-exported so other tests can use it; not used here directly.
#[allow(dead_code)]
fn _keep_locate_in_scope(line_starts: &[usize], byte_offset: usize) -> (u32, u32) {
    locate(line_starts, byte_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> TokenRegistry {
        TokenRegistry::from_tokens(["--bg", "--accent", "--text-primary"])
    }

    #[test]
    fn lints_undocumented_token_in_const_str() {
        let src = "const STYLE: &str = \"color: var(--missing);\";\n";
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert_eq!(diags.len(), 1, "expected 1 diagnostic, got: {diags:?}");
        assert_eq!(diags[0].token.as_deref(), Some("--missing"));
    }

    #[test]
    fn skips_documented_token() {
        let src = "const STYLE: &str = \"color: var(--accent);\";\n";
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert!(diags.is_empty(), "expected empty, got: {diags:?}");
    }

    #[test]
    fn finds_token_inside_macro_invocation() {
        // rsx!-like macro — syn's default Visit doesn't recurse into macro
        // tokens, so this verifies our custom walker.
        let src = r#"
fn render() {
    some_macro! {
        div {
            style: "color: var(--missing);",
        }
    }
}
"#;
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert_eq!(diags.len(), 1, "expected 1 diagnostic, got: {diags:?}");
        assert_eq!(diags[0].token.as_deref(), Some("--missing"));
    }

    #[test]
    fn finds_multiple_tokens_in_one_literal() {
        let src = "const S: &str = \"a: var(--bad1); b: var(--bad2);\";\n";
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn handles_raw_string_literals() {
        let src = "const S: &str = r\"var(--bad);\";\n";
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn ignores_byte_string_literals() {
        // Byte strings can't carry CSS — we should skip them and not panic.
        let src = "const B: &[u8] = b\"var(--bad)\";\n";
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert!(diags.is_empty());
    }

    #[test]
    fn unparseable_source_returns_empty() {
        let src = "this is not rust !!!";
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert!(diags.is_empty());
    }

    #[test]
    fn doc_comments_are_not_linted() {
        // Doc comments desugar to #[doc = "…"] string-literal attributes.
        // They legitimately cite token names as examples and must not be
        // flagged.
        let src = r"
/// Example: pass `var(--missing)` to opt out.
fn render() {}
";
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert!(
            diags.is_empty(),
            "doc comments must not lint, got: {diags:?}"
        );
    }

    #[test]
    fn other_string_attributes_are_not_linted() {
        let src = r#"
#[deprecated = "swap to var(--missing) (intentional, this is metadata)"]
fn old() {}
"#;
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert!(diags.is_empty());
    }

    #[test]
    fn cfg_test_module_is_skipped() {
        // Tokens inside #[cfg(test)] mod blocks should not be linted —
        // they are typically test fixtures intentionally referencing
        // undocumented tokens.
        let src = r#"
const REAL: &str = "color: var(--accent);";

#[cfg(test)]
mod tests {
    const BOGUS: &str = "color: var(--definitely-bad);";
}
"#;
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert!(
            diags.is_empty(),
            "expected no diagnostics (test module should be skipped), got: {diags:?}"
        );
    }

    #[test]
    fn diagnostic_position_lines_up_with_literal() {
        let src = "fn x() {\n    let s = \"var(--missing)\";\n}\n";
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert_eq!(diags.len(), 1);
        // Literal starts on line 2.
        assert_eq!(diags[0].line, 2);
        // Span should cover the literal source (including quotes).
        let span = &src[diags[0].byte_offset..diags[0].byte_offset + diags[0].byte_len];
        assert!(
            span.starts_with('"'),
            "span should start at literal: {span:?}"
        );
        assert!(span.ends_with('"'), "span should end at literal: {span:?}");
    }
}
