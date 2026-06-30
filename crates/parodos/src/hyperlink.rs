//! OSC 8 terminal hyperlink support.

use std::{path::Path, sync::LazyLock};

use regex::Regex;
use unicode_width::UnicodeWidthStr;

use crate::env::{Env, RealEnv};

/// A link found during markdown rendering, positioned within rendered lines.
///
/// Line index and column are **relative** to the `Vec<Line>` returned by
/// the consumer's markdown renderer and do **not** include any
/// content-prefix span that downstream views may prepend to each line.
#[expect(
    dead_code,
    reason = "link-position model is crate-visible until renderer wiring lands"
)]
#[derive(Debug, Clone)]
pub(crate) struct MdLink {
    /// Zero-based index into the returned `Vec<Line>`.
    pub(crate) line_idx: usize,
    /// Terminal display column within that line, content-prefix excluded.
    pub(crate) display_col: u16,
    /// Visible display text of the link (for re-rendering with OSC 8).
    pub(crate) text: String,
    /// Target URL.
    pub(crate) url: String,
}

/// A hyperlink fully resolved to absolute screen coordinates, ready to emit.
#[expect(
    dead_code,
    reason = "link-position model is crate-visible until renderer wiring lands"
)]
#[derive(Debug, Clone)]
pub(crate) struct OscLink {
    /// Absolute terminal display column (0-based).
    pub(crate) screen_x: u16,
    /// Absolute terminal row (0-based).
    pub(crate) screen_y: u16,
    /// Visible display text.
    pub(crate) text: String,
    /// Target URL.
    pub(crate) url: String,
    /// Accent colour (R, G, B) to apply when re-writing the link text.
    pub(crate) accent: (u8, u8, u8),
}

/// Format an OSC 8 hyperlink **opening** sequence.
///
/// Format: `ESC ] 8 ;; URL BEL`
#[must_use]
// kanon:ignore RUST/pub-visibility -- external TUI consumers emit OSC 8 hyperlinks
pub fn osc8_open(url: &str) -> String {
    format!("\x1b]8;;{url}\x07")
}

/// OSC 8 hyperlink **closing** sequence: `ESC ] 8 ;; BEL`
#[must_use]
// kanon:ignore RUST/pub-visibility -- external TUI consumers emit OSC 8 hyperlinks
pub const fn osc8_close() -> &'static str {
    "\x1b]8;;\x07"
}

/// Convert a byte offset in `text` to a terminal display column.
///
/// If `byte_offset` is not on a UTF-8 character boundary, it is rounded down
/// to the previous boundary. Offsets past the end of the string use the full
/// string width. The returned value saturates at `u16::MAX`.
#[must_use]
// kanon:ignore RUST/pub-visibility -- external TUI consumers position hyperlinks by terminal display column
pub fn display_column_for_byte_offset(text: &str, byte_offset: usize) -> u16 {
    let capped = byte_offset.min(text.len());
    let mut end = capped;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let prefix = text.get(..end).unwrap_or("");
    u16::try_from(UnicodeWidthStr::width(prefix)).unwrap_or(u16::MAX)
}

/// Returns `true` if the running terminal is known to support OSC 8 hyperlinks.
///
/// Detection is cached on first call (env vars are read once). Terminals that
/// support OSC 8 as of March 2026: Ghostty, iTerm2 ≥ 3.x, `WezTerm`, Kitty,
/// Windows Terminal, foot, Alacritty ≥ 0.14.
///
/// Terminals that do **not** support it: raw Linux console, most older xterm
/// derivatives. Callers should degrade gracefully (underline + colour only).
#[must_use]
// kanon:ignore RUST/pub-visibility -- external TUI consumers probe OSC 8 support
pub fn supports_hyperlinks() -> bool {
    static CACHE: LazyLock<bool> = LazyLock::new(probe_hyperlink_support);
    *CACHE
}

