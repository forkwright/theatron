//! OSC 8 terminal hyperlink support.

use std::sync::LazyLock;

use regex::Regex;

use crate::env::{Env, RealEnv};

/// A link found during markdown rendering, positioned within rendered lines.
///
/// Line index and column are **relative** to the `Vec<Line>` returned by
/// the consumer's markdown renderer and do **not** include any
/// content-prefix span that downstream views may prepend to each line.
///
/// Consumed cross-repo: aletheia's `koilon` markdown renderer constructs
/// `MdLink` values and its chat view resolves them into [`OscLink`]s.
#[derive(Debug, Clone)]
// kanon:ignore TOPOLOGY/shallow-struct -- cross-repo data contract: aletheia koilon constructs these in its markdown renderer; behavior lives with the consumer
pub struct MdLink {
    /// Zero-based index into the returned `Vec<Line>`.
    pub line_idx: usize,
    /// Display column within that line, in terminal cells (Unicode display
    /// width, content-prefix excluded). Producers must measure spans with
    /// `unicode-width`, not byte length, or links on lines containing
    /// non-ASCII text render at the wrong position.
    pub col: u16,
    /// Visible display text of the link (for re-rendering with OSC 8).
    pub text: String,
    /// Target URL.
    pub url: String,
}

/// A hyperlink fully resolved to absolute screen coordinates, ready to emit.
///
/// Consumed cross-repo: aletheia's `koilon` view layer returns
/// `Vec<OscLink>` from its render pass for OSC 8 post-processing.
#[derive(Debug, Clone)]
// kanon:ignore TOPOLOGY/shallow-struct -- cross-repo data contract: aletheia koilon's render pass returns Vec<OscLink>; behavior lives with the consumer
pub struct OscLink {
    /// Absolute terminal column (0-based).
    pub screen_x: u16,
    /// Absolute terminal row (0-based).
    pub screen_y: u16,
    /// Visible display text.
    pub text: String,
    /// Target URL.
    pub url: String,
    /// Accent colour (R, G, B) to apply when re-writing the link text.
    pub accent: (u8, u8, u8),
}

/// Format an OSC 8 hyperlink **opening** sequence.
///
/// Format: `ESC ] 8 ;; URL BEL`.
///
/// If `url` contains any C0 control character (0x00–0x1F), DEL (0x7F), or C1
/// control character (0x80–0x9F, including the 8-bit ST at 0x9C), the URL is
/// rejected and returned unchanged for plain-text rendering instead of emitting
/// a malformed OSC sequence.
#[must_use]
pub fn osc8_open(url: &str) -> String {
    if is_osc8_url_safe(url) {
        format!("\x1b]8;;{url}\x07")
    } else {
        url.to_string()
    }
}

/// Returns `true` if `url` contains no bytes that can terminate or alter an
/// OSC 8 sequence.
fn is_osc8_url_safe(url: &str) -> bool {
    !url.bytes()
        .any(|b| matches!(b, 0x00..=0x1F | 0x7F | 0x80..=0x9F))
}

/// OSC 8 hyperlink **closing** sequence: `ESC ] 8 ;; BEL`
#[must_use]
pub const fn osc8_close() -> &'static str {
    "\x1b]8;;\x07"
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
pub fn detect_urls(text: &str) -> Vec<(usize, usize, &str)> {
    #[expect(
        clippy::expect_used,
        reason = "regex is a compile-time string literal and is always valid"
    )]
    static URL_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"https?://[^\s<>{}|\\^`\[\]'"\x00-\x1F\x7F\x80-\x9F]+"#)
            .expect("hyperlink URL regex is valid")
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
/// Returns `(start, end, path, url)` where `url` is a `file://` URL with an
/// absolute path (RFC 8089 empty-authority form, `file:///abs/path`) suitable
/// for OSC 8. Relative paths are resolved against the current working
/// directory; a match is omitted when resolution fails rather than emitting a
/// malformed URL. Only paths with known source extensions are matched to
/// avoid false positives on arbitrary words.
///
/// Companion to [`detect_urls`]: consumers (e.g. a markdown renderer) call it
/// over prose text, skipping code blocks and inline code.
#[must_use]
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
        // WHY: RFC 8089 requires an absolute path -- `file://src/lib.rs`
        // would parse `src` as the authority (hostname). Resolve relative
        // paths against CWD; skip the match if CWD is unavailable.
        let abs = if std::path::Path::new(path_only).is_absolute() {
            std::path::PathBuf::from(path_only)
        } else {
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(path_only),
                Err(_) => continue,
            }
        };
        let url = format!("file://{}", abs.display());
        out.push((m.start(), m.end(), m.as_str(), url));
    }
    out
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "test assertions use direct indexing for clarity"
)]
#[path = "hyperlink_tests.rs"]
mod tests;
