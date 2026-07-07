//! Token registry — the canonical set of CSS custom properties declared by
//! `DESIGN-TOKENS.md`.
//!
//! The registry collects tokens out of inline code spans and fenced code
//! blocks in the markdown spec. Anything matching `--[a-z][a-z0-9-]*` inside
//! such spans counts as a declared token name.

use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use regex::Regex;
use snafu::ResultExt;

use crate::{Error, IoSnafu};

/// Match `--token` identifiers (CSS custom property naming convention).
#[expect(
    clippy::expect_used,
    reason = "hardcoded regex compilation; failure is a programming error"
)]
fn token_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"--[a-z][a-z0-9-]*").expect("token regex compiles"))
}

/// Set of CSS custom-property tokens declared as canonical by the spec.
#[derive(Debug, Clone)]
pub struct TokenRegistry {
    documented: HashSet<String>,
}

impl TokenRegistry {
    /// Build a registry from a `DESIGN-TOKENS.md` file on disk.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the spec cannot be read.
    pub fn from_design_tokens_md(path: &Path) -> Result<Self, Error> {
        let source = std::fs::read_to_string(path).context(IoSnafu { path })?;
        Ok(Self::from_markdown(&source))
    }

    /// Build a registry from in-memory markdown source.
    #[must_use]
    pub fn from_markdown(source: &str) -> Self {
        let mut documented = HashSet::new();
        let mut in_code_block = false;
        for event in Parser::new(source) {
            match event {
                Event::Code(s) => collect(&s, &mut documented),
                Event::Start(Tag::CodeBlock(
                    CodeBlockKind::Fenced(_) | CodeBlockKind::Indented,
                )) => {
                    in_code_block = true;
                }
                Event::End(TagEnd::CodeBlock) => in_code_block = false,
                Event::Text(s) if in_code_block => collect(&s, &mut documented),
                _ => {} // kanon:ignore RUST/empty-match-arm -- headings, paragraphs, lists, and other markdown constructs do not contain declared token names
            }
        }
        Self { documented }
    }

    /// Construct a registry directly from a token list (useful for tests).
    #[must_use]
    pub fn from_tokens<I, S>(tokens: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            documented: tokens.into_iter().map(Into::into).collect(),
        }
    }

    /// Whether `token` (e.g. `"--accent"`) is in the registry.
    #[must_use]
    pub fn contains(&self, token: &str) -> bool {
        self.documented.contains(token)
    }

    /// Number of documented tokens.
    #[must_use]
    pub fn len(&self) -> usize {
        self.documented.len()
    }

    /// True if no tokens were extracted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.documented.is_empty()
    }

    /// Iterate over documented token names. Order is unspecified.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.documented.iter().map(String::as_str)
    }
}

fn collect(source: &str, sink: &mut HashSet<String>) {
    for m in token_regex().find_iter(source) {
        sink.insert(m.as_str().to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- from_design_tokens_md (the crate's only disk-reading entry
    // point) — QA #186.4 ---

    #[test]
    fn from_design_tokens_md_reads_and_parses_a_real_file() {
        let dir = tempdir();
        let path = dir.join("DESIGN-TOKENS.md");
        std::fs::write(
            &path,
            "| Token | Role |\n|---|---|\n| `--bg` | Page base |\n| `--accent` | Brass gold |\n",
        )
        .unwrap();

        let registry = TokenRegistry::from_design_tokens_md(&path).expect("read + parse");
        assert!(registry.contains("--bg"));
        assert!(registry.contains("--accent"));
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn from_design_tokens_md_reports_io_error_for_missing_file() {
        let dir = tempdir();
        let absent = dir.join("does-not-exist.md");

        let err =
            TokenRegistry::from_design_tokens_md(&absent).expect_err("missing file must error");
        match err {
            Error::Io { path, .. } => assert_eq!(path, absent),
            other => panic!("expected Error::Io, got: {other:?}"),
        }
    }

    fn tempdir() -> std::path::PathBuf {
        let base =
            std::env::temp_dir().join(format!("dokimasia-tokens-test-{}", std::process::id()));
        let dir = base.join(format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn extracts_from_inline_code_in_markdown_table() {
        let md = "\
| Token | Role |
|---|---|
| `--bg` | Page base |
| `--accent` | Brass gold |
";
        let r = TokenRegistry::from_markdown(md);
        assert!(r.contains("--bg"));
        assert!(r.contains("--accent"));
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn extracts_from_fenced_code_block() {
        let md = "\
Example:

```css
:root {
    --bg: #111;
    --text-primary: #eee;
}
```
";
        let r = TokenRegistry::from_markdown(md);
        assert!(r.contains("--bg"));
        assert!(r.contains("--text-primary"));
    }

    #[test]
    fn ignores_prose_outside_code_spans() {
        // The literal "--bg" appears in prose (not in a code span) and
        // should NOT count, because the spec convention is to wrap token
        // names in backticks.
        let md = "Use --bg for the page background.";
        let r = TokenRegistry::from_markdown(md);
        assert!(!r.contains("--bg"));
        assert!(r.is_empty());
    }

    #[test]
    fn handles_multiple_tokens_per_code_span() {
        let md = "Defaults: `--bg --bg-surface --bg-elevated`";
        let r = TokenRegistry::from_markdown(md);
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn from_tokens_constructs_directly() {
        let r = TokenRegistry::from_tokens(["--accent", "--bg"]);
        assert_eq!(r.len(), 2);
        assert!(r.contains("--accent"));
    }

    #[test]
    fn token_regex_excludes_uppercase_and_underscore() {
        let r = TokenRegistry::from_markdown("`--BAD_NAME` `--Bad`");
        // Only --b should match (everything after - lowercase only).
        // --BAD_NAME doesn't match because B is uppercase.
        // --Bad doesn't match because B is uppercase.
        assert!(r.is_empty());
    }
}
