//! Git-style multi-file diff parsing.

use super::{DiffFile, parse_unified_diff};

/// Parse a git-style multi-file diff into one [`DiffFile`] per file.
///
/// This reuses [`parse_unified_diff`] for each file section, preserving the
/// existing gramma diff model and word-level diff behavior. File paths are
/// derived from `+++` / `---` headers when present, with the `diff --git`
/// header used as a fallback for binary or metadata-only sections.
#[must_use]
pub fn parse_git_diff(raw: &str) -> Vec<DiffFile> {
    git_diff_sections(raw)
        .into_iter()
        .filter_map(|section| {
            let path = git_diff_section_path(&section)?;
            let raw = section.join("\n");
            Some(parse_unified_diff(&path, &raw))
        })
        .collect()
}

fn git_diff_sections(raw: &str) -> Vec<Vec<&str>> {
    let mut sections = Vec::new();
    let mut current = Vec::new();

    for line in raw.lines() {
        if line.starts_with("diff --git ") {
            if !current.is_empty() {
                sections.push(current);
                current = Vec::new();
            }
            current.push(line);
        } else if !current.is_empty()
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("@@ ")
        {
            current.push(line);
        }
    }

    if !current.is_empty() {
        sections.push(current);
    }

    sections
}

fn git_diff_section_path(section: &[&str]) -> Option<String> {
    section
        .iter()
        .find_map(|line| line.strip_prefix("+++ ").and_then(normalize_diff_path))
        .or_else(|| {
            section
                .iter()
                .find_map(|line| line.strip_prefix("--- ").and_then(normalize_diff_path))
        })
        .or_else(|| {
            section
                .iter()
                .find_map(|line| line.strip_prefix("diff --git ").and_then(diff_git_path))
        })
}

fn diff_git_path(header: &str) -> Option<String> {
    if let Some(path) = diff_git_shared_path(header) {
        return Some(path);
    }

    let mut paths = diff_git_path_tokens(header);
    let old_path = paths.next().and_then(normalize_diff_path);
    let new_path = paths.next().and_then(normalize_diff_path);

    new_path.or(old_path)
}

/// Resolve `a/<path> b/<path>` when `<path>` is byte-identical on
/// both sides — the common non-rename case. Git emits this form
/// unquoted whenever the path has no bytes requiring C-quoting, so a
/// naive whitespace split truncates a space-containing name at its
/// first space. Anchoring on the literal `" b/"` separator and
/// requiring both halves to match after stripping the `a/` prefix
/// recovers the full path without depending on `---`/`+++` lines
/// (the caller, `git_diff_section_path`, has already tried those and
/// found none — this fallback only runs for binary/mode-only
/// sections).
fn diff_git_shared_path(header: &str) -> Option<String> {
    let after_a = header.strip_prefix("a/")?;
    let sep = " b/";
    let sep_idx = after_a.find(sep)?;
    let old = after_a.get(..sep_idx)?;
    let new = after_a.get(sep_idx + sep.len()..)?;
    (old == new).then(|| old.to_string())
}

fn diff_git_path_tokens(header: &str) -> impl Iterator<Item = &str> {
    let mut tokens = Vec::new();
    let mut rest = header.trim_start();

    while !rest.is_empty() {
        if rest.starts_with('"') {
            match quoted_token_end(rest) {
                Some(end) => {
                    let token = rest.get(..=end).unwrap_or(rest);
                    tokens.push(token);
                    rest = rest.get(end + 1..).unwrap_or("").trim_start();
                }
                // WHY: unterminated quote — stop rather than emit a
                // truncated token that still carries the stray
                // opening `"`.
                None => break,
            }
        } else if let Some((token, remainder)) = rest.split_once(char::is_whitespace) {
            tokens.push(token);
            rest = remainder.trim_start();
        } else {
            tokens.push(rest);
            break;
        }
    }

    tokens.into_iter()
}

/// Byte index of the closing `"` for a quoted token starting at byte
/// 0 of `s` (`s` must start with `"`). Honors backslash escapes so an
/// escaped quote (`\"`) inside the path does not terminate the token
/// early — git's C-quoting can carry both simple escapes and `\NNN`
/// octal byte escapes ahead of the closing quote.
fn quoted_token_end(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 1; // skip opening quote
    while let Some(&b) = bytes.get(i) {
        match b {
            b'\\' => i += 2,
            b'"' => return Some(i),
            _ => i += 1,
        }
    }
    None
}

fn normalize_diff_path(path: &str) -> Option<String> {
    let path = path
        .split_once('\t')
        .map_or(path, |(path, _metadata)| path)
        .trim();

    if path == "/dev/null" || path.is_empty() {
        return None;
    }

    // NOTE: Git C-quotes paths containing non-ASCII or special bytes;
    // decode escape sequences after stripping the surrounding quotes.
    let decoded = path
        .strip_prefix('"')
        .and_then(|path| path.strip_suffix('"'))
        .map(decode_git_quoted);
    let path = decoded.as_deref().unwrap_or(path);
    let path = path
        .strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path);

    Some(path.to_string())
}

