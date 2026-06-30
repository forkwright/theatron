//! URL helpers for HTTP API clients.
//!
//! Small substrate for endpoint construction so consumers don't
//! hand-roll percent-encoding per call site. Follows RFC 3986
//! "unreserved characters" (`ALPHA`, `DIGIT`, `-`, `.`, `_`, `~`)
//! and emits `%XX` uppercase-hex for everything else.

/// Percent-encode a string for use as a URL path segment.
///
/// Implements the RFC 3986 [unreserved character][rfc3986] rule:
/// `A`–`Z`, `a`–`z`, `0`–`9`, `-`, `.`, `_`, `~` pass through
/// unchanged; every other byte is emitted as `%XX` uppercase
/// hex of the byte value.
///
/// Note: this encodes for the **path** segment context only — query
/// and fragment contexts have different reserved sets and are not
/// the concern here. For full URL construction, use the `url` or
/// `reqwest::Url` crates.
///
/// [rfc3986]: https://www.rfc-editor.org/rfc/rfc3986#section-2.3
///
/// # Examples
///
/// ```
/// use keryx::url::encode_path_segment;
///
/// assert_eq!(encode_path_segment("hello-world"), "hello-world");
/// assert_eq!(encode_path_segment("a/b"), "a%2Fb");
/// assert_eq!(encode_path_segment("hello world"), "hello%20world");
/// assert_eq!(encode_path_segment("café"), "caf%C3%A9");
/// ```
#[must_use]
// kanon:ignore RUST/pub-visibility -- public endpoint-construction helper for HTTP clients
pub fn encode_path_segment(segment: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(segment.len());
    for &byte in segment.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => {
                encoded.push('%');
                encoded.push(char::from(HEX[usize::from(byte >> 4)]));
                encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
        }
    }
    encoded
}

