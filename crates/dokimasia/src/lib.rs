//! Design-token enforcement linter.
//!
//! Scans CSS and Rust source files for `var(--token)` references and
//! verifies that each token is declared in the canonical `DESIGN-TOKENS.md`
//! specification. Undeclared tokens are reported as errors so CI fails
//! before design-token drift compounds.
//!
//! ## Architecture
//!
//! - [`TokenRegistry`] parses `DESIGN-TOKENS.md` (markdown tables) and
//!   collects every backtick-wrapped token name as the source of truth.
//! - The CSS scanner walks `var(--*)` references via regex with byte-offset
//!   line/column tracking.
//! - The Rust scanner uses `syn` to parse source into an AST, walks every
//!   string literal (including those nested inside `rsx!` and other macro
//!   invocations), and extracts `var(--*)` patterns from each literal's
//!   contents.
//! - The manifest scanner checks every `Cargo.toml` for a
//!   `[patch.crates-io]` table and reports it as an error per fleet
//!   doctrine (patches against external deps live in forkwright forks,
//!   not workspace patch-blocks).
//! - [`Diagnostic`] carries file/line/column/severity/code/message and can
//!   be rendered as human-readable diagnostics (codespan-reporting) or JSON.

#![deny(missing_docs, clippy::all, clippy::pedantic)]
#![forbid(unsafe_code)]

pub mod cli;
mod css;
mod diagnostic;
mod linter;
mod manifest;
mod render;
mod rust;
mod tokens;

pub use diagnostic::{Diagnostic, Severity};
pub use linter::{LintConfig, Linter};
pub use render::{lossy_loader, render_human, render_json};
pub use tokens::TokenRegistry;

/// Errors returned by linter setup. Per-file IO failures are reported as
/// `Severity::Warning` diagnostics rather than `Err` (so a single bad file
/// can't abort an entire walk — see [`Linter::lint_path`]).
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum Error {
    /// I/O failure reading the spec file (`DESIGN-TOKENS.md`).
    #[snafu(display("failed to read {}: {source}", path.display()))]
    Io {
        /// Path that failed to read.
        path: std::path::PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Markdown spec parse error.
    #[snafu(display("failed to parse spec {}: {message}", path.display()))]
    Spec {
        /// Spec path.
        path: std::path::PathBuf,
        /// Why parsing failed.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn error_display_includes_path_and_context() {
        let spec = Error::Spec {
            path: std::path::PathBuf::from("DESIGN-TOKENS.md"),
            message: "no tables found".to_string(),
        };
        let rendered = spec.to_string();
        assert!(rendered.contains("DESIGN-TOKENS.md"), "got: {rendered}");
        assert!(rendered.contains("no tables found"), "got: {rendered}");

        let io = Error::Io {
            path: std::path::PathBuf::from("missing.md"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "gone"),
        };
        let rendered = io.to_string();
        assert!(rendered.contains("missing.md"), "got: {rendered}");
        assert!(rendered.contains("gone"), "got: {rendered}");
    }
}
