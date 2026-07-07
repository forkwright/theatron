//! Cargo manifest scanner.
//!
//! Flags `[patch.crates-io]` blocks in `Cargo.toml` files. Per fleet
//! doctrine, patches against external deps must live in fleet forks under
//! `forkwright/` rather than as workspace patch-blocks — those bit-rot and
//! obscure the dependency graph.

use std::path::Path;

use crate::css::build_line_index;
use crate::diagnostic::Diagnostic;
use crate::tokens::TokenRegistry;

/// Locate a `[patch.crates-io]` table header on a single source line, in
/// any legal TOML spelling — bare or quoted keys, arbitrary whitespace
/// around the dot or inside the brackets — optionally followed by a
/// trailing `# comment`.
///
/// *Whether* a `[patch.crates-io]` table exists in the document is
/// decided by `toml::from_str` in [`lint_manifest`], which accepts every
/// legal spelling. This scanner only locates *which line* it's on for
/// diagnostic positioning; if it recognized fewer spellings than the
/// parser, a legally-alternate header would silently mislocate to the
/// line-1 fallback in [`lint_manifest`]. Returns `(bracket_byte_offset,
/// header_byte_len)` relative to `line` — the span from `[` through the
/// matching `]`, inclusive.
fn match_patch_header(line: &str) -> Option<(usize, usize)> {
    let after_ws = line.trim_start();
    let leading_ws = line.len() - after_ws.len();
    if !after_ws.starts_with('[') {
        return None;
    }

    let close_rel = find_header_close_bracket(after_ws)?;
    let inner = after_ws.get(1..close_rel)?;
    match split_dotted_keys(inner).as_slice() {
        [patch, crates_io] if patch == "patch" && crates_io == "crates-io" => {
            Some((leading_ws, close_rel + 1))
        }
        _ => None,
    }
}

/// Find the byte index (into `s`, which starts with `[`) of the matching
/// `]`, skipping over `]` characters that appear inside a quoted key
/// segment (`[patch."crates-io]-like"]` — contrived, but a `]` inside a
/// quoted key is legal TOML and must not end the scan early).
fn find_header_close_bracket(s: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (idx, c) in s.char_indices().skip(1) {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if q == '"' && c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            ']' => return Some(idx),
            _ => {} // NOTE: ordinary key-path byte inside the `[patch...]` header -- keep scanning for the closing `]`
        }
    }
    None
}

/// Split a TOML dotted-key header's inner content (`patch.crates-io`,
/// ` patch . "crates-io" `, `patch."crates-io"`, ...) into normalized,
/// unquoted key segments. An unterminated quote yields a segment that
/// still contains the stray quote character, which will simply fail the
/// caller's equality check against the expected key names.
fn split_dotted_keys(inner: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for c in inner.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '.' => {
                keys.push(std::mem::take(&mut current).trim().to_string());
            }
            Some(_) | None => current.push(c),
        }
    }
    keys.push(current.trim().to_string());
    keys
}