/// Join a base URL with a path, normalizing the slash boundary so the
/// result has exactly one `/` between them.
///
/// Strips trailing `/` from `base_url` and leading `/` from `path`, then
/// joins with a single separator. Either side may be empty; if `base_url`
/// is empty the normalized path is returned, and vice versa.
///
/// This is the common case for endpoint construction: a configured base
/// URL (operator-supplied, may or may not have a trailing slash) joined
/// with a route segment (template-supplied, may or may not have a
/// leading slash). Without a helper, every call site hand-rolls
/// `format!("{}/{}", base.trim_end_matches('/'), path)` and gets it
/// subtly wrong (double slash, missing slash, panic on empty input).
///
/// Note: this is a slash-normalizing string join, not a URL parser.
/// For full URL construction (query strings, fragments, encoding), use
/// the `url` or `reqwest::Url` crates.
///
/// # Examples
///
/// ```
/// use keryx::url::join_base_path;
///
/// // canonical case: trailing slash + leading slash collapse to one
/// assert_eq!(join_base_path("https://api/", "/v1/foo"), "https://api/v1/foo");
/// // missing both slashes: helper inserts the boundary
/// assert_eq!(join_base_path("https://api", "v1/foo"), "https://api/v1/foo");
/// // already-correct boundary stays correct
/// assert_eq!(join_base_path("https://api", "/v1/foo"), "https://api/v1/foo");
/// // empty path returns base alone (trailing slash stripped)
/// assert_eq!(join_base_path("https://api/", ""), "https://api");
/// ```
#[must_use]
// kanon:ignore RUST/pub-visibility -- public endpoint-construction helper for HTTP clients
pub fn join_base_path(base_url: &str, path: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    if base.is_empty() {
        return path.to_string();
    }
    if path.is_empty() {
        return base.to_string();
    }
    format!("{base}/{path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_unreserved_alphanumeric_through() {
        assert_eq!(encode_path_segment("hello-world"), "hello-world");
        assert_eq!(encode_path_segment("abc123"), "abc123");
        assert_eq!(
            encode_path_segment("Mixed_Case.value~v1"),
            "Mixed_Case.value~v1"
        );
    }

    #[test]
    fn percent_encodes_reserved_ascii() {
        assert_eq!(encode_path_segment("a/b"), "a%2Fb");
        assert_eq!(encode_path_segment("hello world"), "hello%20world");
        assert_eq!(encode_path_segment("id=1&x=2"), "id%3D1%26x%3D2");
        assert_eq!(encode_path_segment("a:b"), "a%3Ab");
        assert_eq!(encode_path_segment("a?b"), "a%3Fb");
        assert_eq!(encode_path_segment("a#b"), "a%23b");
    }

    #[test]
    fn percent_encodes_multibyte_utf8() {
        assert_eq!(encode_path_segment("café"), "caf%C3%A9");
        assert_eq!(encode_path_segment("日本語"), "%E6%97%A5%E6%9C%AC%E8%AA%9E");
    }

    #[test]
    fn empty_segment_returns_empty_string() {
        assert_eq!(encode_path_segment(""), "");
    }

    #[test]
    fn encodes_control_characters() {
        assert_eq!(encode_path_segment("\n"), "%0A");
        assert_eq!(encode_path_segment("\t"), "%09");
        assert_eq!(encode_path_segment("\0"), "%00");
    }

    #[test]
    fn uppercase_hex_in_percent_encoding() {
        // RFC 3986 § 2.1: percent-encoded sequences SHOULD use
        // uppercase hex.
        assert_eq!(encode_path_segment("\u{ff}"), "%C3%BF");
        assert!(
            encode_path_segment(" ")
                .chars()
                .all(|c| !c.is_ascii_lowercase())
        );
    }

    #[test]
    fn join_base_path_canonical_double_slash_collapses() {
        // Trailing `/` on base + leading `/` on path → exactly one `/`.
        assert_eq!(
            join_base_path("https://api/", "/v1/foo"),
            "https://api/v1/foo"
        );
    }

    #[test]
    fn join_base_path_inserts_missing_separator() {
        // Neither side has the `/` — helper inserts one.
        assert_eq!(
            join_base_path("https://api", "v1/foo"),
            "https://api/v1/foo"
        );
    }

    #[test]
    fn join_base_path_preserves_correct_boundary() {
        // Base no trailing slash + path leading slash → already correct.
        assert_eq!(
            join_base_path("https://api", "/v1/foo"),
            "https://api/v1/foo"
        );
        // Base trailing slash + path no leading slash → already correct.
        assert_eq!(
            join_base_path("https://api/", "v1/foo"),
            "https://api/v1/foo"
        );
    }

    #[test]
    fn join_base_path_strips_multiple_trailing_slashes() {
        assert_eq!(
            join_base_path("https://api///", "v1/foo"),
            "https://api/v1/foo"
        );
        assert_eq!(
            join_base_path("https://api///", "///v1/foo"),
            "https://api/v1/foo"
        );
    }

    #[test]
    fn join_base_path_empty_path_returns_base_without_trailing_slash() {
        assert_eq!(join_base_path("https://api/", ""), "https://api");
        assert_eq!(join_base_path("https://api", ""), "https://api");
        assert_eq!(join_base_path("https://api////", ""), "https://api");
    }

    #[test]
    fn join_base_path_empty_base_returns_path_without_leading_slash() {
        assert_eq!(join_base_path("", "/v1/foo"), "v1/foo");
        assert_eq!(join_base_path("", "v1/foo"), "v1/foo");
        assert_eq!(join_base_path("", "////v1/foo"), "v1/foo");
    }

    #[test]
    fn join_base_path_both_empty_returns_empty() {
        assert_eq!(join_base_path("", ""), "");
        assert_eq!(join_base_path("/", "/"), "");
        assert_eq!(join_base_path("////", "////"), "");
    }

    #[test]
    fn join_base_path_preserves_path_internal_slashes() {
        // Only the boundary slashes are normalized; slashes inside the
        // base or path stay as-is.
        assert_eq!(
            join_base_path("https://api/v1", "/sessions/abc/messages"),
            "https://api/v1/sessions/abc/messages"
        );
    }
}