fn probe_hyperlink_support() -> bool {
    let env = RealEnv;

    // NOTE: TERM_PROGRAM: most reliable signal on macOS and some Linux terminals
    if let Some(prog) = env.var("TERM_PROGRAM") {
        match prog.as_str() {
            "iTerm.app" | "WezTerm" | "ghostty" | "Ghostty" | "kitty" => return true,
            _ => {
                // NOTE: unrecognized TERM_PROGRAM, continue probing other signals
            }
        }
    }

    // NOTE: Ghostty
    if env.var("GHOSTTY_BIN_DIR").is_some() || env.var("GHOSTTY_RESOURCES_DIR").is_some() {
        return true;
    }

    // NOTE: WezTerm
    if env.var("WEZTERM_EXECUTABLE").is_some() || env.var("WEZTERM_PANE").is_some() {
        return true;
    }

    // NOTE: Kitty
    if env.var("KITTY_PID").is_some() || env.var("KITTY_WINDOW_ID").is_some() {
        return true;
    }

    // NOTE: Windows Terminal
    if env.var("WT_SESSION").is_some() {
        return true;
    }

    // NOTE: foot
    if let Some(term) = env.var("TERM")
        && (term == "foot" || term == "foot-extra")
    {
        return true;
    }

    // NOTE: Alacritty
    if env.var("ALACRITTY_SOCKET").is_some() {
        return true;
    }

    false
}

/// Extract HTTP/HTTPS URLs from plain text.
///
/// Returns `(start_byte, end_byte, url_str)` tuples.
///
/// **Trailing punctuation** (`.`, `,`, `;`, `!`, `?`, `:`) is stripped.
/// A trailing `)` is stripped only when the URL contains fewer `(` than `)`.
///
/// The caller is responsible for skipping code blocks and inline code;
/// those contexts should not be passed to this function.
// kanon:ignore RUST/pub-visibility -- external TUI consumers detect URL spans for hyperlink rendering
pub fn detect_urls(text: &str) -> Vec<(usize, usize, &str)> {
    #[expect(
        clippy::expect_used,
        reason = "regex is a compile-time string literal and is always valid"
    )]
    static URL_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"https?://[^\s<>{}|\\^`\[\]'"]+"#).expect("hyperlink URL regex is valid")
    });

    let mut out = Vec::new();
    for m in URL_RE.find_iter(text) {
        let start = m.start();
        let trimmed_len = trim_trailing_punct(text.get(start..m.end()).unwrap_or(""));
        let end = start + trimmed_len;
        if end > start {
            out.push((start, end, text.get(start..end).unwrap_or("")));
        }
    }
    out
}

