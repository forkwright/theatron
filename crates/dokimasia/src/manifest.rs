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

/// The forbidden table header, in its canonical spelling.
const PATCH_HEADER: &str = "[patch.crates-io]";

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
        // NOTE: accept a bare header or a header followed by an inline
        // `# comment` — both valid TOML spellings of the same table.
        let trimmed = line_str.trim();
        let is_header = trimmed == PATCH_HEADER
            || trimmed
                .strip_prefix(PATCH_HEADER)
                .is_some_and(|rest| rest.trim_start().starts_with('#'));
        if is_header {
            found_line = u32::try_from(line_idx).unwrap_or(0) + 1;
            // Find byte offset of the `[` character on this line.
            let bracket_rel = line_str.find('[').unwrap_or(0);
            found_offset = line_start + bracket_rel;
            found_col = u32::try_from(bracket_rel).unwrap_or(0) + 1;
            // WHY: length is the header token itself, measured from the
            // bracket — never from line start (indentation previously
            // inflated the span past the closing `]`, and past EOF when
            // the header was the final line).
            found_len = PATCH_HEADER.len();
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
