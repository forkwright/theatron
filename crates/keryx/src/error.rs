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
//! - [`ApiError::BodyTooLarge`] — response body exceeded the
//!   configured maximum size before it could be fully buffered.
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
    ///
    /// # Security
    ///
    /// The `Display` impl redacts any userinfo (embedded credentials)
    /// from a URL carried by `source` — `reqwest::Error`'s own
    /// `Display` echoes the request URL verbatim, which would leak a
    /// credential embedded in the URL (`https://user:pass@host/...`)
    /// into logs.
    #[snafu(display("{operation}: {}", redact_reqwest_error(source)))]
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

    /// Response body exceeded the configured maximum size before it
    /// could be fully buffered.
    ///
    /// See `response::read_body_capped` for the read path that
    /// enforces this cap.
    #[snafu(display("{operation}: response body exceeded {limit} bytes"))]
    BodyTooLarge {
        /// Which API call's response body was too large.
        operation: &'static str,
        /// The configured maximum body size in bytes.
        limit: usize,
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
    /// - [`ApiError::BodyTooLarge`] — retrying won't shrink the body.
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
            Self::BadResponse { .. }
            | Self::BodyTooLarge { .. }
            | Self::Auth
            | Self::InvalidToken => false,
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
    /// - [`ApiError::BodyTooLarge`] — like `BadResponse`, this only
    ///   surfaces from a 2xx response whose body exceeded the cap;
    ///   the specific 2xx code isn't preserved.
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
            | Self::BodyTooLarge { .. }
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
            | Self::BadResponse { operation, .. }
            | Self::BodyTooLarge { operation, .. } => Some(operation),
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
            | Self::BodyTooLarge { .. }
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
            | Self::BodyTooLarge { .. }
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
            | Self::BodyTooLarge { .. }
            | Self::Auth
            | Self::InvalidToken => false,
        }
    }
}

/// Render a [`reqwest::Error`] for display with any URL userinfo
/// (embedded credentials) stripped.
///
/// `reqwest::Error`'s own `Display` impl includes the request URL
/// verbatim when one is known, which would echo a credential embedded
/// in the URL (`https://user:pass@host/...`) straight into logs. This
/// redacts the userinfo component before rendering by replacing the
/// raw (credentialed) URL text wherever it appears in the rendered
/// message with a credential-stripped copy — this targets exactly the
/// leak without having to reconstruct reqwest's own message format.
fn redact_reqwest_error(err: &reqwest::Error) -> String {
    let rendered = err.to_string();
    let Some(url) = err.url() else {
        return rendered;
    };
    if url.password().is_none() && url.username().is_empty() {
        return rendered;
    }

    let mut redacted = url.clone();
    // WHY: `set_username`/`set_password` only fail for
    // "cannot-be-a-base" URLs (e.g. `data:`, `mailto:`); a URL that
    // reqwest sent a request to is always `http(s)`, which can always
    // carry userinfo. Degrade to a fixed placeholder instead of
    // panicking on the unreachable error path.
    if redacted.set_username("").is_err() || redacted.set_password(None).is_err() {
        return rendered.replace(url.as_str(), "[redacted]");
    }
    rendered.replace(url.as_str(), redacted.as_str())
}

/// Result alias for keryx API operations.
pub type Result<T> = std::result::Result<T, ApiError>;

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
