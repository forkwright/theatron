//! Tests for [`crate::hyperlink`].

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
fn supports_hyperlinks_does_not_panic() {
    assert!(
        std::panic::catch_unwind(supports_hyperlinks).is_ok(),
        "supports_hyperlinks should probe/cache terminal support without panicking"
    );
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
    assert!(
        paths[0].3.starts_with("file:///"),
        "relative paths must resolve to the absolute empty-authority form, got {}",
        paths[0].3
    );
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
        paths[0].3.starts_with("file:///"),
        "URL must use the absolute file:/// form"
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
fn detect_file_paths_trims_dotslash_prefix() {
    let paths = detect_file_paths("./src/lib.rs");
    assert_eq!(paths.len(), 1);
    // WHY: `file://src/lib.rs` would parse `src` as an RFC 8089
    // authority; the dot-slash prefix must resolve to an absolute path.
    assert!(
        paths[0].3.starts_with("file:///"),
        "expected absolute file URL, got {}",
        paths[0].3
    );
    assert!(paths[0].3.ends_with("/src/lib.rs"));
}

#[test]
fn detect_file_paths_keeps_absolute_path_verbatim() {
    let paths = detect_file_paths("see /tmp/scratch/build.rs for the hook");
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].3, "file:///tmp/scratch/build.rs");
}

// --- OSC 8 control-character rejection ---

#[test]
fn osc8_open_rejects_url_with_bel() {
    let url = "https://example.com\x07injected";
    let out = osc8_open(url);
    assert!(
        !out.starts_with('\x1b'),
        "BEL in URL must abort OSC 8 hyperlink emission"
    );
    assert!(
        !out.contains("\x1b]8;;"),
        "must not emit a malformed OSC 8 open"
    );
    assert_eq!(out, url, "rejected URL must render as plain text");
}

#[test]
fn osc8_open_rejects_url_with_esc() {
    let url = "https://example.com\x1binjected";
    let out = osc8_open(url);
    assert!(
        !out.starts_with('\x1b'),
        "ESC in URL must abort OSC 8 hyperlink emission"
    );
    assert!(
        !out.contains("\x1b]8;;"),
        "must not emit a malformed OSC 8 open"
    );
    assert_eq!(out, url, "rejected URL must render as plain text");
}

#[test]
fn osc8_open_rejects_url_with_8bit_st() {
    // U+009C (8-bit ST) encoded as UTF-8
    let url = "https://example.com\u{009C}injected";
    let out = osc8_open(url);
    assert!(
        !out.starts_with('\x1b'),
        "8-bit ST in URL must abort OSC 8 hyperlink emission"
    );
    assert!(
        !out.contains("\x1b]8;;"),
        "must not emit a malformed OSC 8 open"
    );
    assert_eq!(out, url, "rejected URL must render as plain text");
}

#[test]
fn osc8_open_clean_url_unchanged() {
    let seq = osc8_open("https://example.com");
    assert_eq!(seq, "\x1b]8;;https://example.com\x07");
}

#[test]
fn osc8_open_rejects_all_forbidden_control_bytes() {
    for b in 0x00_u32..=0x1F {
        let ch = char::from_u32(b).expect("C0 range is valid Unicode");
        let url = format!("https://x.com/{ch}tail");
        let out = osc8_open(&url);
        assert!(
            !out.starts_with('\x1b'),
            "C0 byte 0x{b:02X} must abort OSC 8 hyperlink emission"
        );
        assert!(
            !out.contains("\x1b]8;;"),
            "C0 byte 0x{b:02X} must not appear inside an OSC 8 payload"
        );
    }

    // DEL
    let del_url = format!("https://x.com/{}tail", '\x7F');
    let del_out = osc8_open(&del_url);
    assert!(
        !del_out.starts_with('\x1b'),
        "DEL (0x7F) must abort OSC 8 hyperlink emission"
    );
    assert!(
        !del_out.contains("\x1b]8;;"),
        "DEL (0x7F) must not appear inside an OSC 8 payload"
    );

    // C1 controls (0x80–0x9F)
    for b in 0x80_u32..=0x9F {
        let ch = char::from_u32(b).expect("C1 range is valid Unicode");
        let url = format!("https://x.com/{ch}tail");
        let out = osc8_open(&url);
        assert!(
            !out.starts_with('\x1b'),
            "C1 byte 0x{b:02X} must abort OSC 8 hyperlink emission"
        );
        assert!(
            !out.contains("\x1b]8;;"),
            "C1 byte 0x{b:02X} must not appear inside an OSC 8 payload"
        );
    }
}

#[test]
fn osc8_open_allows_international_url() {
    // WHY (#183): 'À' (U+00C0) UTF-8-encodes to 0xC2 0x80 -- the old
    // byte-range check treated the 0x80 continuation byte as a C1 control
    // and rejected the whole URL even though U+00C0 is an ordinary letter.
    // Also covers CJK ('é' composed forms) to confirm no multi-byte
    // character trips the safety check.
    let url = "https://example.com/caf\u{00E9}/\u{00C0}/\u{4F60}\u{597D}";
    let out = osc8_open(url);
    assert_eq!(
        out,
        format!("\x1b]8;;{url}\x07"),
        "international URL must not be rejected as unsafe"
    );
}

#[test]
fn detect_urls_excludes_control_characters() {
    let urls = detect_urls("see https://evil.com/\x07bad");
    assert!(
        urls.iter().all(|(_, _, url)| {
            !url.bytes()
                .any(|b| matches!(b, 0x00..=0x1F | 0x7F | 0x80..=0x9F))
        }),
        "detected URLs must not contain OSC-terminating control bytes"
    );
}