/// Returns the byte length of `url` with trailing punctuation stripped.
#[expect(
    clippy::indexing_slicing,
    reason = "end is decremented only after a guard `end == 0` break, so end - 1 is always in bounds"
)]
fn trim_trailing_punct(url: &str) -> usize {
    let bytes = url.as_bytes();
    let mut end = bytes.len();
    loop {
        if end == 0 {
            break;
        }
        match bytes[end - 1] {
            b'.' | b',' | b';' | b'!' | b'?' | b':' => end -= 1,
            b')' => {
                let sub = url.get(..end).unwrap_or("");
                let opens = sub.bytes().filter(|&b| b == b'(').count();
                let closes = sub.bytes().filter(|&b| b == b')').count();
                if closes > opens {
                    end -= 1;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
    end
}

/// Detect `path/to/file.rs` and `path/to/file.rs:LINE` patterns in text.
///
/// Returns `(start, end, path, url)` where `url` is a `file://` URL suitable
/// for OSC 8. Only paths with known source extensions are matched to
/// avoid false positives on arbitrary words.
#[must_use]
// kanon:ignore RUST/pub-visibility -- external TUI consumers detect file spans for hyperlink rendering
pub fn detect_file_paths(text: &str) -> Vec<(usize, usize, &str, String)> {
    #[expect(
        clippy::expect_used,
        reason = "regex is a compile-time string literal and is always valid"
    )]
    static PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
        // NOTE: crates/foo/src/bar.rs:142  or  src/foo.rs  or  ./src/foo.ts
        Regex::new(r"(?:\.{0,2}/)?(?:[a-zA-Z0-9_\-]+/)+[a-zA-Z0-9_\-]+\.[a-zA-Z]{1,6}(?::[0-9]+)?")
            .expect("file path regex is valid")
    });

    static SOURCE_EXTS: &[&str] = &[
        "rs", "ts", "tsx", "js", "jsx", "py", "go", "c", "cpp", "h", "java", "kt", "swift", "rb",
        "toml", "yaml", "yml", "json", "md", "txt",
    ];

    let mut out = Vec::new();
    for m in PATH_RE.find_iter(text) {
        let s = m.as_str();
        let base = s.split(':').next().unwrap_or(s);
        let ext = base.rsplit('.').next().unwrap_or("");
        if !SOURCE_EXTS.contains(&ext) {
            continue;
        }
        let path_only = base.trim_start_matches("./");
        let Some(url) = file_url_for_path(path_only) else {
            continue;
        };
        out.push((m.start(), m.end(), m.as_str(), url));
    }
    out
}

fn file_url_for_path(path: &str) -> Option<String> {
    let path = Path::new(path);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    Some(format!("file://{}", absolute.display()))
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "test assertions use direct indexing for clarity"
)]
mod tests {
    use super::*;

    #[test]
    fn detects_https_url() {
        let urls = detect_urls("See https://example.com for details");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://example.com");
    }

