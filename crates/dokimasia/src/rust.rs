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

use crate::css::{build_line_index, mask_strings_and_comments};
use crate::diagnostic::Diagnostic;
use crate::tokens::TokenRegistry;

/// Match `var(--token)` within string literal *contents* (not source).
#[expect(
    clippy::expect_used,
    reason = "hardcoded regex compilation; failure is a programming error"
)]
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
    line_starts: &'a [usize], // kanon:ignore RUST/indexing-slicing -- type annotation, not runtime indexing
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

    fn visit_item(&mut self, item: &'ast syn::Item) {
        // Skip any item gated on `#[cfg(test)]` (or any nested combinator
        // containing it). The original implementation only skipped `mod`
        // items and only matched `cfg(test)` / `cfg(any(test, …))` at the
        // top level — `cfg(all(test, feature = "…"))` and cfg(test) on
        // non-module items (functions, consts, statics) leaked through.
        if has_cfg_test(item_attrs(item)) {
            return;
        }
        syn::visit::visit_item(self, item);
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

/// Best-effort attribute extraction from any `syn::Item` variant.
fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    use syn::Item;
    match item {
        Item::Const(i) => &i.attrs,
        Item::Enum(i) => &i.attrs,
        Item::ExternCrate(i) => &i.attrs,
        Item::Fn(i) => &i.attrs,
        Item::ForeignMod(i) => &i.attrs,
        Item::Impl(i) => &i.attrs,
        Item::Macro(i) => &i.attrs,
        Item::Mod(i) => &i.attrs,
        Item::Static(i) => &i.attrs,
        Item::Struct(i) => &i.attrs,
        Item::Trait(i) => &i.attrs,
        Item::TraitAlias(i) => &i.attrs,
        Item::Type(i) => &i.attrs,
        Item::Union(i) => &i.attrs,
        Item::Use(i) => &i.attrs,
        // Item::Verbatim and any future variants — no attrs to inspect.
        _ => &[],
    }
}

/// Detect `#[cfg(test)]` recursively, including under `cfg(any(...))`,
/// `cfg(all(...))`, and `cfg(not(...))` combinators.
fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        attr.parse_args::<syn::Meta>()
            .ok()
            .is_some_and(|meta| meta_contains_test(&meta))
    })
}

