//! Git-style multi-file diff parsing.

use super::{DiffFile, parse_unified_diff};

/// Parse a git-style multi-file diff into one [`DiffFile`] per file.
///
/// This reuses [`parse_unified_diff`] for each file section, preserving the
/// existing gramma diff model and word-level diff behavior. File paths are
/// derived from `+++` / `---` headers when present, with the `diff --git`
/// header used as a fallback for binary or metadata-only sections.
#[must_use]
// kanon:ignore RUST/pub-visibility -- re-exported gramma git-diff parser for external renderer crates
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
    let mut paths = diff_git_path_tokens(header);
    let old_path = paths.next().and_then(normalize_diff_path);
    let new_path = paths.next().and_then(normalize_diff_path);

    new_path.or(old_path)
}

fn diff_git_path_tokens(header: &str) -> impl Iterator<Item = &str> {
    let mut tokens = Vec::new();
    let mut rest = header.trim_start();

    while !rest.is_empty() {
        if let Some(quoted) = rest.strip_prefix('"') {
            if let Some(end) = quoted.find('"') {
                if let (Some(token), Some(remainder)) = (rest.get(..end + 2), quoted.get(end + 1..))
                {
                    tokens.push(token);
                    rest = remainder.trim_start();
                } else {
                    break;
                }
            } else {
                tokens.push(rest);
                break;
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

fn normalize_diff_path(path: &str) -> Option<String> {
    let path = path
        .split_once('\t')
        .map_or(path, |(path, _metadata)| path)
        .trim();

    if path == "/dev/null" || path.is_empty() {
        return None;
    }

    let path = path
        .strip_prefix('"')
        .and_then(|path| path.strip_suffix('"'))
        .unwrap_or(path);
    let path = path
        .strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path);

    Some(path.to_string())
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
}
