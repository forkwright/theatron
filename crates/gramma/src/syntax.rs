//! File-extension to syntect language token resolution.
//!
//! Companion to [`highlight::detect_language`](crate::highlight::detect_language)
//! (which parses Markdown fenced-code-block info strings). These
//! helpers map a file *path* or its bare *extension* to the syntect
//! language token used by [`highlight::highlight_code`](crate::highlight::highlight_code),
//! so consumers rendering file contents (file viewers, diff views)
//! don't hand-roll an extension table per call site.
//!
//! Unknown extensions return `"text"` — the syntect plain-text
//! fallback that [`highlight_code`](crate::highlight::highlight_code)
//! already recognizes.

/// Map a file path to a syntect language token using its extension.
///
/// Splits on the rightmost `.`, then delegates to
/// [`language_from_extension`]. Paths without an extension (or with
/// an unrecognized extension) return `"text"`.
///
/// # Examples
///
/// ```
/// use gramma::syntax::language_from_path;
///
/// assert_eq!(language_from_path("src/lib.rs"), "rust");
/// assert_eq!(language_from_path("README.md"), "markdown");
/// assert_eq!(language_from_path("Dockerfile"), "text");
/// assert_eq!(language_from_path("no-extension"), "text");
/// ```
#[must_use]
// kanon:ignore RUST/pub-visibility -- public file-viewer syntax helper for external renderer crates
pub fn language_from_path(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    language_from_extension(ext)
}

/// Map a bare file extension (without leading `.`) to a syntect
/// language token.
///
/// Returns `"text"` for any unrecognized extension. The extension is
/// matched as-is (case-sensitive) against a curated table covering
/// the common languages found in fleet-desktop file viewers.
///
/// # Examples
///
/// ```
/// use gramma::syntax::language_from_extension;
///
/// assert_eq!(language_from_extension("rs"), "rust");
/// assert_eq!(language_from_extension("tsx"), "tsx");
/// assert_eq!(language_from_extension("yml"), "yaml");
/// assert_eq!(language_from_extension("xyz"), "text");
/// ```
#[must_use]
// kanon:ignore RUST/pub-visibility -- public file-viewer syntax helper for external renderer crates
pub fn language_from_extension(ext: &str) -> &'static str {
    match ext {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "tsx" => "tsx",
        "jsx" => "jsx",
        "rb" => "ruby",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "cs" => "cs",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "sh" | "bash" => "bash",
        "fish" => "fish",
        "zsh" => "zsh",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "xml" => "xml",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "sql" => "sql",
        "md" | "markdown" => "markdown",
        "lua" => "lua",
        "r" | "R" => "r",
        "zig" => "zig",
        _ => "text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_from_path_resolves_canonical_rust() {
        assert_eq!(language_from_path("src/lib.rs"), "rust");
        assert_eq!(language_from_path("foo.rs"), "rust");
    }

    #[test]
    fn language_from_path_resolves_markdown() {
        assert_eq!(language_from_path("README.md"), "markdown");
        assert_eq!(language_from_path("docs/intro.markdown"), "markdown");
    }

    #[test]
    fn language_from_path_resolves_typescript_variants_distinctly() {
        // tsx and jsx are distinct syntect tokens — preserve them
        // rather than collapsing to typescript / javascript.
        assert_eq!(language_from_path("App.tsx"), "tsx");
        assert_eq!(language_from_path("Component.jsx"), "jsx");
        assert_eq!(language_from_path("foo.ts"), "typescript");
        assert_eq!(language_from_path("foo.js"), "javascript");
    }

    #[test]
    fn language_from_path_handles_no_extension_as_text() {
        assert_eq!(language_from_path("Dockerfile"), "text");
        assert_eq!(language_from_path("Makefile"), "text");
        assert_eq!(language_from_path("no-extension"), "text");
    }

    #[test]
    fn language_from_path_handles_dot_only() {
        assert_eq!(language_from_path("."), "text");
        assert_eq!(language_from_path(".gitignore"), "text");
    }

    #[test]
    fn language_from_path_takes_rightmost_extension() {
        assert_eq!(language_from_path("archive.tar.gz"), "text");
        assert_eq!(language_from_path("backup.20260508.json"), "json");
    }

    #[test]
    fn language_from_extension_alias_groups_resolve_consistently() {
        assert_eq!(language_from_extension("yaml"), "yaml");
        assert_eq!(language_from_extension("yml"), "yaml");
        assert_eq!(language_from_extension("c"), "c");
        assert_eq!(language_from_extension("h"), "c");
        assert_eq!(language_from_extension("cpp"), "cpp");
        assert_eq!(language_from_extension("cc"), "cpp");
        assert_eq!(language_from_extension("cxx"), "cpp");
        assert_eq!(language_from_extension("hpp"), "cpp");
        assert_eq!(language_from_extension("kt"), "kotlin");
        assert_eq!(language_from_extension("kts"), "kotlin");
        assert_eq!(language_from_extension("sh"), "bash");
        assert_eq!(language_from_extension("bash"), "bash");
        assert_eq!(language_from_extension("html"), "html");
        assert_eq!(language_from_extension("htm"), "html");
        assert_eq!(language_from_extension("md"), "markdown");
        assert_eq!(language_from_extension("markdown"), "markdown");
    }

    #[test]
    fn language_from_extension_unknown_returns_text() {
        assert_eq!(language_from_extension("xyz"), "text");
        assert_eq!(language_from_extension(""), "text");
        assert_eq!(language_from_extension("unknown-language"), "text");
    }

    #[test]
    fn language_from_extension_case_sensitive() {
        // Match real syntect tokens; consumers normalize before
        // calling if they want case-insensitive matching.
        assert_eq!(language_from_extension("RS"), "text");
        assert_eq!(language_from_extension("Py"), "text");
        // R is intentionally distinct from r — both map to "r".
        assert_eq!(language_from_extension("R"), "r");
        assert_eq!(language_from_extension("r"), "r");
    }
}
