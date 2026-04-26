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
//! - [`Diagnostic`] carries file/line/column/severity/code/message and can
//!   be rendered as human-readable diagnostics (codespan-reporting) or JSON.

#![warn(missing_docs, clippy::all, clippy::pedantic)]
#![forbid(unsafe_code)]

mod css;
mod diagnostic;
mod linter;
mod rust;
mod tokens;

pub use diagnostic::{Diagnostic, Severity};
pub use linter::{LintConfig, Linter};
pub use tokens::TokenRegistry;

/// Errors returned by linter setup and file IO.
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    /// I/O failure reading a source file or the spec.
    #[snafu(display("failed to read {}: {source}", path.display()))]
    Io {
        /// Path that failed to read.
        path: std::path::PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// File walker failure.
    #[snafu(display("failed to walk path {}: {source}", path.display()))]
    Walk {
        /// Root path being walked.
        path: std::path::PathBuf,
        /// Underlying walker error.
        source: ignore::Error,
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
