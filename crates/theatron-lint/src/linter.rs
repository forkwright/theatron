//! Linter orchestrator + path walker.
//!
//! Bundles a [`TokenRegistry`] with a [`LintConfig`] and dispatches files
//! to the CSS or Rust scanner based on extension.

use std::ffi::OsStr;
use std::path::Path;

use ignore::WalkBuilder;
use snafu::ResultExt;

use crate::css::lint_css;
use crate::diagnostic::Diagnostic;
use crate::rust::lint_rust;
use crate::tokens::TokenRegistry;
use crate::{Error, IoSnafu, WalkSnafu};

/// Linter configuration.
#[derive(Debug, Clone)]
pub struct LintConfig {
    /// Skip files matched by `.gitignore` (and parent gitignores).
    pub respect_gitignore: bool,
    /// Skip hidden files and directories (those whose name begins with `.`).
    pub skip_hidden: bool,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            respect_gitignore: true,
            skip_hidden: true,
        }
    }
}

/// Top-level linter handle. Cheap to construct; clone-friendly.
#[derive(Debug, Clone)]
pub struct Linter {
    registry: TokenRegistry,
    config: LintConfig,
}

impl Linter {
    /// Build a linter with default config.
    #[must_use]
    pub fn new(registry: TokenRegistry) -> Self {
        Self {
            registry,
            config: LintConfig::default(),
        }
    }

    /// Override the default [`LintConfig`].
    #[must_use]
    pub fn with_config(mut self, config: LintConfig) -> Self {
        self.config = config;
        self
    }

    /// Borrow the underlying token registry.
    #[must_use]
    pub fn registry(&self) -> &TokenRegistry {
        &self.registry
    }

    /// Lint a single file by extension dispatch. CSS and Rust files are
    /// scanned; other extensions return an empty diagnostic list.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the file cannot be read.
    pub fn lint_file(&self, path: &Path) -> Result<Vec<Diagnostic>, Error> {
        let ext = path.extension().and_then(OsStr::to_str);
        match ext {
            Some("css") => {
                let source = std::fs::read_to_string(path).context(IoSnafu { path })?;
                Ok(lint_css(&self.registry, &source, path))
            }
            Some("rs") => {
                let source = std::fs::read_to_string(path).context(IoSnafu { path })?;
                Ok(lint_rust(&self.registry, &source, path))
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Recursively lint every CSS and Rust file under `path`. If `path`
    /// is a file, it is linted directly.
    ///
    /// # Errors
    ///
    /// Returns the first walker or I/O error encountered. Diagnostics
    /// from successfully scanned files prior to the error are discarded —
    /// callers needing partial results should walk files themselves.
    pub fn lint_path(&self, path: &Path) -> Result<Vec<Diagnostic>, Error> {
        if path.is_file() {
            return self.lint_file(path);
        }
        let walker = WalkBuilder::new(path)
            .hidden(self.config.skip_hidden)
            .git_ignore(self.config.respect_gitignore)
            .git_global(self.config.respect_gitignore)
            .git_exclude(self.config.respect_gitignore)
            .build();
        let mut diagnostics = Vec::new();
        for entry in walker {
            let entry = entry.context(WalkSnafu { path })?;
            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                diagnostics.extend(self.lint_file(entry.path())?);
            }
        }
        // Stable order: by file path, then byte offset.
        diagnostics.sort_by(|a, b| a.file.cmp(&b.file).then(a.byte_offset.cmp(&b.byte_offset)));
        Ok(diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> TokenRegistry {
        TokenRegistry::from_tokens(["--bg", "--accent"])
    }

    #[test]
    fn lint_file_css_finds_undocumented() {
        let dir = tempdir();
        let path = dir.join("a.css");
        std::fs::write(&path, "div { color: var(--bad); }").unwrap();
        let linter = Linter::new(registry());
        let diags = linter.lint_file(&path).unwrap();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].token.as_deref(), Some("--bad"));
    }

    #[test]
    fn lint_file_unknown_extension_skips() {
        let dir = tempdir();
        let path = dir.join("a.txt");
        std::fs::write(&path, "var(--bad)").unwrap();
        let linter = Linter::new(registry());
        let diags = linter.lint_file(&path).unwrap();
        assert!(diags.is_empty());
    }

    #[test]
    fn lint_path_recurses_and_sorts() {
        let dir = tempdir();
        std::fs::write(dir.join("z.css"), "div { color: var(--zbad); }").unwrap();
        let nested = dir.join("nested");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(nested.join("a.css"), "div { color: var(--abad); }").unwrap();
        let linter = Linter::new(registry());
        let diags = linter.lint_path(&dir).unwrap();
        assert_eq!(diags.len(), 2);
        // Sorted by file path: nested/a.css before z.css.
        assert!(diags[0].file.ends_with("a.css"));
        assert!(diags[1].file.ends_with("z.css"));
    }

    #[test]
    fn lint_path_handles_single_file_argument() {
        let dir = tempdir();
        let path = dir.join("a.css");
        std::fs::write(&path, "div { color: var(--bad); }").unwrap();
        let linter = Linter::new(registry());
        let diags = linter.lint_path(&path).unwrap();
        assert_eq!(diags.len(), 1);
    }

    fn tempdir() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("theatron-lint-test-{}", std::process::id()));
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
}