/// Lint a Cargo manifest source string, returning a diagnostic if a
/// `[patch.crates-io]` table is present.
///
/// The `_registry` parameter is unused but required so the dispatch
/// signature in `linter.rs::read_and_scan` matches the existing
/// `fn(&TokenRegistry, &str, &Path) -> Vec<Diagnostic>` shape used by
/// `lint_css` and `lint_rust`.
pub(crate) fn lint_manifest(
    _registry: &TokenRegistry,
    source: &str,
    path: &Path,
) -> Vec<Diagnostic> {
    let parsed: toml::Value = match toml::from_str(source) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    // Check for top-level `patch.crates-io` table.
    let has_patch_crates_io = parsed
        .get("patch")
        .and_then(|p| p.get("crates-io"))
        .is_some();

    if !has_patch_crates_io {
        return Vec::new();
    }

    // Find the line where `[patch.crates-io]` appears in raw source.
    let line_starts = build_line_index(source);
    let mut found_line = 1_u32;
    let mut found_col = 1_u32;
    let mut found_offset = 0_usize;
    let mut found_len = 0_usize;

    for (line_idx, line_start) in line_starts.iter().enumerate() {
        let line_end = line_starts
            .get(line_idx + 1)
            .copied()
            .unwrap_or(source.len());
        let line_str = &source[*line_start..line_end]; // kanon:ignore RUST/indexing-slicing -- line_start/line_end come from build_line_index(source), always in-bounds and at char boundaries (line_starts hold byte positions immediately after `\n`)
        if let Some((bracket_rel, header_len)) = match_patch_header(line_str) {
            found_line = u32::try_from(line_idx).unwrap_or(0) + 1;
            found_offset = line_start + bracket_rel;
            found_col = u32::try_from(bracket_rel).unwrap_or(0) + 1;
            // WHY: length is the header token itself, measured from the
            // bracket — never from line start (indentation previously
            // inflated the span past the closing `]`, and past EOF when
            // the header was the final line).
            found_len = header_len;
            break;
        }
    }

    vec![Diagnostic::forbidden_patch_block(
        path.to_path_buf(),
        found_line,
        found_col,
        found_offset,
        found_len,
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::TokenRegistry;

    fn registry() -> TokenRegistry {
        TokenRegistry::from_tokens(["--bg"])
    }

    #[test]
    fn flags_patch_crates_io_block() {
        let src = r#"
[package]
name = "foo"
version = "0.1.0"

[patch.crates-io]
serde = { git = "https://forge.forkwright.com/forkwright/serde" }
"#;
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "forbidden-patch-block");
        assert_eq!(diags[0].severity, crate::diagnostic::Severity::Error);
        // Line of the `[patch.crates-io]` header.
        // Source has a leading newline so [package] is line 2;
        // [patch.crates-io] is line 6.
        assert_eq!(diags[0].line, 6);
    }

    #[test]
    fn indented_header_span_covers_exactly_the_header() {
        let src = "[package]\nname = \"foo\"\nversion = \"0.1.0\"\n\n  [patch.crates-io]\n  serde = { git = \"https://example.com/serde\" }\n";
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 5);
        assert_eq!(diags[0].column, 3);
        let span = &src[diags[0].byte_offset..diags[0].byte_offset + diags[0].byte_len];
        assert_eq!(
            span, "[patch.crates-io]",
            "span must cover exactly the header"
        );
    }

    #[test]
    fn quoted_key_header_variant_is_located_precisely() {
        // `patch."crates-io"` is a legal TOML dotted key — the quoted
        // segment is equivalent to the bare `crates-io` key. Before the
        // fix this fell through to the line-1/col-1/zero-length fallback
        // because the raw-line scan only matched the exact literal
        // `[patch.crates-io]`.
        let src = "[package]\nname = \"foo\"\nversion = \"0.1.0\"\n\n[patch.\"crates-io\"]\nserde = { git = \"https://example.com/serde\" }\n";
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags[0].line, 5,
            "must not mislocate to the line-1 fallback"
        );
        assert_eq!(diags[0].column, 1);
        let span = &src[diags[0].byte_offset..diags[0].byte_offset + diags[0].byte_len];
        assert_eq!(span, "[patch.\"crates-io\"]");
    }

    #[test]
    fn whitespace_inside_brackets_header_variant_is_located_precisely() {
        // `[ patch.crates-io ]` and `[patch . crates-io]` are both legal
        // TOML — whitespace is allowed around dotted-key segments and
        // inside table-header brackets.
        let src = "[package]\nname = \"foo\"\nversion = \"0.1.0\"\n\n[ patch . crates-io ]\nserde = { git = \"https://example.com/serde\" }\n";
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags[0].line, 5,
            "must not mislocate to the line-1 fallback"
        );
        assert_eq!(diags[0].column, 1);
        let span = &src[diags[0].byte_offset..diags[0].byte_offset + diags[0].byte_len];
        assert_eq!(span, "[ patch . crates-io ]");
    }

    #[test]
    fn indented_header_at_end_of_file_renders_without_error() {
        // Header is the final line, no trailing newline — the old
        // line-start-relative length overran the file boundary here and
        // render_human aborted with an io::Error.
        let src = "[package]\nname = \"foo\"\nversion = \"0.1.0\"\n\n  [patch.crates-io]";
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert_eq!(diags.len(), 1);
        let end = diags[0].byte_offset + diags[0].byte_len;
        assert!(
            end <= src.len(),
            "span end {end} overruns file len {}",
            src.len()
        );
        let span = &src[diags[0].byte_offset..end];
        assert_eq!(span, "[patch.crates-io]");

        let mut buf: Vec<u8> = Vec::new();
        let mut writer = codespan_reporting::term::termcolor::NoColor::new(&mut buf);
        crate::render::render_human(&diags, &mut writer, |_| src.to_string())
            .expect("render_human must succeed on an EOF-adjacent span");
    }

    #[test]
    fn header_with_trailing_comment_reports_correct_position() {
        // `[patch.crates-io] # reason` is valid TOML; the raw-line scan
        // previously missed it and fell back to a 1:1 zero-width span.
        let src = "[package]\nname = \"foo\"\nversion = \"0.1.0\"\n\n[patch.crates-io] # vendored until upstream release\nserde = { git = \"https://example.com/serde\" }\n";
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 5, "must point at the header line, not 1:1");
        assert_eq!(diags[0].column, 1);
        let span = &src[diags[0].byte_offset..diags[0].byte_offset + diags[0].byte_len];
        assert_eq!(span, "[patch.crates-io]", "span must exclude the comment");
    }

    #[test]
    fn no_patch_block_no_diagnostics() {
        let src = r#"
[package]
name = "foo"
version = "0.1.0"

[dependencies]
serde = "1"
"#;
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert!(diags.is_empty());
    }

    #[test]
    fn other_patch_registries_are_out_of_scope() {
        // [patch.<other-registry>] is allowed — only crates-io is forbidden.
        let src = r#"
[package]
name = "foo"
version = "0.1.0"

[patch.some-other-registry]
serde = { git = "https://example.com/serde" }
"#;
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert!(diags.is_empty());
    }

    #[test]
    fn malformed_toml_returns_empty() {
        // cargo check surfaces parse errors elsewhere; the linter should
        // not double-report them as lint findings.
        let src = "this is :: not toml [[[";
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert!(diags.is_empty());
    }
}
