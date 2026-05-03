//! API-layer error type for HTTP clients.
//!
//! Generic transport / response error for any fleet HTTP client built
//! on keryx. Failure modes:
//!
//! - [`ApiError::Http`] — transport or connection error from `reqwest`
//!   (no response received).
//! - [`ApiError::Timeout`] — request exceeded its timeout budget.
//!   Constructed by callers who detect `reqwest::Error::is_timeout`
//!   and want to surface the timeout to consumers as a distinct
//!   variant.
//! - [`ApiError::Server`] — non-2xx HTTP response with a
//!   human-readable message extracted from the server body when
//!   available.
//! - [`ApiError::RateLimited`] — 429 response (split out of `Server`
//!   for caller convenience). Carries the optional `Retry-After`
//!   value when the server supplies it.
//! - [`ApiError::BadResponse`] — server returned 2xx but the body
//!   couldn't be deserialized into the expected DTO. Carries the
//!   parser error for diagnostics.
//! - [`ApiError::Auth`] — credentials rejected (401/403).
//! - [`ApiError::InvalidToken`] — token contains bytes that aren't
//!   valid in an HTTP header value.
//!
//! `ApiError` is `#[non_exhaustive]`; consumer `match` statements
//! must include a wildcard arm so this enum can grow additional
//! variants without breaking downstream code.

use snafu::prelude::*;

/// Errors returned by HTTP API clients built on keryx.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum ApiError {
    /// HTTP transport or connection error (no response received).
    #[snafu(display("{operation}: {source}"))]
    Http {
        /// Which API call failed.
        operation: &'static str,
        /// Underlying reqwest error.
        source: reqwest::Error,
    },

    /// Request exceeded its timeout budget.
    ///
    /// Constructed by callers who detect [`reqwest::Error::is_timeout`]
    /// and want to surface the timeout to consumers as a distinct
    /// variant rather than burying it inside [`ApiError::Http`].
    /// Useful for retry logic that distinguishes timeouts (worth
    /// retrying) from connection refusals (usually not).
    #[snafu(display("{operation}: timed out after {timeout_secs}s"))]
    Timeout {
        /// Which API call timed out.
        operation: &'static str,
        /// Configured timeout in whole seconds.
        timeout_secs: u64,
    },

    /// Non-2xx HTTP response. Message is extracted from the server
    /// body when possible.
    #[snafu(display("{operation}: {status} {message}"))]
    Server {
        /// Which API call failed.
        operation: &'static str,
        /// HTTP status code from the response.
        status: u16,
        /// Human-readable error from the server.
        message: String,
    },

    /// 429 Too Many Requests response.
    ///
    /// Split out of [`ApiError::Server`] for caller convenience: a
    /// retry layer that uniformly classifies "back off and retry"
    /// from "fail loudly" benefits from this being a distinct
    /// variant. Carries the `Retry-After` value when the server
    /// supplies it (per RFC 9110 § 10.2.3).
    #[snafu(display(
        "{operation}: 429 rate limited{}",
        retry_after_secs.map_or(String::new(), |s| format!(" (retry after {s}s)"))
    ))]
    RateLimited {
        /// Which API call was rate-limited.
        operation: &'static str,
        /// `Retry-After` header in seconds, if the server supplied
        /// a delta-seconds value. None if the header was absent or
        /// in HTTP-date format we didn't parse.
        retry_after_secs: Option<u64>,
    },

    /// Server returned 2xx but the body couldn't be deserialized.
    ///
    /// Common causes: server schema drifted, client deserializer
    /// expects a field the server didn't send, or the body wasn't
    /// the expected content type. Carries the parser error for
    /// diagnostics.
    #[snafu(display("{operation}: bad response body: {source}"))]
    BadResponse {
        /// Which API call returned an unparseable body.
        operation: &'static str,
        /// Underlying deserialization error.
        source: serde_json::Error,
    },

    /// Credentials rejected by the server (401/403).
    #[snafu(display("authentication failed: token expired or invalid"))]
    Auth,

    /// Token contains characters that are not valid in an HTTP header
    /// value.
    #[snafu(display("invalid token: contains characters not valid in HTTP headers"))]
    InvalidToken,
}

/// Result alias for keryx API operations.
pub type Result<T> = std::result::Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_error_displays_status_and_message() {
        let err = ApiError::Server {
            operation: "get_pulls",
            status: 500,
            message: "internal error".to_string(),
        };
        let s = format!("{err}");
        assert!(s.contains("get_pulls"), "operation in display: {s}");
        assert!(s.contains("500"), "status in display: {s}");
        assert!(s.contains("internal error"), "message in display: {s}");
    }

    #[test]
    fn auth_error_displays_distinct_message() {
        let err = ApiError::Auth;
        let s = format!("{err}");
        assert!(s.contains("authentication"), "auth in display: {s}");
    }

    #[test]
    fn invalid_token_error_displays_distinct_message() {
        let err = ApiError::InvalidToken;
        let s = format!("{err}");
        assert!(s.contains("invalid token"), "invalid token in display: {s}");
    }

    #[test]
    fn timeout_error_displays_operation_and_seconds() {
        let err = ApiError::Timeout {
            operation: "get_runs",
            timeout_secs: 30,
        };
        let s = format!("{err}");
        assert!(s.contains("get_runs"), "operation in display: {s}");
        assert!(s.contains("30"), "timeout secs in display: {s}");
        assert!(s.contains("timed out"), "verb in display: {s}");
    }

    #[test]
    fn rate_limited_error_displays_retry_after_when_present() {
        let err = ApiError::RateLimited {
            operation: "list_prs",
            retry_after_secs: Some(60),
        };
        let s = format!("{err}");
        assert!(s.contains("list_prs"), "operation in display: {s}");
        assert!(s.contains("429"), "status in display: {s}");
        assert!(s.contains("retry after 60s"), "retry-after in display: {s}");
    }

    #[test]
    fn rate_limited_error_omits_retry_after_when_absent() {
        let err = ApiError::RateLimited {
            operation: "list_prs",
            retry_after_secs: None,
        };
        let s = format!("{err}");
        assert!(s.contains("list_prs"), "operation in display: {s}");
        assert!(s.contains("429"), "status in display: {s}");
        assert!(!s.contains("retry after"), "no retry-after when None: {s}");
    }

    #[test]
    fn bad_response_error_displays_operation_and_source() {
        let json_err = serde_json::from_str::<i32>("not a number").unwrap_err();
        let err = ApiError::BadResponse {
            operation: "get_health",
            source: json_err,
        };
        let s = format!("{err}");
        assert!(s.contains("get_health"), "operation in display: {s}");
        assert!(s.contains("bad response body"), "verb in display: {s}");
    }

    #[test]
    fn api_error_is_send_sync() {
        // Compile-time check: ApiError crosses thread / await
        // boundaries cleanly. Required for use in async tasks.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ApiError>();
    }

    #[test]
    fn api_error_implements_std_error() {
        // ApiError composes with `?` into anyhow / boxed-error chains.
        fn assert_error<T: std::error::Error>() {}
        assert_error::<ApiError>();
    }
}
