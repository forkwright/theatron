//! Linter orchestrator + path walker.
//!
//! Bundles a [`TokenRegistry`] with a [`LintConfig`] and dispatches files
//! to the CSS or Rust scanner based on extension.
//!
//! Per-file IO failures (invalid UTF-8, permission denied, walker errors,
//! etc.) emit `Severity::Warning` diagnostics rather than aborting the
//! whole walk. Without that policy, one inaccessible file would discard
//! every diagnostic collected before it — caught by QA swarm A03.

use std::ffi::OsStr;
use std::path::Path;

use ignore::WalkBuilder;

use crate::css::lint_css;
use crate::diagnostic::Diagnostic;
use crate::rust::lint_rust;
use crate::tokens::TokenRegistry;

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
    /// IO failures (file not readable, permission denied) and invalid
    /// UTF-8 are reported as `Severity::Warning` diagnostics with code
    /// `"file-read-error"`. The function never returns `Err` — partial
    /// results are always preferable to aborting a whole lint run.
    #[must_use]
    pub fn lint_file(&self, path: &Path) -> Vec<Diagnostic> {
        let ext = path.extension().and_then(OsStr::to_str);
        match ext {
            Some("css") => self.read_and_scan(path, lint_css),
            Some("rs") => self.read_and_scan(path, lint_rust),
            _ => Vec::new(),
        }
    }

    /// Recursively lint every CSS and Rust file under `path`. If `path`
    /// is a file, it is linted directly.
    ///
    /// Walker errors (permission denied, dangling symlink, etc.) emit
    /// `Severity::Warning` diagnostics and the walk continues.
    #[must_use]
    pub fn lint_path(&self, path: &Path) -> Vec<Diagnostic> {
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
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_some_and(|ft| ft.is_file()) {
                        diagnostics.extend(self.lint_file(entry.path()));
                    }
                }
                Err(e) => {
                    diagnostics.push(Diagnostic::file_warning(
                        path.to_path_buf(),
                        format!("walker error: {e}"),
                    ));
                }
            }
        }
        // Stable order: errors-and-warnings sorted alongside undoc-token
        // findings by file path then byte offset.
        diagnostics.sort_by(|a, b| a.file.cmp(&b.file).then(a.byte_offset.cmp(&b.byte_offset)));
        diagnostics
    }

    /// Read `path` and run `scan` on the contents.
    ///
    /// Uses `from_utf8_lossy` so files with invalid byte sequences scan
    /// correctly for the valid prefix/suffix instead of failing the whole
    /// run. IO errors become `Severity::Warning` diagnostics on the
    /// returned vector.
    fn read_and_scan(
        &self,
        path: &Path,
        scan: fn(&TokenRegistry, &str, &Path) -> Vec<Diagnostic>,
    ) -> Vec<Diagnostic> {
        match std::fs::read(path) {
            Ok(bytes) => {
                let source = String::from_utf8_lossy(&bytes);
                scan(&self.registry, &source, path)
            }
            Err(e) => vec![Diagnostic::file_warning(
                path.to_path_buf(),
                format!("read failed: {e}"),
            )],
        }
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
        let diags = linter.lint_file(&path);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].token.as_deref(), Some("--bad"));
    }

    #[test]
    fn lint_file_unknown_extension_skips() {
        let dir = tempdir();
        let path = dir.join("a.txt");
        std::fs::write(&path, "var(--bad)").unwrap();
        let linter = Linter::new(registry());
        let diags = linter.lint_file(&path);
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
        let diags = linter.lint_path(&dir);
        assert_eq!(diags.len(), 2);
        assert!(diags[0].file.ends_with("a.css"));
        assert!(diags[1].file.ends_with("z.css"));
    }

    #[test]
    fn lint_path_handles_single_file_argument() {
        let dir = tempdir();
        let path = dir.join("a.css");
        std::fs::write(&path, "div { color: var(--bad); }").unwrap();
        let linter = Linter::new(registry());
        let diags = linter.lint_path(&path);
        assert_eq!(diags.len(), 1);
    }

    // ---- Graceful per-file errors (caught by QA swarm A03 H-01, M-13) ---

    #[test]
    fn invalid_utf8_does_not_abort_walk() {
        let dir = tempdir();
        let bad = dir.join("bad.css");
        // Valid prefix + invalid UTF-8 byte + valid suffix.
        std::fs::write(&bad, b"div { color: red; }\xff\xfevar(--bad);").unwrap();
        let good = dir.join("good.css");
        std::fs::write(&good, "div { color: var(--alsobad); }").unwrap();

        let linter = Linter::new(registry());
        let diags = linter.lint_path(&dir);
        // Both files contribute findings; --bad and --alsobad both
        // surface (the invalid bytes were lossy-decoded).
        let tokens: Vec<_> = diags.iter().filter_map(|d| d.token.clone()).collect();
        assert!(
            tokens.contains(&"--alsobad".to_string()),
            "good file's finding must survive bad file: {tokens:?}"
        );
    }

    #[test]
    fn missing_file_reports_warning_not_panic() {
        let dir = tempdir();
        let absent = dir.join("absent.css");
        let linter = Linter::new(registry());
        let diags = linter.lint_file(&absent);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, crate::diagnostic::Severity::Warning);
        assert_eq!(diags[0].code, "file-read-error");
    }

    fn tempdir() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("dokimasia-test-{}", std::process::id()));
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
