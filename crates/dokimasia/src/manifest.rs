//! Cargo manifest scanner.
//!
//! Flags `[patch.crates-io]` blocks in `Cargo.toml` files. Per fleet
//! doctrine, patches against external deps must live in fleet forks under
//! `forkwright/` rather than as workspace patch-blocks — those bit-rot and
//! obscure the dependency graph.
//!
//! Also holds intra-workspace path dependencies in version lockstep with
//! `workspace.package.version`. Cargo's caret semantics let those constraints
//! name an older release line without breaking the build, so nothing else
//! surfaces the drift.

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

/// Lint a Cargo manifest source string for `[patch.crates-io]` tables and
/// for intra-workspace path dependencies whose version constraint has
/// drifted from `workspace.package.version`.
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

    let mut diagnostics = check_patch_crates_io(&parsed, source, path);
    diagnostics.extend(check_version_lockstep(&parsed, source, path));
    diagnostics
}

/// Report every intra-workspace path dependency whose declared `version`
/// differs from `workspace.package.version`.
///
/// WHY this is enforced rather than left to review: the release automation
/// bumps `workspace.package.version` and nothing else, so keeping these
/// constraints correct otherwise depends on remembering an edit the release
/// mechanism is guaranteed not to make. Cargo's caret semantics let a 1.4.0
/// package satisfy a `1.3.0` requirement, so the drift builds green and
/// stays invisible until the first major bump stops satisfying it.
fn check_version_lockstep(parsed: &toml::Value, source: &str, path: &Path) -> Vec<Diagnostic> {
    let Some(workspace) = parsed.get("workspace") else {
        return Vec::new();
    };
    let Some(workspace_version) = workspace
        .get("package")
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
    else {
        return Vec::new();
    };
    let Some(dependencies) = workspace
        .get("dependencies")
        .and_then(toml::Value::as_table)
    else {
        return Vec::new();
    };

    dependencies
        .iter()
        .filter_map(|(name, spec)| {
            let table = spec.as_table()?;
            table.get("path")?;
            let declared = table.get("version")?.as_str()?;
            if declared == workspace_version {
                return None;
            }
            let (line, column, byte_offset, byte_len) = locate_dependency(source, name);
            Some(Diagnostic::version_lockstep_drift(
                path.to_path_buf(),
                line,
                column,
                byte_offset,
                byte_len,
                name,
                declared,
                workspace_version,
            ))
        })
        .collect()
}

/// Locate the declaration of workspace dependency `name` in the raw source,
/// as `(line, column, byte_offset, byte_len)`.
///
/// Matches both spellings Cargo accepts: an inline `name = { ... }` entry and
/// an expanded `[workspace.dependencies.name]` header. Falls back to the head
/// of the file when neither is found, so a diagnostic is never dropped for
/// want of a position.
fn locate_dependency(source: &str, name: &str) -> (u32, u32, usize, usize) {
    let expanded_header = format!("[workspace.dependencies.{name}]");
    for (line_idx, line_start) in build_line_index(source).iter().enumerate() {
        let line_end = source[*line_start..] // kanon:ignore RUST/indexing-slicing -- line_start comes from build_line_index(source), always in-bounds and at a char boundary
            .find('\n')
            .map_or(source.len(), |offset| line_start + offset);
        let line = &source[*line_start..line_end]; // kanon:ignore RUST/indexing-slicing -- both bounds derive from build_line_index(source) and a `\n` search within it
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let is_inline = trimmed
            .strip_prefix(name)
            .is_some_and(|rest| rest.trim_start().starts_with('='));
        if is_inline || trimmed.starts_with(&expanded_header) {
            let column = u32::try_from(indent).unwrap_or(0) + 1;
            let line_number = u32::try_from(line_idx).unwrap_or(0) + 1;
            let span = if is_inline {
                name.len()
            } else {
                expanded_header.len()
            };
            return (line_number, column, line_start + indent, span);
        }
    }
    (1, 1, 0, 0)
}

