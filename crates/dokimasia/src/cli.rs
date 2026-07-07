//! `dokimasia` CLI argument parsing and execution.
//!
//! Exit codes:
//! - 0 — no error-severity diagnostics
//! - 1 — at least one error-severity diagnostic
//! - 2 — invocation or I/O failure

use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

use crate::{
    Diagnostic, LintConfig, Linter, Severity, TokenRegistry, lossy_loader, render_human,
    render_json,
};

/// Errors that can occur during CLI execution.
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum RunError {
    /// Failed to read or parse the token registry.
    #[snafu(display("failed to load token registry: {source}"))]
    TokenRegistry {
        /// Underlying registry error.
        source: crate::Error,
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

impl From<crate::Error> for RunError {
    fn from(source: crate::Error) -> Self {
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

/// Command-line arguments for the `dokimasia` binary.
#[derive(Debug, Parser)]
#[command(
    name = "dokimasia",
    about = "δοκιμασία — design-token + standards enforcement. Fails CI on tokens not declared in DESIGN-TOKENS.md.",
    version
)]
pub struct Args {
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
    /// output (no effect with `--format json`, which never emits one).
    #[arg(long, short)]
    quiet: bool,
}

/// Output format for rendered diagnostics.
#[derive(Debug, Clone, ValueEnum)]
enum Format {
    /// Human-readable diagnostics with source-snippet context.
    Human,
    /// One JSON array of structured diagnostics on stdout.
    Json,
}

/// Parse the token registry and lint every path in `args`, returning the
/// linter (for registry-size reporting) alongside the collected
/// diagnostics.
///
/// Factored out of `run()` so the gate-decision test suite can drive the
/// same production path (registry parse + walk) without needing to
/// capture stdout or duplicate `run()`'s wiring.
///
/// # Errors
///
/// Returns [`RunError::TokenRegistry`] if the spec cannot be read or
/// parsed, and [`RunError::NoTokens`] if the spec parses to zero tokens
/// (refusing to run avoids flagging every token as undocumented).
fn gather_diagnostics(args: &Args) -> Result<(Linter, Vec<Diagnostic>), RunError> {
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
    Ok((linter, all_diagnostics))
}

/// Run the `dokimasia` CLI: parse the token registry, lint every requested
/// path, render diagnostics in the requested format, and compute the
/// process exit code.
///
/// # Errors
///
/// Returns [`RunError::TokenRegistry`] if the token registry spec cannot
/// be read or parsed, [`RunError::NoTokens`] if the spec parses to zero
/// tokens, [`RunError::Render`] if writing human-readable diagnostics
/// fails, and [`RunError::Json`] if diagnostic JSON serialization fails.
pub fn run(args: &Args) -> Result<ExitCode, RunError> {
    let (linter, all_diagnostics) = gather_diagnostics(args)?;

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
            // WHY: explicit writer (not println!) so the library never
            // hardcodes a print macro -- mirrors render_human's injected
            // `&mut impl Write` above.
            writeln!(std::io::stdout(), "{}", render_json(&all_diagnostics)?)?;
        }
    }

    if !args.quiet && matches!(args.format, Format::Human) {
        let registry_size = linter.registry().len();
        writeln!(
            std::io::stderr(),
            "dokimasia: {error_count} error(s), {warning_count} warning(s) across {} file(s) (registry: {registry_size} tokens)",
            files_seen(&all_diagnostics),
        )?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `ExitCode` has no `PartialEq` impl (by design, upstream) — compare
    /// via `Debug` formatting instead.
    fn assert_exit_eq(actual: ExitCode, expected: ExitCode) {
        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
    }

    fn tempdir() -> PathBuf {
        let base = std::env::temp_dir().join(format!("dokimasia-main-test-{}", std::process::id()));
        let dir = base.join(format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock is after unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("create temp fixture dir");
        dir
    }

    fn write_fixture(dir: &std::path::Path, name: &str, contents: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, contents).expect("write fixture file");
        path
    }

    fn args(design_tokens: PathBuf, paths: Vec<PathBuf>, format: Format) -> Args {
        Args {
            paths,
            design_tokens,
            format,
            no_gitignore: true,
            quiet: true,
        }
    }

    #[test]
    fn run_succeeds_when_no_undocumented_tokens() {
        let dir = tempdir();
        let tokens = write_fixture(&dir, "DESIGN-TOKENS.md", "`--bg`\n");
        let src = write_fixture(&dir, "a.css", "div { color: var(--bg); }");
        let a = args(tokens, vec![src], Format::Human);

        let exit = run(&a).expect("run must succeed on a fully-documented source");
        assert_exit_eq(exit, ExitCode::SUCCESS);
    }

    #[test]
    fn run_fails_when_error_diagnostic_present() {
        let dir = tempdir();
        let tokens = write_fixture(&dir, "DESIGN-TOKENS.md", "`--bg`\n");
        let src = write_fixture(&dir, "a.css", "div { color: var(--missing); }");
        let a = args(tokens, vec![src], Format::Human);

        let exit = run(&a).expect("run must not error out on lint findings");
        assert_exit_eq(exit, ExitCode::from(1));
    }

    #[test]
    fn run_refuses_when_spec_has_zero_tokens() {
        let dir = tempdir();
        let tokens = write_fixture(
            &dir,
            "DESIGN-TOKENS.md",
            "no backtick tokens in this spec.\n",
        );
        let src = write_fixture(&dir, "a.css", "div { color: var(--whatever); }");
        let a = args(tokens.clone(), vec![src], Format::Human);

        let err = run(&a).expect_err("a zero-token spec must refuse to run");
        assert!(
            matches!(&err, RunError::NoTokens { path } if *path == tokens),
            "expected NoTokens{{path: {tokens:?}}}, got: {err:?}"
        );
    }

    #[test]
    fn json_format_run_succeeds_and_diagnostics_round_trip_through_json() {
        let dir = tempdir();
        let tokens = write_fixture(&dir, "DESIGN-TOKENS.md", "`--bg`\n");
        let src = write_fixture(&dir, "a.css", "div { color: var(--missing); }");
        let a = args(tokens, vec![src], Format::Json);

        // Drive the same gate-decision branch `--format json` takes in
        // `run()`. `println!` inside `run()` isn't captured here — the
        // JSON *shape* is verified below via the same production
        // (registry + linter) path `run()` uses internally.
        let exit = run(&a).expect("json-format run must not error");
        assert_exit_eq(exit, ExitCode::from(1));

        let (_linter, diagnostics) =
            gather_diagnostics(&a).expect("gather_diagnostics must reuse the same fixture cleanly");
        let json = render_json(&diagnostics).expect("diagnostics must serialize to JSON");
        let parsed: Vec<Diagnostic> = serde_json::from_str(&json).expect("JSON must be parseable");
        assert_eq!(parsed, diagnostics);
        assert!(
            parsed
                .iter()
                .any(|d| d.token.as_deref() == Some("--missing")),
            "expected --missing token in parsed diagnostics: {parsed:?}"
        );
    }
}
