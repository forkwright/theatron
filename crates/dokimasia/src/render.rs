//! Diagnostic rendering — human and JSON formats.
//!
//! Lives in the library (not just `main.rs`) so the rendering pipeline is
//! unit-testable. The CLI binary is a thin wrapper that selects a format
//! and writes to stderr/stdout.

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use codespan_reporting::diagnostic::{Diagnostic as CdDiagnostic, Label, Severity as CdSeverity};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::{self, Config as TermConfig, termcolor::WriteColor};

use crate::diagnostic::{Diagnostic, Severity};

/// Render a slice of diagnostics in human-readable form (codespan-reporting).
///
/// `loader` is called once per unique file path to fetch the source — the
/// CLI binary passes a closure that does `read() + from_utf8_lossy`, but
/// tests pass a closure backed by an in-memory map.
///
/// # Errors
///
/// Returns an [`io::Error`] if the underlying writer fails.
// kanon:ignore RUST/pub-visibility -- re-exported renderer used by dokimasia CLI and library consumers
pub fn render_human<W, L>(
    diagnostics: &[Diagnostic],
    writer: &mut W,
    mut loader: L,
) -> io::Result<()>
where
    W: WriteColor,
    L: FnMut(&PathBuf) -> String,
{
    if diagnostics.is_empty() {
        return Ok(());
    }
    let mut files = SimpleFiles::new();
    let mut file_ids: HashMap<PathBuf, usize> = HashMap::new();
    for d in diagnostics {
        if !file_ids.contains_key(&d.file) {
            let source = loader(&d.file);
            let id = files.add(d.file.display().to_string(), source);
            file_ids.insert(d.file.clone(), id);
        }
    }
    let config = TermConfig::default();
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
        term::emit(writer, &config, &files, &cd).map_err(|e| match e {
            codespan_reporting::files::Error::Io(io_err) => io_err,
            other => io::Error::other(other.to_string()),
        })?;
    }
    Ok(())
}

/// Read a source file using `from_utf8_lossy` so the renderer agrees
/// with the linter's view of files containing invalid UTF-8.
///
/// On read failure (file removed between lint and render, permissions
/// changed, etc.) returns a single-line sentinel describing the error
/// so the diagnostic output makes the failure visible instead of
/// silently emitting an empty source frame.
#[must_use]
// kanon:ignore RUST/pub-visibility -- re-exported source loader paired with render_human
pub fn lossy_loader(path: &PathBuf) -> String {
    match std::fs::read(path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(e) => format!("<dokimasia: failed to read {}: {e}>", path.display()),
    }
}

/// Render a slice of diagnostics as a pretty-printed JSON array.
///
/// # Errors
///
/// Returns the serde error if any field can't serialize (in practice
/// never, since `Diagnostic` is plain data).
// kanon:ignore RUST/pub-visibility -- re-exported renderer used by dokimasia CLI and library consumers
pub fn render_json(diagnostics: &[Diagnostic]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(diagnostics)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use codespan_reporting::term::termcolor::NoColor;

    use super::*;

    fn diag_undoc(
        file: &str,
        line: u32,
        col: u32,
        off: usize,
        len: usize,
        token: &str,
    ) -> Diagnostic {
        Diagnostic::undocumented_token(PathBuf::from(file), line, col, off, len, token.to_string())
    }

    #[test]
    fn render_human_writes_error_with_label() {
        let src = "div { color: var(--missing); }\n".to_string();
        let mut sources: HashMap<PathBuf, String> = HashMap::new();
        sources.insert(PathBuf::from("a.css"), src);

        let diags = vec![diag_undoc("a.css", 1, 14, 13, 9, "--missing")];
        let mut buf: Vec<u8> = Vec::new();
        let mut writer = NoColor::new(&mut buf);
        render_human(&diags, &mut writer, |p| sources[p].clone()).expect("render");
        let out = String::from_utf8(buf).expect("utf8");

        // Error-severity label, the lint code, the message, and the file path
        // all appear in the output.
        assert!(out.contains("error"), "expected severity label: {out}");
        assert!(
            out.contains("undocumented-token"),
            "expected lint code: {out}"
        );
        assert!(
            out.contains("--missing"),
            "expected token in message: {out}"
        );
        assert!(out.contains("a.css"), "expected file path: {out}");
    }

    #[test]
    fn render_human_empty_writes_nothing() {
        let mut buf: Vec<u8> = Vec::new();
        let mut writer = NoColor::new(&mut buf);
        render_human(&[], &mut writer, |_| String::new()).expect("render");
        assert!(buf.is_empty());
    }

    #[test]
    fn render_human_renders_warning_for_file_error() {
        let mut buf: Vec<u8> = Vec::new();
        let mut writer = NoColor::new(&mut buf);
        let diags = vec![Diagnostic::file_warning(
            PathBuf::from("missing.css"),
            "no such file".to_string(),
        )];
        render_human(&diags, &mut writer, |_| String::new()).expect("render");
        let out = String::from_utf8(buf).expect("utf8");
        assert!(out.contains("warning"), "expected warning label: {out}");
        assert!(out.contains("file-read-error"), "expected code: {out}");
    }

    #[test]
    fn render_json_produces_parseable_array() {
        let diags = vec![diag_undoc("a.css", 2, 5, 10, 9, "--missing")];
        let json = render_json(&diags).expect("serialize");
        // Round-trip must yield the same diagnostics.
        let parsed: Vec<Diagnostic> = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed, diags);
    }

    #[test]
    fn render_json_empty_is_valid_array() {
        let json = render_json(&[]).expect("serialize");
        assert_eq!(json.trim(), "[]");
    }

    #[test]
    fn render_json_schema_is_stable() {
        // Snapshot of the JSON shape — change this only when the schema
        // intentionally evolves. Consumers may parse the output.
        let diags = vec![diag_undoc("a.css", 1, 14, 13, 9, "--missing")];
        let json = render_json(&diags).expect("serialize");
        // Must contain every documented field name.
        for field in [
            "\"file\"",
            "\"line\"",
            "\"column\"",
            "\"byte_offset\"",
            "\"byte_len\"",
            "\"severity\"",
            "\"code\"",
            "\"message\"",
            "\"token\"",
        ] {
            assert!(
                json.contains(field),
                "field {field} missing from JSON: {json}"
            );
        }
        assert!(
            json.contains("\"severity\": \"error\""),
            "severity should serialize as lowercase: {json}"
        );
    }
}
