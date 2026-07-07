//! `dokimasia` CLI entry point.
//!
//! Exit codes:
//! - 0 — no error-severity diagnostics
//! - 1 — at least one error-severity diagnostic
//! - 2 — invocation or I/O failure

#![warn(clippy::all, clippy::pedantic)]
#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::Parser;
use dokimasia::cli::{Args, run};

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