fn meta_contains_test(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(p) => p.is_ident("test"),
        syn::Meta::List(list) => {
            // `not(test)` means "compile when NOT testing" — that's production
            // code and MUST be linted. Recurse only through `all(...)` /
            // `any(...)` combinators, never through `not(...)`. (Caught by
            // QA wave 2 #13 R-01.)
            if list.path.is_ident("not") {
                return false;
            }
            list.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            )
            .is_ok_and(|inner| inner.iter().any(meta_contains_test))
        }
        syn::Meta::NameValue(_) => false,
    }
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
                _ => {} // kanon:ignore RUST/empty-match-arm -- punctuation and identifiers are not string literals
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

        // Mask nested CSS strings and comments inside the literal so we
        // don't false-positive on `style: "background: \"var(--ok)\""`
        // or false-negative on `var(--foo /* note */)`.
        let masked = mask_strings_and_comments(content);

        for caps in var_regex().captures_iter(&masked) {
            let Some(token) = caps.get(1).map(|m| m.as_str()) else {
                continue;
            };
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

/// Map a proc-macro2 `LineColumn` (1-indexed line, 0-indexed character
/// column) to a byte offset using the source's line index.
fn lc_to_offset(source: &str, line_starts: &[usize], lc: LineColumn) -> Option<usize> {
    let line_idx = lc.line.checked_sub(1)?;
    let line_start = *line_starts.get(line_idx)?;
    let line_end = line_starts
        .get(line_idx + 1)
        .copied()
        .unwrap_or(source.len());
    let line = source.get(line_start..line_end)?;
    let line_offset = line
        .char_indices()
        .nth(lc.column)
        .map_or(line.len(), |(byte_offset, _)| byte_offset);
    line_start.checked_add(line_offset)
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
    fn utf8_before_literal_uses_char_boundary_byte_span() {
        let src = "const NOTE: &str = \"\\u{1f4a5}\"; const S: &str = \"color: var(--bad);\";\n";
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert_eq!(diags.len(), 1, "expected 1 diagnostic, got: {diags:?}");

        let diag = &diags[0];
        let span_end = diag.byte_offset + diag.byte_len;
        assert!(src.is_char_boundary(diag.byte_offset));
        assert!(src.is_char_boundary(span_end));
        assert_eq!(&src[diag.byte_offset..span_end], "\"color: var(--bad);\"");

        let mut writer = codespan_reporting::term::termcolor::NoColor::new(Vec::new());
        crate::render::render_human(&diags, &mut writer, |_| src.to_string()).expect("render");
    }

    #[test]
    fn ignores_byte_string_literals() {
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

    // ---- Recursive cfg-test (caught by QA swarm A01 H1/H2) ---

    #[test]
    fn cfg_all_test_module_is_skipped() {
        let src = r#"
#[cfg(all(test, feature = "extra"))]
mod combined {
    const BOGUS: &str = "color: var(--bogus-all);";
}
"#;
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert!(
            diags.is_empty(),
            "cfg(all(test, ...)) module must be skipped, got: {diags:?}"
        );
    }

    #[test]
    fn cfg_any_test_module_is_skipped() {
        let src = r#"
#[cfg(any(test, debug_assertions))]
mod also_tests {
    const BOGUS: &str = "color: var(--bogus-any);";
}
"#;
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert!(diags.is_empty());
    }

    #[test]
    fn cfg_test_on_function_is_skipped() {
        // Tokens inside #[cfg(test)] FUNCTIONS (not just modules) must be
        // skipped — used by inline test helpers + integration test scopes.
        let src = r#"
#[cfg(test)]
fn helper() {
    let _ = "color: var(--definitely-bad);";
}
"#;
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert!(
            diags.is_empty(),
            "cfg(test) on functions must be skipped, got: {diags:?}"
        );
    }

    #[test]
    fn cfg_not_test_is_production_code_and_lints() {
        // `cfg(not(test))` evaluates to "compile when NOT testing" — that's
        // production code. Must be linted. (Caught by QA wave 2 #13 R-01.)
        let src = r#"
#[cfg(not(test))]
const PROD: &str = "color: var(--bogus-prod);";
"#;
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert_eq!(
            diags.len(),
            1,
            "cfg(not(test)) is PRODUCTION code, must lint, got: {diags:?}"
        );
        assert_eq!(diags[0].token.as_deref(), Some("--bogus-prod"));
    }

    #[test]
    fn cfg_test_on_const_is_skipped() {
        let src = r#"
#[cfg(test)]
const BOGUS: &str = "color: var(--bogus-const);";
"#;
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert!(diags.is_empty());
    }

    // ---- Masking inside string literal contents (caught by QA swarm A01) ---

    #[test]
    fn masks_nested_string_inside_literal_content() {
        // The literal contains a CSS-like string with an escaped quote
        // wrapping a var(). We must not flag inside the inner string.
        let src = r#"const S: &str = "background: \"var(--definitely-bad)\";";"#;
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert!(
            diags.is_empty(),
            "var() inside escaped inner string must be masked, got: {diags:?}"
        );
    }

    #[test]
    fn detects_token_with_mid_token_comment_in_literal() {
        let src = r#"const S: &str = "color: var(--missing /* note */);";"#;
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].token.as_deref(), Some("--missing"));
    }

    #[test]
    fn diagnostic_position_lines_up_with_literal() {
        let src = "fn x() {\n    let s = \"var(--missing)\";\n}\n";
        let diags = lint_rust(&registry(), src, Path::new("a.rs"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 2);
        let span = &src[diags[0].byte_offset..diags[0].byte_offset + diags[0].byte_len];
        assert!(
            span.starts_with('"'),
            "span should start at literal: {span:?}"
        );
        assert!(span.ends_with('"'), "span should end at literal: {span:?}");
    }
}
