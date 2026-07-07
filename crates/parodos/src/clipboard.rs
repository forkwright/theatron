//! System clipboard read/write for terminal apps.
//!
//! Tries [`arboard`] (native) first; falls back to OSC52 escape sequences for
//! headless / SSH / tmux contexts. PNG-encoding helpers convert raw RGBA
//! image bytes for image-bearing clipboard content.

use crate::env::{Env, RealEnv};

/// Copy text to the system clipboard.
/// Tries arboard (native) first, falls back to OSC52 escape sequence.
///
/// # Errors
/// Returns an error if both the native clipboard and OSC52 fallback fail.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_text(text) {
            Ok(()) => {
                tracing::debug!("copied {} bytes to clipboard (native)", text.len());
                Ok(())
            }
            Err(e) => {
                tracing::warn!("native clipboard failed: {e}, trying OSC52");
                copy_osc52(text)
            }
        },
        Err(e) => {
            tracing::warn!("clipboard init failed: {e}, trying OSC52");
            copy_osc52(text)
        }
    }
}

/// Content read from the system clipboard.
#[expect(
    missing_docs,
    reason = "ClipboardContent variants and image-payload fields are self-evident from naming"
)]
#[non_exhaustive]
pub enum ClipboardContent {
    Text(String),
    Image {
        png_data: Vec<u8>,
        width: u32,
        height: u32,
    },
    Empty,
}

/// Read from the system clipboard, returning text or image data.
/// Tries arboard (native) first, falls back to system CLI tools for text.
pub fn read_from_clipboard() -> ClipboardContent {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            // WHY: try image first -- if the clipboard holds an image and we ask for text,
            // some platforms return the file path instead of the actual image data.
            if let Ok(img) = clipboard.get_image() {
                // WHY: arboard returns dimensions as usize; clipboard images are
                // bounded by screen size and always fit in u32. Skip the image
                // on impossible overflow rather than truncate silently.
                if let (Ok(width), Ok(height)) =
                    (u32::try_from(img.width), u32::try_from(img.height))
                    && let Some(png_data) = rgba_to_png(&img.bytes, width, height)
                {
                    return ClipboardContent::Image {
                        png_data,
                        width,
                        height,
                    };
                }
            }
            resolve_text_read(clipboard.get_text(), read_system_clipboard_text)
        }
        Err(e) => {
            tracing::warn!("clipboard read failed: {e}, trying system tools");
            read_system_clipboard_text()
        }
    }
}

/// Resolve a native clipboard text read.
///
/// Non-empty text passes through; empty text is a legitimately empty
/// clipboard. A backend read error invokes `fallback` (the system-tool
/// chain) so a post-init failure is never silently reported as `Empty`.
fn resolve_text_read(
    result: Result<String, arboard::Error>,
    fallback: impl FnOnce() -> ClipboardContent,
) -> ClipboardContent {
    match result {
        Ok(text) if !text.is_empty() => ClipboardContent::Text(text),
        Ok(_) => ClipboardContent::Empty,
        Err(e) => {
            tracing::warn!("native clipboard text read failed: {e}, trying system tools");
            fallback()
        }
    }
}

/// Encode raw RGBA pixel data as PNG bytes.
fn rgba_to_png(rgba: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    use image::ImageBuffer;

    let img: ImageBuffer<image::Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, rgba.to_vec())?;
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).ok()?;
    Some(buf.into_inner())
}

/// Fallback: read text from system clipboard tools (wl-paste, xclip, pbpaste).
fn read_system_clipboard_text() -> ClipboardContent {
    let tools: &[(&str, &[&str])] = &[
        ("wl-paste", &["--no-newline"]),
        ("xclip", &["-selection", "clipboard", "-o"]),
        ("pbpaste", &[]),
    ];
    for (cmd, args) in tools {
        // kanon:ignore RUST/no-direct-process-command -- parodos is a leaf substrate crate with no process-wrapper layer; spawning the platform clipboard CLIs directly is the fallback's entire purpose
        if let Ok(output) = std::process::Command::new(cmd).args(*args).output()
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            if !text.is_empty() {
                return ClipboardContent::Text(text);
            }
        }
    }
    ClipboardContent::Empty
}