/// Decode git's C-style path quoting: `\NNN` octal byte escapes plus
/// the simple escapes git emits (`\\`, `\"`, `\t`, `\n`, `\r`).
/// Unknown escape sequences are kept verbatim.
fn decode_git_quoted(quoted: &str) -> String {
    let bytes = quoted.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        if byte == b'\\' {
            if let Some(octal) = decode_octal_escape(bytes, i) {
                out.push(octal);
                i += 4;
                continue;
            }
            if let Some(simple) = bytes.get(i + 1).copied().and_then(decode_simple_escape) {
                out.push(simple);
                i += 2;
                continue;
            }
        }
        out.push(byte);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Decode a `\NNN` three-octal-digit escape starting at the backslash.
/// Returns `None` for short, non-octal, or out-of-range (`> \377`)
/// sequences.
fn decode_octal_escape(bytes: &[u8], backslash: usize) -> Option<u8> {
    let high = octal_digit(bytes.get(backslash + 1))?;
    let mid = octal_digit(bytes.get(backslash + 2))?;
    let low = octal_digit(bytes.get(backslash + 3))?;
    u8::try_from(high * 64 + mid * 8 + low).ok()
}

/// Numeric value of an octal digit byte, or `None` for anything else.
fn octal_digit(byte: Option<&u8>) -> Option<u32> {
    match byte {
        Some(&digit @ b'0'..=b'7') => Some(u32::from(digit - b'0')),
        _ => None,
    }
}

/// Decoded byte for a single-character C escape, or `None` if the
/// character is not a recognized escape.
fn decode_simple_escape(escape: u8) -> Option<u8> {
    match escape {
        b'\\' => Some(b'\\'),
        b'"' => Some(b'"'),
        b't' => Some(b'\t'),
        b'n' => Some(b'\n'),
        b'r' => Some(b'\r'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_git_diff_splits_multi_file_git_diff() {
        let raw = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1,1 +1,1 @@\n-old\n+new\ndiff --git a/b.rs b/b.rs\n--- a/b.rs\n+++ b/b.rs\n@@ -5,1 +5,2 @@ fn b()\n keep\n+more\n";

        let files = parse_git_diff(raw);

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "a.rs");
        assert_eq!(files[0].additions, 1);
        assert_eq!(files[0].deletions, 1);
        assert_eq!(files[1].path, "b.rs");
        assert_eq!(files[1].additions, 1);
        assert_eq!(files[1].deletions, 0);
        assert_eq!(files[1].hunks[0].context_label, "fn b()");
    }

    #[test]
    fn parse_git_diff_deleted_file_uses_old_path_when_new_path_is_dev_null() {
        let raw = "diff --git a/old.rs b/old.rs\ndeleted file mode 100644\n--- a/old.rs\n+++ /dev/null\n@@ -1,1 +0,0 @@\n-gone\n";

        let files = parse_git_diff(raw);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "old.rs");
        assert_eq!(files[0].additions, 0);
        assert_eq!(files[0].deletions, 1);
    }

    #[test]
    fn parse_git_diff_accepts_single_file_unified_diff_without_git_header() {
        let raw = "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1 @@\n-old\n+new\n";

        let files = parse_git_diff(raw);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/lib.rs");
        assert_eq!(files[0].hunks.len(), 1);
    }

    #[test]
    fn parse_git_diff_keeps_binary_section_path_without_hunks() {
        let raw = "diff --git a/assets/logo.png b/assets/logo.png\nindex 1111111..2222222 100644\nBinary files a/assets/logo.png and b/assets/logo.png differ\n";

        let files = parse_git_diff(raw);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "assets/logo.png");
        assert!(files[0].hunks.is_empty());
        assert_eq!(files[0].additions, 0);
        assert_eq!(files[0].deletions, 0);
    }

    #[test]
    fn parse_git_diff_keeps_quoted_binary_section_path() {
        let raw = "diff --git \"a/assets/site logo.png\" \"b/assets/site logo.png\"\nindex 1111111..2222222 100644\nBinary files \"a/assets/site logo.png\" and \"b/assets/site logo.png\" differ\n";

        let files = parse_git_diff(raw);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "assets/site logo.png");
        assert!(files[0].hunks.is_empty());
    }

    #[test]
    fn parse_git_diff_ignores_malformed_input_without_file_paths() {
        let files = parse_git_diff("not a diff\n@@ -1 +1 @@\n-old\n+new\n");

        assert!(files.is_empty());
    }

    #[test]
    fn parse_git_diff_decodes_octal_escaped_quoted_paths() {
        let raw = "diff --git \"a/src/caf\\303\\251.rs\" \"b/src/caf\\303\\251.rs\"\n--- \"a/src/caf\\303\\251.rs\"\n+++ \"b/src/caf\\303\\251.rs\"\n@@ -1,1 +1,1 @@\n-old\n+new\n";

        let files = parse_git_diff(raw);

        assert_eq!(files.len(), 1);
        assert_eq!(files.first().map(|f| f.path.as_str()), Some("src/café.rs"));
    }

    #[test]
    fn parse_git_diff_decodes_quoted_path_from_diff_git_header_fallback() {
        let raw =
            "diff --git \"a/na\\303\\257ve.png\" \"b/na\\303\\257ve.png\"\nBinary files differ\n";

        let files = parse_git_diff(raw);

        assert_eq!(files.len(), 1);
        assert_eq!(files.first().map(|f| f.path.as_str()), Some("naïve.png"));
    }

    #[test]
    fn decode_git_quoted_handles_simple_escapes_and_keeps_unknown_verbatim() {
        assert_eq!(decode_git_quoted("a\\tb"), "a\tb");
        assert_eq!(decode_git_quoted("a\\\\b"), "a\\b");
        assert_eq!(decode_git_quoted("a\\\"b"), "a\"b");
        assert_eq!(decode_git_quoted("a\\zb"), "a\\zb");
        assert_eq!(decode_git_quoted("trailing\\"), "trailing\\");
    }

    #[test]
    fn decode_git_quoted_rejects_out_of_range_octal_as_literal() {
        // NOTE: `\777` = 511 exceeds a byte; kept verbatim, not wrapped.
        assert_eq!(decode_git_quoted("a\\777b"), "a\\777b");
    }

    #[test]
    fn parse_git_diff_decodes_escaped_quote_inside_quoted_diff_git_header_path() {
        // WHY: An escaped quote (`\"`) inside a C-quoted path must not
        // terminate the token early — the closing `"` is the first
        // *unescaped* quote, not the first raw `"` byte (#181).
        let raw =
            "diff --git \"a/say \\\"hi\\\".txt\" \"b/say \\\"hi\\\".txt\"\nBinary files differ\n";

        let files = parse_git_diff(raw);

        assert_eq!(files.len(), 1);
        assert_eq!(
            files.first().map(|f| f.path.as_str()),
            Some("say \"hi\".txt")
        );
    }

    #[test]
    fn parse_git_diff_ignores_unterminated_quoted_diff_git_header_path() {
        // WHY: An opening `"` with no closing quote must not be
        // mis-parsed into a path that still carries the stray quote
        // (#181).
        let raw = "diff --git \"a/broken.txt b/broken.txt\nBinary files differ\n";

        let files = parse_git_diff(raw);

        assert!(files.is_empty());
    }

    #[test]
    fn parse_git_diff_resolves_unquoted_diff_git_header_path_containing_space() {
        // WHY: `diff --git a/my file.txt b/my file.txt` is unquoted
        // (git only C-quotes on special bytes) but the filename has a
        // space, ambiguous under a naive whitespace split. The
        // `a/X b/X` same-path heuristic recovers the full name when
        // no `---`/`+++` lines are present to arbitrate (#181).
        let raw = "diff --git a/my file.txt b/my file.txt\nold mode 100644\nnew mode 100755\n";

        let files = parse_git_diff(raw);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "my file.txt");
        assert!(files[0].hunks.is_empty());
    }

    #[test]
    fn normalize_diff_path_strips_tab_separated_timestamp() {
        // WHY: `---`/`+++` headers may carry a tab-separated timestamp
        // suffix (`path\t<timestamp>`); it must be stripped rather
        // than folded into the reported path (#181).
        let result = normalize_diff_path("a/src/lib.rs\t2024-01-01 00:00:00.000000000 +0000");
        assert_eq!(result, Some("src/lib.rs".to_string()));
    }

    #[test]
    fn parse_git_diff_strips_tab_separated_timestamp_from_diff_headers() {
        let raw = "--- a/src/lib.rs\t2024-01-01 00:00:00.000000000 +0000\n+++ b/src/lib.rs\t2024-01-02 00:00:00.000000000 +0000\n@@ -1,1 +1,1 @@\n-old\n+new\n";

        let files = parse_git_diff(raw);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/lib.rs");
    }

    #[test]
    fn parse_git_diff_resolves_new_path_for_rename_with_modification() {
        // WHY: A rename-with-content-change diff carries two distinct,
        // both-real paths (old_name.rs / new_name.rs, neither
        // /dev/null) via the `diff --git` header, `rename from`/`rename
        // to`, and `---`/`+++`. The `+++` header must win, resolving to
        // the new path (#181).
        let raw = "diff --git a/old_name.rs b/new_name.rs\nsimilarity index 87%\nrename from old_name.rs\nrename to new_name.rs\nindex abc123..def456 100644\n--- a/old_name.rs\n+++ b/new_name.rs\n@@ -1,2 +1,2 @@\n context\n-old content\n+new content\n";

        let files = parse_git_diff(raw);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "new_name.rs");
        assert_eq!(files[0].additions, 1);
        assert_eq!(files[0].deletions, 1);
    }
}
