//! `theatron-lint` CLI entry point.
//!
//! Exit codes:
//! - 0 — no error-severity diagnostics
//! - 1 — at least one error-severity diagnostic
//! - 2 — invocation or I/O failure

#![warn(clippy::all, clippy::pedantic)]
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use codespan_reporting::diagnostic::{Diagnostic as CdDiagnostic, Label, Severity as CdSeverity};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::{
    self,
    termcolor::{ColorChoice, StandardStream},
};

use theatron_lint::{Diagnostic, LintConfig, Linter, Severity, TokenRegistry};

#[derive(Debug, Parser)]
#[command(
    name = "theatron-lint",
    about = "Design-token enforcement linter — fails CI on tokens not declared in DESIGN-TOKENS.md.",
    version
)]
struct Args {
    /// Files or directories to lint.
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    /// Path to the canonical `DESIGN-TOKENS.md` spec.
    #[arg(long, env = "THEATRON_LINT_TOKENS")]
    design_tokens: PathBuf,

    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Human)]
    format: Format,

    /// Do not respect `.gitignore` when walking directories.
    #[arg(long)]
    no_gitignore: bool,

    /// Suppress the summary line at the end of output.
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
            eprintln!("theatron-lint: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(args: &Args) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let registry = TokenRegistry::from_design_tokens_md(&args.design_tokens)?;
    if registry.is_empty() {
        return Err(format!(
            "no tokens parsed from spec {} — refusing to run (would flag everything)",
            args.design_tokens.display()
        )
        .into());
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
        Format::Human => render_human(&all_diagnostics)?,
        Format::Json => render_json(&all_diagnostics)?,
    }

    if !args.quiet && matches!(args.format, Format::Human) {
        let registry_size = linter.registry().len();
        eprintln!(
            "theatron-lint: {error_count} error(s), {warning_count} warning(s) across {} file(s) (registry: {registry_size} tokens)",
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

fn render_human(diagnostics: &[Diagnostic]) -> Result<(), Box<dyn std::error::Error>> {
    if diagnostics.is_empty() {
        return Ok(());
    }
    let mut files = SimpleFiles::new();
    let mut file_ids: HashMap<PathBuf, usize> = HashMap::new();
    for d in diagnostics {
        if !file_ids.contains_key(&d.file) {
            // Use lossy decode + read() so the renderer agrees with the
            // linter's view of invalid-UTF-8 files. read_to_string would
            // fail and produce an empty source, breaking the codespan
            // label (caught by QA wave 2 #13 R-02).
            let source = std::fs::read(&d.file)
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .unwrap_or_default();
            let id = files.add(d.file.display().to_string(), source);
            file_ids.insert(d.file.clone(), id);
        }
    }
    let writer = StandardStream::stderr(ColorChoice::Auto);
    let config = term::Config::default();
    for d in diagnostics {
        let file_id = file_ids[&d.file];
        let severity = match d.severity {
            Severity::Error => CdSeverity::Error,
            Severity::Warning => CdSeverity::Warning,
            Severity::Info => CdSeverity::Note,
        };
        let cd = CdDiagnostic::new(severity)
            .with_message(&d.message)
            .with_code(&d.code)
            .with_labels(vec![Label::primary(
                file_id,
                d.byte_offset..d.byte_offset + d.byte_len,
            )]);
        term::emit(&mut writer.lock(), &config, &files, &cd)?;
    }
    Ok(())
}

fn render_json(diagnostics: &[Diagnostic]) -> Result<(), serde_json::Error> {
    let json = serde_json::to_string_pretty(diagnostics)?;
    println!("{json}");
    Ok(())
}