/// Probe whether the running terminal likely supports OSC 52
/// clipboard escapes.
///
/// Symmetric to [`crate::hyperlink::supports_hyperlinks`] for OSC 8.
/// Useful for consumer code that wants to know up front whether
/// `copy_to_clipboard` will fall through to OSC 52 (vs the native
/// `arboard` backend, which works regardless of terminal).
///
/// Returns `true` for terminals known to support OSC 52: iTerm2,
/// Kitty, `WezTerm`, Alacritty, Ghostty, foot, Windows Terminal,
/// GNOME Terminal (VTE 0.76+), tmux (with passthrough), and the
/// `xterm` / `screen` / `tmux` `TERM` families. `false` for raw
/// Linux console and unknown terminals.
///
/// The result is cached for the process lifetime — terminal
/// capabilities don't change while the program runs.
///
/// # Caveats
///
/// This is a heuristic, not a runtime probe. False positives are
/// possible (a terminal pretending to be xterm without OSC 52
/// support); false negatives are possible (an unrecognized
/// terminal that does support OSC 52). The conservative path is
/// to call [`copy_to_clipboard`] regardless and rely on the
/// arboard-first fallback chain.
#[must_use]
pub fn supports_osc52() -> bool {
    static CACHE: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| probe_osc52(&RealEnv));
    *CACHE
}

fn probe_osc52(env: &impl Env) -> bool {
    // TERM_PROGRAM: most reliable signal on macOS and some Linux terminals.
    if let Some(prog) = env.var("TERM_PROGRAM") {
        match prog.as_str() {
            "iTerm.app" | "WezTerm" | "ghostty" | "Ghostty" | "kitty" | "vscode" => return true,
            _ => {
                // unrecognized TERM_PROGRAM — continue probing
            }
        }
    }

    // Ghostty / WezTerm / Kitty / Alacritty / Windows Terminal — same env-var
    // signals as supports_hyperlinks; these terminals all support OSC 52.
    if env.var("GHOSTTY_BIN_DIR").is_some()
        || env.var("GHOSTTY_RESOURCES_DIR").is_some()
        || env.var("WEZTERM_EXECUTABLE").is_some()
        || env.var("WEZTERM_PANE").is_some()
        || env.var("KITTY_PID").is_some()
        || env.var("KITTY_WINDOW_ID").is_some()
        || env.var("ALACRITTY_SOCKET").is_some()
        || env.var("WT_SESSION").is_some()
    {
        return true;
    }

    // foot
    if let Some(term) = env.var("TERM")
        && (term == "foot" || term == "foot-extra")
    {
        return true;
    }

    // tmux passthrough — OSC 52 works inside tmux when the surrounding
    // terminal supports it (and the copy_osc52 wrapper handles the
    // \x1bPtmux escape). Conservatively say yes on tmux.
    if env.var("TMUX").is_some() {
        return true;
    }

    // Generic xterm / screen TERM families — most modern members
    // support OSC 52.
    if let Some(term) = env.var("TERM") {
        if term.starts_with("xterm") || term.starts_with("screen") || term.starts_with("tmux") {
            return true;
        }
    }

    false
}

/// OSC52 clipboard escape sequence: works over SSH, inside tmux/screen.
/// Supported by: iTerm2, Kitty, `WezTerm`, Alacritty, GNOME Terminal (VTE 0.76+).
fn copy_osc52(text: &str) -> Result<(), String> {
    use std::io::Write;

    let encoded =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, text.as_bytes());

    // NOTE: tmux requires an OSC52 passthrough wrapper for the escape sequence to reach the terminal
    let seq = if RealEnv.var("TMUX").is_some() {
        format!("\x1bPtmux;\x1b\x1b]52;c;{encoded}\x07\x1b\\")
    } else {
        format!("\x1b]52;c;{encoded}\x07")
    };

    std::io::stdout()
        .write_all(seq.as_bytes())
        .map_err(|e| format!("OSC52 write failed: {e}"))?;
    std::io::stdout()
        .flush()
        .map_err(|e| format!("OSC52 flush failed: {e}"))?;

    tracing::debug!("copied {} bytes to clipboard (OSC52)", text.len());
    Ok(())
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "test assertions may panic on failure")]
mod tests {
    use super::*;