/// Report a top-level `[patch.crates-io]` table, positioned at its header.
fn check_patch_crates_io(parsed: &toml::Value, source: &str, path: &Path) -> Vec<Diagnostic> {
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
        assert_eq!(diags, [] as [Diagnostic; 0]);
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
        assert_eq!(diags, [] as [Diagnostic; 0]);
    }

    #[test]
    fn malformed_toml_returns_empty() {
        // cargo check surfaces parse errors elsewhere; the linter should
        // not double-report them as lint findings.
        let src = "this is :: not toml [[[";
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert_eq!(diags, [] as [Diagnostic; 0]);
    }

    #[test]
    fn flags_drifted_inline_path_dependency() {
        let src = r#"
[workspace.package]
version = "1.4.1"

[workspace.dependencies]
bathron = { version = "1.3.0", path = "crates/bathron" }
"#;
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].code, "version-lockstep-drift");
        assert_eq!(diags[0].line, 6, "must point at the declaration");
        assert!(diags[0].message.contains("bathron"));
    }

    #[test]
    fn flags_drifted_expanded_path_dependency() {
        // The expanded-table spelling is the one a reader is least likely to
        // notice, and it is how themelion is declared.
        let src = r#"
[workspace.package]
version = "1.4.1"

[workspace.dependencies.themelion]
version = "1.3.0"
path = "crates/themelion"
"#;
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].line, 5, "must point at the table header");
        let span = &src[diags[0].byte_offset..diags[0].byte_offset + diags[0].byte_len];
        assert_eq!(span, "[workspace.dependencies.themelion]");
    }

    #[test]
    fn matching_path_dependency_version_is_silent() {
        let src = r#"
[workspace.package]
version = "1.4.1"

[workspace.dependencies]
bathron = { version = "1.4.1", path = "crates/bathron" }
"#;
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn registry_dependency_version_is_out_of_scope() {
        // Only path dependencies are held in lockstep — an external crate's
        // version has nothing to do with the workspace version, and flagging
        // it would make the check fire on everything.
        let src = r#"
[workspace.package]
version = "1.4.1"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
"#;
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn path_dependency_without_a_version_is_out_of_scope() {
        // Omitting `version` is the other coherent way to hold the invariant:
        // identity then derives from workspace.package.version alone, and
        // there is no second copy to drift.
        let src = r#"
[workspace.package]
version = "1.4.1"

[workspace.dependencies]
bathron = { path = "crates/bathron" }
"#;
        let diags = lint_manifest(&registry(), src, Path::new("Cargo.toml"));
        assert!(diags.is_empty(), "{diags:?}");
    }

    /// Walk up from this crate to the directory holding the manifest that
    /// declares `[workspace]`.
    ///
    /// WHY not a fixed `../..` join: the depth is a property of the layout,
    /// and a layout change would silently retarget the check at whatever
    /// happened to be two levels up rather than failing.
    fn workspace_root() -> std::path::PathBuf {
        let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        loop {
            let manifest = dir.join("Cargo.toml");
            if std::fs::read_to_string(&manifest)
                .is_ok_and(|text| text.contains("\n[workspace]") || text.starts_with("[workspace]"))
            {
                return dir;
            }
            assert!(
                dir.pop(),
                "no [workspace] manifest above CARGO_MANIFEST_DIR"
            );
        }
    }

    #[test]
    fn this_workspace_keeps_path_dependency_versions_in_lockstep() {
        // WHY this runs against the real manifest rather than a fixture: the
        // release automation bumps workspace.package.version and nothing
        // else, so the invariant needs a check that fires on the repository
        // itself after every release, not one that only proves the detector
        // works. See #220 — v1.4 shipped with all eight constraints at 1.3.0
        // and every gate green.
        let root = workspace_root();
        let manifest = root.join("Cargo.toml");
        let source = std::fs::read_to_string(&manifest).expect("workspace manifest is readable");
        let diags = lint_manifest(&registry(), &source, &manifest);
        assert!(
            diags.is_empty(),
            "workspace manifest violates version lockstep: {diags:#?}"
        );
    }
}
