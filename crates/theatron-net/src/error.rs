//! API-layer error type for HTTP clients.
//!
//! Generic transport / response error for any fleet HTTP client built
//! on theatron-net. Three failure modes:
//!
//! - [`ApiError::Http`] — transport or connection error from `reqwest`
//!   (no response received).
//! - [`ApiError::Server`] — non-2xx HTTP response with a human-readable
//!   message extracted from the server body when available.
//! - [`ApiError::Auth`] — credentials rejected (401/403).
//! - [`ApiError::InvalidToken`] — token contains bytes that aren't
//!   valid in an HTTP header value.

use snafu::prelude::*;

/// Errors returned by HTTP API clients built on theatron-net.
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

    /// Credentials rejected by the server (401/403).
    #[snafu(display("authentication failed: token expired or invalid"))]
    Auth,

    /// Token contains characters that are not valid in an HTTP header
    /// value.
    #[snafu(display("invalid token: contains characters not valid in HTTP headers"))]
    InvalidToken,
}

/// Result alias for theatron-net API operations.
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
}