    #[test]
    fn copy_osc52_generates_valid_sequence() {
        let text = "test clipboard content";
        let encoded =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, text.as_bytes());
        assert!(!encoded.is_empty());
        let decoded =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &encoded).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), text);
    }

    #[test]
    fn copy_osc52_tmux_detection() {
        let text = "test";
        let encoded =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, text.as_bytes());
        let tmux_seq = format!("\x1bPtmux;\x1b\x1b]52;c;{encoded}\x07\x1b\\");
        let normal_seq = format!("\x1b]52;c;{encoded}\x07");
        assert!(tmux_seq.len() > normal_seq.len());
        assert!(tmux_seq.starts_with("\x1bPtmux;"));
    }

    #[test]
    fn rgba_to_png_valid_1x1() {
        let rgba = vec![255, 0, 0, 255]; // 1x1 red pixel
        let png = rgba_to_png(&rgba, 1, 1);
        assert!(png.is_some());
        let data = png.unwrap();
        // PNG magic bytes
        assert_eq!(&data[..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn rgba_to_png_invalid_dimensions_returns_none() {
        let rgba = vec![255, 0, 0, 255]; // 1 pixel but claim 2x2
        let png = rgba_to_png(&rgba, 2, 2);
        assert!(png.is_none());
    }

    #[test]
    fn rgba_to_png_encodes_2x2_gradient() {
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ];
        let png = rgba_to_png(&rgba, 2, 2);
        assert!(png.is_some());
        let data = png.unwrap();
        assert_eq!(&data[..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn rgba_to_png_large_dimensions_succeeds() {
        let rgba = vec![128; 10 * 10 * 4];
        let png = rgba_to_png(&rgba, 10, 10);
        assert!(png.is_some());
    }

    #[test]
    fn clipboard_content_text_holds_value() {
        let content = ClipboardContent::Text("hello".to_string());
        assert!(matches!(content, ClipboardContent::Text(ref s) if s == "hello"));
    }

    #[test]
    fn clipboard_content_image_holds_dimensions() {
        let content = ClipboardContent::Image {
            png_data: vec![1, 2, 3],
            width: 100,
            height: 200,
        };
        assert!(
            matches!(
                content,
                ClipboardContent::Image {
                    width: 100,
                    height: 200,
                    ..
                }
            ),
            "expected image with width 100 and height 200"
        );
    }

    #[test]
    fn text_read_error_falls_back_to_system_tools() {
        // WHY: a backend failure after successful init must be
        // distinguishable from an empty clipboard -- the fallback chain
        // runs instead of returning Empty.
        let out = resolve_text_read(Err(arboard::Error::ContentNotAvailable), || {
            ClipboardContent::Text("from-system-tools".to_string())
        });
        assert!(
            matches!(out, ClipboardContent::Text(ref s) if s == "from-system-tools"),
            "a get_text error must invoke the system-tool fallback, not report Empty"
        );
    }

    #[test]
    fn empty_text_read_is_empty_without_fallback() {
        let out = resolve_text_read(Ok(String::new()), || {
            ClipboardContent::Text("fallback must not run".to_string())
        });
        assert!(matches!(out, ClipboardContent::Empty));
    }

    #[test]
    fn nonempty_text_read_passes_through() {
        let out = resolve_text_read(Ok("hello".to_string()), || ClipboardContent::Empty);
        assert!(matches!(out, ClipboardContent::Text(ref s) if s == "hello"));
    }

    /// In-memory `Env` for terminal-detection tests.
    struct TestEnv {
        vars: std::collections::HashMap<String, String>,
    }
    impl TestEnv {
        fn new(pairs: &[(&str, &str)]) -> Self {
            Self {
                vars: pairs
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect(),
            }
        }
    }
    impl Env for TestEnv {
        fn var(&self, name: &str) -> Option<String> {
            self.vars.get(name).cloned()
        }
    }

    #[test]
    fn probe_osc52_returns_true_for_kitty_term_program() {
        let env = TestEnv::new(&[("TERM_PROGRAM", "kitty")]);
        assert!(probe_osc52(&env));
    }

    #[test]
    fn probe_osc52_returns_true_for_iterm2() {
        let env = TestEnv::new(&[("TERM_PROGRAM", "iTerm.app")]);
        assert!(probe_osc52(&env));
    }

    #[test]
    fn probe_osc52_returns_true_for_kitty_pid_signal() {
        let env = TestEnv::new(&[("KITTY_PID", "1234")]);
        assert!(probe_osc52(&env));
    }

    #[test]
    fn probe_osc52_returns_true_inside_tmux() {
        let env = TestEnv::new(&[("TMUX", "/tmp/tmux-1000/default,1234,0")]);
        assert!(probe_osc52(&env));
    }

    #[test]
    fn probe_osc52_returns_true_for_xterm_term_family() {
        let env = TestEnv::new(&[("TERM", "xterm-256color")]);
        assert!(probe_osc52(&env));
    }

    #[test]
    fn probe_osc52_returns_true_for_screen_term() {
        let env = TestEnv::new(&[("TERM", "screen")]);
        assert!(probe_osc52(&env));
    }

    #[test]
    fn probe_osc52_returns_true_for_foot() {
        let env = TestEnv::new(&[("TERM", "foot")]);
        assert!(probe_osc52(&env));
    }

    #[test]
    fn probe_osc52_returns_false_for_raw_linux_console() {
        let env = TestEnv::new(&[("TERM", "linux")]);
        assert!(!probe_osc52(&env));
    }

    #[test]
    fn probe_osc52_returns_false_for_unknown_term() {
        let env = TestEnv::new(&[("TERM", "vt100")]);
        assert!(!probe_osc52(&env));
    }

    #[test]
    fn probe_osc52_returns_false_for_empty_env() {
        let env = TestEnv::new(&[]);
        assert!(!probe_osc52(&env));
    }
}