    #[test]
    fn detects_http_url() {
        let urls = detect_urls("Go to http://example.com");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "http://example.com");
    }

    #[test]
    fn strips_trailing_dot() {
        let urls = detect_urls("Visit https://example.com. More text.");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://example.com");
    }

    #[test]
    fn strips_trailing_comma() {
        let urls = detect_urls("See https://example.com, then proceed");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://example.com");
    }

    #[test]
    fn strips_trailing_semicolon() {
        let urls = detect_urls("Done https://example.com; next");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://example.com");
    }

    #[test]
    fn strips_trailing_colon() {
        let urls = detect_urls("Source: https://example.com:");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://example.com");
    }

    #[test]
    fn strips_unbalanced_closing_paren() {
        let urls = detect_urls("(see https://example.com)");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://example.com");
    }

    #[test]
    fn keeps_balanced_parens_in_url() {
        let url = "https://en.wikipedia.org/wiki/Rust_(programming_language)";
        let urls = detect_urls(url);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, url);
    }

    #[test]
    fn detects_multiple_urls() {
        let text = "Visit https://one.com and https://two.org today";
        let urls = detect_urls(text);
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0].2, "https://one.com");
        assert_eq!(urls[1].2, "https://two.org");
    }

    #[test]
    fn no_urls_returns_empty() {
        assert!(detect_urls("just plain text here").is_empty());
    }

    #[test]
    fn detects_url_with_path_and_query() {
        let urls = detect_urls("See https://docs.anthropic.com/en/docs/agents?v=2 here");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://docs.anthropic.com/en/docs/agents?v=2");
    }

    #[test]
    fn url_positions_are_correct() {
        let text = "hello https://foo.com world";
        let urls = detect_urls(text);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].0, 6);
        assert_eq!(
            text.get(urls[0].0..urls[0].1).unwrap_or(""),
            "https://foo.com"
        );
    }

    #[test]
    fn strips_multiple_trailing_chars() {
        let urls = detect_urls("See https://foo.com/bar.,");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://foo.com/bar");
    }

    #[test]
    fn osc8_open_format() {
        let seq = osc8_open("https://example.com");
        assert_eq!(seq, "\x1b]8;;https://example.com\x07");
    }

    #[test]
    fn osc8_close_format() {
        assert_eq!(osc8_close(), "\x1b]8;;\x07");
    }

    #[test]
    fn osc8_open_starts_with_esc() {
        assert!(osc8_open("https://x.com").starts_with('\x1b'));
    }

    #[test]
    fn osc8_open_ends_with_bel() {
        assert!(osc8_open("https://x.com").ends_with('\x07'));
    }

    #[test]
    fn osc8_close_ends_with_bel() {
        assert!(osc8_close().ends_with('\x07'));
    }

    #[test]
    fn display_column_for_byte_offset_counts_terminal_width() {
        assert_eq!(display_column_for_byte_offset("é中x", 0), 0);
        assert_eq!(display_column_for_byte_offset("é中x", 2), 1);
        assert_eq!(display_column_for_byte_offset("é中x", 5), 3);
        assert_eq!(display_column_for_byte_offset("é中x", 6), 4);
    }

    #[test]
    fn display_column_for_byte_offset_rounds_down_to_char_boundary() {
        assert_eq!(display_column_for_byte_offset("éx", 1), 0);
    }

    #[test]
    fn osc8_full_wraps_text() {
        let full = format!(
            "{}{}{}",
            osc8_open("https://example.com"),
            "click me",
            osc8_close()
        );
        assert_eq!(full, "\x1b]8;;https://example.com\x07click me\x1b]8;;\x07");
    }

    #[test]
    #[expect(
        unsafe_code,
        reason = "test-only env mutation in single-threaded test context"
    )]
    fn probe_detects_ghostty_resources_dir() {
        // WHY: Use the raw probe (not cached) to test detection logic.
        // SAFETY: test-only env mutation; env vars are not read concurrently here.
        unsafe { std::env::set_var("GHOSTTY_RESOURCES_DIR", "/usr/share/ghostty") };
        let result = probe_hyperlink_support();
        unsafe { std::env::remove_var("GHOSTTY_RESOURCES_DIR") };
        assert!(result, "should detect Ghostty via GHOSTTY_RESOURCES_DIR");
    }

    #[test]
    #[expect(
        unsafe_code,
        reason = "test-only env mutation in single-threaded test context"
    )]
    fn probe_detects_wezterm_pane() {
        // SAFETY: test-only env mutation; env vars are not read concurrently here.
        unsafe { std::env::set_var("WEZTERM_PANE", "1") };
        let result = probe_hyperlink_support();
        unsafe { std::env::remove_var("WEZTERM_PANE") };
        assert!(result, "should detect WezTerm via WEZTERM_PANE");
    }

    #[test]
    #[expect(
        unsafe_code,
        reason = "test-only env mutation in single-threaded test context"
    )]
    fn probe_detects_kitty_window_id() {
        // SAFETY: test-only env mutation; env vars are not read concurrently here.
        unsafe { std::env::set_var("KITTY_WINDOW_ID", "3") };
        let result = probe_hyperlink_support();
        unsafe { std::env::remove_var("KITTY_WINDOW_ID") };
        assert!(result, "should detect Kitty via KITTY_WINDOW_ID");
    }

    #[test]
    #[expect(
        unsafe_code,
        reason = "test-only env mutation in single-threaded test context"
    )]
    fn probe_detects_windows_terminal() {
        // SAFETY: test-only env mutation; env vars are not read concurrently here.
        unsafe { std::env::set_var("WT_SESSION", "some-uuid") };
        let result = probe_hyperlink_support();
        unsafe { std::env::remove_var("WT_SESSION") };
        assert!(result, "should detect Windows Terminal via WT_SESSION");
    }

    #[test]
    fn detects_rust_file_path() {
        let paths = detect_file_paths("See crates/nous/src/actor.rs:142 for details");
        assert_eq!(paths.len(), 1);
        assert!(paths[0].3.starts_with("file://"));
        assert!(paths[0].3.contains("actor.rs"));
    }

    #[test]
    fn ignores_non_source_extension() {
        // NOTE: .bin files are not in SOURCE_EXTS
        let paths = detect_file_paths("target/debug/aletheia.bin");
        assert!(paths.is_empty());
    }

    // --- Additional URL edge cases ---

    #[test]
    fn detects_url_with_fragment() {
        let urls = detect_urls("See https://docs.rs/snafu#error-handling for info");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://docs.rs/snafu#error-handling");
    }

    #[test]
    fn detects_url_with_port_number() {
        let urls = detect_urls("API at http://localhost:8080/api/v1");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "http://localhost:8080/api/v1");
    }

    #[test]
    fn strips_trailing_exclamation() {
        let urls = detect_urls("Check https://example.com!");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://example.com");
    }

    #[test]
    fn strips_trailing_question_mark() {
        let urls = detect_urls("Is https://example.com? the right one");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://example.com");
    }

    #[test]
    fn empty_text_has_no_urls() {
        assert!(
            detect_urls("").is_empty(),
            "empty string should yield no URLs"
        );
    }

    #[test]
    fn ftp_scheme_not_detected() {
        // Only http/https are matched; ftp:// must not appear in results
        assert!(detect_urls("ftp://example.com/file").is_empty());
    }

    // --- Additional terminal detection ---

    #[test]
    #[expect(
        unsafe_code,
        reason = "test-only env mutation in single-threaded test context"
    )]
    fn probe_detects_iterm_app() {
        // SAFETY: test-only env mutation; env vars are not read concurrently here.
        unsafe { std::env::set_var("TERM_PROGRAM", "iTerm.app") };
        let result = probe_hyperlink_support();
        unsafe { std::env::remove_var("TERM_PROGRAM") };
        assert!(result, "should detect iTerm via TERM_PROGRAM=iTerm.app");
    }

    #[test]
    #[expect(
        unsafe_code,
        reason = "test-only env mutation in single-threaded test context"
    )]
    fn probe_detects_foot_via_term_env() {
        // SAFETY: test-only env mutation; env vars are not read concurrently here.
        unsafe { std::env::set_var("TERM", "foot") };
        let result = probe_hyperlink_support();
        unsafe { std::env::remove_var("TERM") };
        assert!(result, "should detect foot terminal via TERM=foot");
    }

    #[test]
    #[expect(
        unsafe_code,
        reason = "test-only env mutation in single-threaded test context"
    )]
    fn probe_detects_alacritty_socket() {
        // SAFETY: test-only env mutation; env vars are not read concurrently here.
        unsafe { std::env::set_var("ALACRITTY_SOCKET", "/run/user/1000/alacritty.sock") };
        let result = probe_hyperlink_support();
        unsafe { std::env::remove_var("ALACRITTY_SOCKET") };
        assert!(result, "should detect Alacritty via ALACRITTY_SOCKET");
    }

    // --- Additional file path detection ---

    #[test]
    fn detects_typescript_file_path() {
        let paths = detect_file_paths("See src/components/App.tsx:42 for the component");
        assert_eq!(paths.len(), 1, "expected one TypeScript path match");
        assert!(
            paths[0].3.contains("App.tsx"),
            "URL should reference the tsx file"
        );
    }

    #[test]
    fn detects_dotslash_prefixed_path() {
        let paths = detect_file_paths("edit ./src/main.rs for details");
        assert_eq!(paths.len(), 1, "expected one path match for ./src/main.rs");
        assert!(
            paths[0].3.starts_with("file://"),
            "URL must use file:// scheme"
        );
    }

    #[test]
    fn detect_urls_includes_url_with_tilde_in_path() {
        let urls = detect_urls("https://example.com/~user/file");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://example.com/~user/file");
    }

    #[test]
    fn detect_urls_includes_userinfo() {
        let urls = detect_urls("https://user:pass@example.com");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://user:pass@example.com");
    }

    #[test]
    fn detect_urls_strips_trailing_comma_and_dot() {
        let urls = detect_urls("See https://example.com/path.,");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://example.com/path");
    }

    #[test]
    fn detect_urls_keeps_balanced_parens_in_path() {
        let url = "https://example.com/(path)";
        let urls = detect_urls(url);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, url);
    }

    #[test]
    fn detect_urls_strips_unbalanced_trailing_paren() {
        let urls = detect_urls("https://example.com/path)");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://example.com/path");
    }

    #[test]
    fn detect_urls_finds_url_at_start_of_string() {
        let text = "https://foo.com is here";
        let urls = detect_urls(text);
        assert_eq!(urls[0].0, 0);
        assert_eq!(urls[0].2, "https://foo.com");
    }

    #[test]
    fn detect_urls_finds_url_at_end_of_string() {
        let text = "see https://foo.com";
        let urls = detect_urls(text);
        assert_eq!(urls[0].1, text.len());
        assert_eq!(urls[0].2, "https://foo.com");
    }

    #[test]
    fn trim_trailing_punct_returns_zero_for_empty_string() {
        assert_eq!(trim_trailing_punct(""), 0);
    }

    #[test]
    fn trim_trailing_punct_leaves_string_without_punct_unchanged() {
        assert_eq!(trim_trailing_punct("abc"), 3);
    }

    #[test]
    fn trim_trailing_punct_strips_multiple_trailing_chars() {
        assert_eq!(trim_trailing_punct("abc..."), 3);
    }

    #[test]
    fn trim_trailing_punct_keeps_balanced_parens() {
        assert_eq!(trim_trailing_punct("abc(def)"), 8);
    }

    #[test]
    fn trim_trailing_punct_strips_unbalanced_closing_paren() {
        assert_eq!(trim_trailing_punct("abc)"), 3);
    }

    #[test]
    fn osc8_open_escapes_empty_url() {
        assert_eq!(osc8_open(""), "\x1b]8;;\x07");
    }

    #[test]
    fn osc8_open_preserves_query_params() {
        let seq = osc8_open("https://x.com?a=1&b=2");
        assert_eq!(seq, "\x1b]8;;https://x.com?a=1&b=2\x07");
    }

    #[test]
    fn detect_file_paths_finds_multiple_paths() {
        let paths = detect_file_paths("see src/a.rs and src/b.rs");
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn detect_file_paths_ignores_path_without_line_number() {
        let paths = detect_file_paths("edit src/main.rs");
        assert_eq!(paths.len(), 1);
        assert!(paths[0].3.contains("main.rs"));
    }

    #[test]
    fn detect_file_paths_ignores_invalid_extension() {
        let paths = detect_file_paths("file.exe");
        assert!(paths.is_empty());
    }

    #[test]
    fn detect_file_paths_ignores_path_with_spaces() {
        let paths = detect_file_paths("src/my file.rs");
        assert!(paths.is_empty());
    }

    #[test]
    fn detect_file_paths_trims_dotslash_prefix() -> std::io::Result<()> {
        let paths = detect_file_paths("./src/lib.rs");
        assert_eq!(paths.len(), 1);
        let expected = format!(
            "file://{}",
            std::env::current_dir()?.join("src/lib.rs").display()
        );
        assert_eq!(paths[0].3, expected);
        assert!(paths[0].3.starts_with("file:///"));
        Ok(())
    }

    #[test]
    fn detect_file_paths_keeps_absolute_paths_as_empty_authority_urls() -> std::io::Result<()> {
        let absolute = std::env::current_dir()?.join("src/lib.rs");
        let text = format!("edit {}", absolute.display());
        let paths = detect_file_paths(&text);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].3, format!("file://{}", absolute.display()));
        assert!(paths[0].3.starts_with("file:///"));
        Ok(())
    }
}
