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
}
