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
//! - [`ApiError::Server`] — non-2xx HTTP response; carries a
//!   human-readable message from the server body when available.
//! - [`ApiError::RateLimited`] — 429 response, distinct from
//!   `Server` so retry layers can classify back-off directly.
//!   Carries the optional `Retry-After` value when the server
//!   supplies it.
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

impl ApiError {
    /// Whether this error is worth retrying.
    ///
    /// Returns `true` for transient failure modes that a retry
    /// layer should back off and retry:
    ///
    /// - [`ApiError::Timeout`] — request exceeded its budget;
    ///   the server may respond on a retry.
    /// - [`ApiError::RateLimited`] — back off (honoring
    ///   `retry_after_secs` if set) and retry.
    /// - [`ApiError::Http`] when the underlying `reqwest::Error`
    ///   is a connect failure or timeout.
    /// - [`ApiError::Server`] for 5xx responses.
    ///
    /// Returns `false` for terminal failures that won't change on
    /// retry without external intervention:
    ///
    /// - [`ApiError::Server`] for 4xx responses (caller error).
    /// - [`ApiError::BadResponse`] — schema mismatch.
    /// - [`ApiError::Auth`] — need a fresh credential.
    /// - [`ApiError::InvalidToken`] — token is malformed.
    /// - [`ApiError::Http`] for non-connect / non-timeout errors
    ///   (TLS handshake failure, body decode error, etc).
    ///
    /// Conservative by default: retry layers that want more
    /// aggressive behaviour (e.g. retry on 4xx for idempotent
    /// reads) should make their own judgment.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout { .. } | Self::RateLimited { .. } => true,
            Self::Http { source, .. } => source.is_connect() || source.is_timeout(),
            Self::Server { status, .. } => *status >= 500,
            Self::BadResponse { .. } | Self::Auth | Self::InvalidToken => false,
        }
    }

    /// Return the HTTP status code carried by this error, if any.
    ///
    /// Returns `Some(status)` for variants that received an HTTP
    /// response:
    ///
    /// - [`ApiError::Server`] — the wire status (4xx, 5xx, etc).
    /// - [`ApiError::RateLimited`] — always `429` (the variant
    ///   exists exactly because the response was 429).
    ///
    /// Returns `None` for variants that didn't receive a response
    /// or whose status would be ambiguous:
    ///
    /// - [`ApiError::Http`] — transport-level failure (no response).
    /// - [`ApiError::Timeout`] — no response received.
    /// - [`ApiError::BadResponse`] — 2xx response but unparseable
    ///   body; the specific 2xx code isn't preserved.
    /// - [`ApiError::Auth`] — the credential rejection could be
    ///   401 or 403; returning a single specific value would lose
    ///   information. Use the original `Server` variant if status
    ///   precision matters for auth failures.
    /// - [`ApiError::InvalidToken`] — pre-flight failure, no
    ///   request was sent.
    ///
    /// Useful for consumer code that wants to log or branch on
    /// the wire status without manual destructuring per variant.
    #[must_use]
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::Server { status, .. } => Some(*status),
            Self::RateLimited { .. } => Some(429),
            Self::Http { .. }
            | Self::Timeout { .. }
            | Self::BadResponse { .. }
            | Self::Auth
            | Self::InvalidToken => None,
        }
    }

    /// Return the operation name embedded in this error, if the
    /// variant carries one.
    ///
    /// Returns `None` for variants that have no operation context
    /// ([`ApiError::Auth`], [`ApiError::InvalidToken`]). Returns
    /// `Some(&'static str)` for the rest.
    ///
    /// Useful for consumer code that wants to log or route on the
    /// operation name without manually destructuring each variant:
    ///
    /// ```ignore
    /// match client.fetch().await {
    ///     Err(e) => tracing::error!(op = ?e.operation(), "request failed: {e}"),
    ///     Ok(v) => ...
    /// }
    /// ```
    #[must_use]
    // kanon:ignore RUST/doc-promised-observability -- doc shows caller-side `tracing::error!` usage; this method is a pure getter, side-effect free.
    pub fn operation(&self) -> Option<&'static str> {
        match self {
            Self::Http { operation, .. }
            | Self::Timeout { operation, .. }
            | Self::Server { operation, .. }
            | Self::RateLimited { operation, .. }
            | Self::BadResponse { operation, .. } => Some(operation),
            Self::Auth | Self::InvalidToken => None,
        }
    }

    /// Return the `Retry-After` delta-seconds value carried by a
    /// rate-limit error, if the server supplied one.
    ///
    /// Returns `Some(secs)` when the variant is
    /// [`ApiError::RateLimited`] and the server included a
    /// `Retry-After` header in delta-seconds form (per RFC 9110
    /// § 10.2.3). Returns `None` for every other variant, and also
    /// for [`ApiError::RateLimited`] when the header was absent or
    /// in HTTP-date form that the caller didn't parse.
    ///
    /// Useful for retry layers that want to honour server backoff
    /// hints without manually destructuring:
    ///
    /// ```ignore
    /// if let Some(secs) = err.retry_after() {
    ///     tokio::time::sleep(Duration::from_secs(secs)).await;
    /// }
    /// ```
    #[must_use]
    pub fn retry_after(&self) -> Option<u64> {
        match self {
            Self::RateLimited {
                retry_after_secs, ..
            } => *retry_after_secs,
            Self::Http { .. }
            | Self::Timeout { .. }
            | Self::Server { .. }
            | Self::BadResponse { .. }
            | Self::Auth
            | Self::InvalidToken => None,
        }
    }

    /// Whether this error is a 4xx client-error response.
    ///
    /// Returns `true` for [`ApiError::Server`] with a status in
    /// `400..=499`, and for [`ApiError::RateLimited`] (always 429).
    /// Returns `false` for every other variant — those are
    /// transport-level (`Http`, `Timeout`), payload (`BadResponse`),
    /// or pre-flight (`InvalidToken`) failures, which by definition
    /// did not receive a 4xx response. [`ApiError::Auth`] also
    /// returns `false` because the variant erases the specific
    /// 401-vs-403 status; consumers wanting to count it as a
    /// client error should check `is_auth_failure()` separately
    /// (when that lands).
    ///
    /// Pairs with [`is_server_error`](Self::is_server_error) and
    /// [`is_retryable`](Self::is_retryable) to give consumers the
    /// canonical HTTP-class trio without manual status arithmetic.
    #[must_use]
    pub fn is_client_error(&self) -> bool {
        match self {
            Self::Server { status, .. } => (400..=499).contains(status),
            Self::RateLimited { .. } => true,
            Self::Http { .. }
            | Self::Timeout { .. }
            | Self::BadResponse { .. }
            | Self::Auth
            | Self::InvalidToken => false,
        }
    }

    /// Whether this error is a 5xx server-error response.
    ///
    /// Returns `true` for [`ApiError::Server`] with a status in
    /// `500..=599`. Returns `false` for every other variant —
    /// transport / payload / pre-flight failures didn't receive
    /// a 5xx response by definition, and `RateLimited` is always
    /// 429 (a client error).
    ///
    /// Pairs with [`is_client_error`](Self::is_client_error) and
    /// [`is_retryable`](Self::is_retryable). Note `is_server_error`
    /// is a strict subset of `is_retryable` (every 5xx is
    /// retryable, but `is_retryable` also catches `Timeout`,
    /// `RateLimited`, and connect-failure `Http`).
    #[must_use]
    pub fn is_server_error(&self) -> bool {
        match self {
            Self::Server { status, .. } => (500..=599).contains(status),
            Self::Http { .. }
            | Self::Timeout { .. }
            | Self::RateLimited { .. }
            | Self::BadResponse { .. }
            | Self::Auth
            | Self::InvalidToken => false,
        }
    }
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
    fn api_error_is_send_sync_std_error() {
        // WHY: the generic bound enforces Send + Sync + Error at
        // compile time (required for async tasks and `?` error
        // propagation); the runtime assertion verifies the Display
        // path through the Error impl.
        fn display_via_error<T: std::error::Error + Send + Sync>(err: &T) -> String {
            err.to_string()
        }
        assert_eq!(
            display_via_error(&ApiError::Auth),
            "authentication failed: token expired or invalid"
        );
    }

    #[test]
    fn operation_returns_some_for_operation_carrying_variants() {
        let http = ApiError::Http {
            operation: "fetch_pulls",
            source: build_dummy_reqwest_error(),
        };
        let timeout = ApiError::Timeout {
            operation: "fetch_runs",
            timeout_secs: 30,
        };
        let server = ApiError::Server {
            operation: "list_prs",
            status: 500,
            message: String::new(),
        };
        let rate_limited = ApiError::RateLimited {
            operation: "list_users",
            retry_after_secs: None,
        };
        let bad_response = ApiError::BadResponse {
            operation: "get_repo",
            source: serde_json::from_str::<i32>("nope").unwrap_err(),
        };
        assert_eq!(http.operation(), Some("fetch_pulls"));
        assert_eq!(timeout.operation(), Some("fetch_runs"));
        assert_eq!(server.operation(), Some("list_prs"));
        assert_eq!(rate_limited.operation(), Some("list_users"));
        assert_eq!(bad_response.operation(), Some("get_repo"));
    }

    #[test]
    fn operation_returns_none_for_context_free_variants() {
        assert_eq!(ApiError::Auth.operation(), None);
        assert_eq!(ApiError::InvalidToken.operation(), None);
    }

    #[test]
    fn is_retryable_true_for_timeout() {
        let err = ApiError::Timeout {
            operation: "x",
            timeout_secs: 5,
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn is_retryable_true_for_rate_limited() {
        let with_retry = ApiError::RateLimited {
            operation: "x",
            retry_after_secs: Some(60),
        };
        let without_retry = ApiError::RateLimited {
            operation: "x",
            retry_after_secs: None,
        };
        assert!(with_retry.is_retryable());
        assert!(without_retry.is_retryable());
    }

    #[test]
    fn is_retryable_true_for_5xx_server() {
        for status in [500, 502, 503, 504, 599] {
            let err = ApiError::Server {
                operation: "x",
                status,
                message: String::new(),
            };
            assert!(
                err.is_retryable(),
                "5xx status {status} should be retryable"
            );
        }
    }

    #[test]
    fn is_retryable_false_for_4xx_server() {
        for status in [400, 403, 404, 422, 499] {
            let err = ApiError::Server {
                operation: "x",
                status,
                message: String::new(),
            };
            assert!(
                !err.is_retryable(),
                "4xx status {status} should not be retryable"
            );
        }
    }

    #[test]
    fn is_retryable_false_for_terminal_variants() {
        let bad_response = ApiError::BadResponse {
            operation: "x",
            source: serde_json::from_str::<i32>("nope").unwrap_err(),
        };
        assert!(!bad_response.is_retryable());
        assert!(!ApiError::Auth.is_retryable());
        assert!(!ApiError::InvalidToken.is_retryable());
    }

    #[test]
    fn status_code_returns_wire_status_for_server() {
        for status in [400, 404, 500, 503] {
            let err = ApiError::Server {
                operation: "x",
                status,
                message: String::new(),
            };
            assert_eq!(err.status_code(), Some(status));
        }
    }

    #[test]
    fn status_code_returns_429_for_rate_limited() {
        let with = ApiError::RateLimited {
            operation: "x",
            retry_after_secs: Some(60),
        };
        let without = ApiError::RateLimited {
            operation: "x",
            retry_after_secs: None,
        };
        assert_eq!(with.status_code(), Some(429));
        assert_eq!(without.status_code(), Some(429));
    }

    #[test]
    fn status_code_returns_none_for_response_less_variants() {
        let http = ApiError::Http {
            operation: "x",
            source: build_dummy_reqwest_error(),
        };
        let timeout = ApiError::Timeout {
            operation: "x",
            timeout_secs: 5,
        };
        let bad_response = ApiError::BadResponse {
            operation: "x",
            source: serde_json::from_str::<i32>("nope").unwrap_err(),
        };
        assert_eq!(http.status_code(), None);
        assert_eq!(timeout.status_code(), None);
        assert_eq!(bad_response.status_code(), None);
        assert_eq!(ApiError::Auth.status_code(), None);
        assert_eq!(ApiError::InvalidToken.status_code(), None);
    }

    #[test]
    fn retry_after_returns_some_for_rate_limited_with_header() {
        let err = ApiError::RateLimited {
            operation: "list_prs",
            retry_after_secs: Some(60),
        };
        assert_eq!(err.retry_after(), Some(60));
    }

    #[test]
    fn retry_after_returns_none_for_rate_limited_without_header() {
        let err = ApiError::RateLimited {
            operation: "list_prs",
            retry_after_secs: None,
        };
        assert_eq!(err.retry_after(), None);
    }

    #[test]
    fn retry_after_returns_none_for_other_variants() {
        let http = ApiError::Http {
            operation: "x",
            source: build_dummy_reqwest_error(),
        };
        let timeout = ApiError::Timeout {
            operation: "x",
            timeout_secs: 5,
        };
        let server = ApiError::Server {
            operation: "x",
            status: 503,
            message: String::new(),
        };
        let bad_response = ApiError::BadResponse {
            operation: "x",
            source: serde_json::from_str::<i32>("nope").unwrap_err(),
        };
        assert_eq!(http.retry_after(), None);
        assert_eq!(timeout.retry_after(), None);
        assert_eq!(server.retry_after(), None);
        assert_eq!(bad_response.retry_after(), None);
        assert_eq!(ApiError::Auth.retry_after(), None);
        assert_eq!(ApiError::InvalidToken.retry_after(), None);
    }

    #[test]
    fn is_client_error_true_for_4xx_server() {
        for status in [400, 401, 403, 404, 422, 429, 499] {
            let err = ApiError::Server {
                operation: "x",
                status,
                message: String::new(),
            };
            assert!(err.is_client_error(), "{status} should be client error");
            assert!(
                !err.is_server_error(),
                "{status} should not be server error"
            );
        }
    }

    #[test]
    fn is_server_error_true_for_5xx_server() {
        for status in [500, 502, 503, 504, 599] {
            let err = ApiError::Server {
                operation: "x",
                status,
                message: String::new(),
            };
            assert!(err.is_server_error(), "{status} should be server error");
            assert!(
                !err.is_client_error(),
                "{status} should not be client error"
            );
        }
    }

    #[test]
    fn rate_limited_is_client_error_not_server_error() {
        let err = ApiError::RateLimited {
            operation: "x",
            retry_after_secs: Some(60),
        };
        assert!(err.is_client_error());
        assert!(!err.is_server_error());
    }

    #[test]
    fn class_predicates_false_for_response_less_variants() {
        let http = ApiError::Http {
            operation: "x",
            source: build_dummy_reqwest_error(),
        };
        let timeout = ApiError::Timeout {
            operation: "x",
            timeout_secs: 5,
        };
        let bad_response = ApiError::BadResponse {
            operation: "x",
            source: serde_json::from_str::<i32>("nope").unwrap_err(),
        };
        for err in [
            http,
            timeout,
            bad_response,
            ApiError::Auth,
            ApiError::InvalidToken,
        ] {
            assert!(!err.is_client_error(), "no response → not 4xx ({err:?})");
            assert!(!err.is_server_error(), "no response → not 5xx ({err:?})");
        }
    }

    #[test]
    fn class_predicates_partition_server_status_codes() {
        // Every 4xx is client_error xor server_error; same for 5xx.
        // Boundary statuses (399, 600) fall in neither.
        let make = |status| ApiError::Server {
            operation: "x",
            status,
            message: String::new(),
        };
        assert!(!make(399).is_client_error() && !make(399).is_server_error());
        assert!(make(400).is_client_error() && !make(400).is_server_error());
        assert!(make(499).is_client_error() && !make(499).is_server_error());
        assert!(!make(500).is_client_error() && make(500).is_server_error());
        assert!(!make(599).is_client_error() && make(599).is_server_error());
        assert!(!make(600).is_client_error() && !make(600).is_server_error());
    }

    /// Build a `reqwest::Error` synchronously for the
    /// `operation_returns_some` test. `reqwest::Error` has no
    /// public constructor; the cheapest path is to call the async
    /// client's builder with an invalid URL, which fails
    /// pre-network at `build()` time and returns a real
    /// `reqwest::Error`.
    fn build_dummy_reqwest_error() -> reqwest::Error {
        crate::install_test_crypto_provider();
        reqwest::Client::new().get("not a url").build().unwrap_err()
    }
}
