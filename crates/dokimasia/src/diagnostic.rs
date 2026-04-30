//! Lint diagnostics: file/position/severity/message tuples that the linter
//! produces and consumers render.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Severity classification for a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Lint failure — CI should fail.
    Error,
    /// Concerning but not blocking.
    Warning,
    /// Informational only.
    Info,
}

impl Severity {
    /// Short uppercase label used in human-readable output.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

/// A single lint finding tied to a source location.
///
/// Positions use 1-indexed line and column counts so they line up with
/// editor and `rustc`-style diagnostic conventions. `byte_offset` and
/// `byte_len` allow renderers (e.g. codespan-reporting) to highlight the
/// exact span without recomputing it from line/column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Source file the finding came from.
    pub file: PathBuf,
    /// 1-indexed line number.
    pub line: u32,
    /// 1-indexed column (in bytes from line start).
    pub column: u32,
    /// Byte offset from the file start where the finding begins.
    pub byte_offset: usize,
    /// Length of the finding span in bytes.
    pub byte_len: usize,
    /// How serious the finding is.
    pub severity: Severity,
    /// Stable lint code (e.g. `"undocumented-token"`) for filtering.
    pub code: String,
    /// Human-readable description of what's wrong.
    pub message: String,
    /// The token name involved, if applicable (e.g. `"--accent-muted"`).
    pub token: Option<String>,
}

impl Diagnostic {
    /// Construct an undocumented-token error for a specific token reference.
    #[must_use]
    pub fn undocumented_token(
        file: PathBuf,
        line: u32,
        column: u32,
        byte_offset: usize,
        byte_len: usize,
        token: String,
    ) -> Self {
        let message = format!("token `{token}` is not declared in DESIGN-TOKENS.md");
        Self {
            file,
            line,
            column,
            byte_offset,
            byte_len,
            severity: Severity::Error,
            code: "undocumented-token".to_string(),
            message,
            token: Some(token),
        }
    }

    /// Construct a file-level warning (e.g. read failure, walker error).
    /// Position is set to (1, 1) and span is empty since there is no
    /// in-file position to point at.
    #[must_use]
    pub fn file_warning(file: PathBuf, message: String) -> Self {
        Self {
            file,
            line: 1,
            column: 1,
            byte_offset: 0,
            byte_len: 0,
            severity: Severity::Warning,
            code: "file-read-error".to_string(),
            message,
            token: None,
        }
    }

    /// Construct a forbidden-patch-block error for a workspace or crate
    /// `Cargo.toml` that contains a `[patch.crates-io]` table. Patches against
    /// external deps must live in fleet forks under `forkwright/`; workspace
    /// patch blocks bit-rot and obscure the dep graph.
    #[must_use]
    pub fn forbidden_patch_block(
        file: PathBuf,
        line: u32,
        column: u32,
        byte_offset: usize,
        byte_len: usize,
    ) -> Self {
        Self {
            file,
            line,
            column,
            byte_offset,
            byte_len,
            severity: Severity::Error,
            code: "forbidden-patch-block".to_string(),
            message: "Cargo.toml `[patch.crates-io]` block forbidden — patches stay in forkwright forks per fleet doctrine; remove the block and pin the fleet fork directly via git URL".to_string(),
            token: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_labels_are_lowercase() {
        assert_eq!(Severity::Error.label(), "error");
        assert_eq!(Severity::Warning.label(), "warning");
        assert_eq!(Severity::Info.label(), "info");
    }

    #[test]
    fn undocumented_token_message_includes_name() {
        let d = Diagnostic::undocumented_token(
            PathBuf::from("a.css"),
            1,
            1,
            0,
            14,
            "--accent-muted".to_string(),
        );
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.code, "undocumented-token");
        assert!(d.message.contains("--accent-muted"));
        assert_eq!(d.token.as_deref(), Some("--accent-muted"));
    }

    #[test]
    fn forbidden_patch_block_is_error_with_fleet_message() {
        let d = Diagnostic::forbidden_patch_block(PathBuf::from("Cargo.toml"), 6, 1, 50, 19);
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.code, "forbidden-patch-block");
        assert!(d.message.contains("forkwright"));
        assert!(d.token.is_none());
    }

    #[test]
    fn diagnostic_serde_roundtrip() {
        let d = Diagnostic::undocumented_token(
            PathBuf::from("a.css"),
            10,
            5,
            42,
            14,
            "--shadow-md".to_string(),
        );
        let json = serde_json::to_string(&d).expect("serialize");
        let back: Diagnostic = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, d);
    }
}
