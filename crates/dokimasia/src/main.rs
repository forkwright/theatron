//! `dokimasia` CLI entry point.
//!
//! Exit codes:
//! - 0 — no error-severity diagnostics
//! - 1 — at least one error-severity diagnostic
//! - 2 — invocation or I/O failure

#![warn(clippy::all, clippy::pedantic)]
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

use dokimasia::{
    Diagnostic, LintConfig, Linter, Severity, TokenRegistry, lossy_loader, render_human,
    render_json,
};

/// Errors that can occur during CLI execution.
#[derive(Debug, snafu::Snafu)]
enum RunError {
    /// Failed to read or parse the token registry.
    #[snafu(display("failed to load token registry: {source}"))]
    TokenRegistry {
        /// Underlying registry error.
        source: dokimasia::Error,
    },

    /// No tokens were extracted from the spec file.
    #[snafu(display(
        "no tokens parsed from spec {} -- refusing to run (would flag everything)",
        path.display()
    ))]
    NoTokens {
        /// Path to the spec file.
        path: PathBuf,
    },

    /// I/O failure writing human-readable diagnostics.
    #[snafu(display("render error: {source}"))]
    Render {
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// JSON serialization failure.
    #[snafu(display("JSON error: {source}"))]
    Json {
        /// Underlying `serde_json` error.
        source: serde_json::Error,
    },
}

impl From<dokimasia::Error> for RunError {
    fn from(source: dokimasia::Error) -> Self {
        Self::TokenRegistry { source }
    }
}

impl From<std::io::Error> for RunError {
    fn from(source: std::io::Error) -> Self {
        Self::Render { source }
    }
}

impl From<serde_json::Error> for RunError {
    fn from(source: serde_json::Error) -> Self {
        Self::Json { source }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "dokimasia",
    about = "δοκιμασία — design-token + standards enforcement. Fails CI on tokens not declared in DESIGN-TOKENS.md.",
    version
)]
struct Args {
    /// Files or directories to lint.
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    /// Path to the canonical `DESIGN-TOKENS.md` spec.
    #[arg(long, env = "DOKIMASIA_TOKENS")]
    design_tokens: PathBuf,

    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Human)]
    format: Format,

    /// Do not respect `.gitignore` when walking directories.
    #[arg(long)]
    no_gitignore: bool,

    /// Suppress the summary line printed to stderr after human-readable
    /// output (no effect with --format json).
    #[arg(long, short)]
    quiet: bool,
}

#[derive(Debug, Clone, ValueEnum)]
enum Format {
    /// Human-readable diagnostics with source-snippet context.
    Human,
    /// One JSON array of structured diagnostics on stdout.
    Json,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(exit_code) => exit_code,
        Err(e) => {
            eprintln!("dokimasia: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(args: &Args) -> Result<ExitCode, RunError> {
    let registry = TokenRegistry::from_design_tokens_md(&args.design_tokens)?;
    if registry.is_empty() {
        return Err(RunError::NoTokens {
            path: args.design_tokens.clone(),
        });
    }

    let config = LintConfig {
        respect_gitignore: !args.no_gitignore,
        ..LintConfig::default()
    };
    let linter = Linter::new(registry).with_config(config);

    let mut all_diagnostics = Vec::new();
    for path in &args.paths {
        all_diagnostics.extend(linter.lint_path(path));
    }

    let error_count = all_diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warning_count = all_diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();

    match args.format {
        Format::Human => {
            let writer = StandardStream::stderr(ColorChoice::Auto);
            render_human(&all_diagnostics, &mut writer.lock(), lossy_loader)?;
        }
        Format::Json => {
            println!("{}", render_json(&all_diagnostics)?);
        }
    }

    if !args.quiet && matches!(args.format, Format::Human) {
        let registry_size = linter.registry().len();
        eprintln!(
            "dokimasia: {error_count} error(s), {warning_count} warning(s) across {} file(s) (registry: {registry_size} tokens)",
            files_seen(&all_diagnostics),
        );
    }

    Ok(if error_count > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn files_seen(diagnostics: &[Diagnostic]) -> usize {
    use std::collections::HashSet;
    diagnostics
        .iter()
        .map(|d| &d.file)
        .collect::<HashSet<_>>()
        .len()
}
