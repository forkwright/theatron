//! Response classification and decode helpers.
//!
//! Transforms `reqwest::Response` outputs into typed [`crate::ApiError`]
//! variants so consumers don't hand-roll status classification or
//! response-body parsing per call site.
//!
//! These helpers make the [`ApiError`] variants
//! ([`Auth`](ApiError::Auth), [`RateLimited`](ApiError::RateLimited),
//! [`Server`](ApiError::Server), [`BadResponse`](ApiError::BadResponse))
//! reachable from normal `reqwest::Client::send` call sites without
//! per-consumer status-table boilerplate.

use serde::de::DeserializeOwned;
use snafu::ResultExt;

use crate::error::{ApiError, BadResponseSnafu, HttpSnafu, RateLimitedSnafu, Result, ServerSnafu};

/// Classify a [`reqwest::Response`] by status, returning `Ok(response)`
/// for any 2xx and mapping non-2xx to the appropriate [`ApiError`]
/// variant.
///
/// Status mapping:
///
/// - **2xx** — returned unchanged.
/// - **401 / 403** — [`ApiError::Auth`].
/// - **429** — [`ApiError::RateLimited`] with `retry_after_secs`
///   parsed from the `Retry-After` header when it carries a
///   delta-seconds value (HTTP-date form is not parsed; the field
///   is `None` in that case).
/// - **other non-2xx** — [`ApiError::Server`] with `message`
///   extracted from the response body's top-level `message` or
///   `error` JSON field, falling back to `"<status> <reason>"`
///   (e.g. `"500 Internal Server Error"`).
///
/// Compose with [`decode_json`] when you want a typed DTO from the
/// success body:
///
/// ```no_run
/// use keryx::response::{decode_json, ensure_success};
/// # async fn example(resp: reqwest::Response) -> keryx::Result<()> {
/// let resp = ensure_success(resp, "get_thing").await?;
/// let body: serde_json::Value = decode_json(resp, "get_thing").await?;
/// # let _ = body;
/// # Ok(()) }
/// ```
///
/// # Errors
///
/// - [`ApiError::Auth`] on 401 or 403.
/// - [`ApiError::RateLimited`] on 429.
/// - [`ApiError::Server`] on any other non-2xx status.
pub async fn ensure_success(
    response: reqwest::Response,
    operation: &'static str,
) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(ApiError::Auth);
    }

    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after_secs = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());
        return RateLimitedSnafu {
            operation,
            retry_after_secs,
        }
        .fail();
    }

    let status_code = status.as_u16();
    let reason = status.canonical_reason().unwrap_or("");
    let fallback = format!("{status_code} {reason}").trim().to_string();

    let message = match response.text().await {
        Ok(body) => extract_body_message(&body).unwrap_or(fallback),
        Err(_) => fallback,
    };

    ServerSnafu {
        operation,
        status: status_code,
        message,
    }
    .fail()
}

/// Read a [`reqwest::Response`] body and deserialize it as `T`.
///
/// Body read failure → [`ApiError::Http`]; deserialization failure →
/// [`ApiError::BadResponse`]. Use [`ensure_success`] first when the
/// response status hasn't been validated — this helper assumes the
/// response is a 2xx that should carry a `T`-shaped body.
///
/// # Errors
///
/// - [`ApiError::Http`] if the body read fails.
/// - [`ApiError::BadResponse`] if the body cannot be deserialized
///   into `T` (server schema drift, wrong content type, malformed
///   JSON, etc.).
pub async fn decode_json<T>(response: reqwest::Response, operation: &'static str) -> Result<T>
where
    T: DeserializeOwned,
{
    let body = response.text().await.context(HttpSnafu { operation })?;
    serde_json::from_str(&body).context(BadResponseSnafu { operation })
}

/// Extract a human-readable error message from a JSON response body.
///
/// Looks for top-level `message` or `error` string fields (in that
/// order). Returns `None` when the body isn't JSON, is JSON but has
/// neither field, or the field exists but isn't a string.
fn extract_body_message(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value
        .get("message")
        .or_else(|| value.get("error"))
        .and_then(serde_json::Value::as_str)
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    /// Spin up a one-shot HTTP server that returns the given raw
    /// response bytes for the next incoming connection. Returns the
    /// bound `SocketAddr` so tests can `client.get(format!("http://{addr}/"))`.
    async fn one_shot(response_bytes: &'static [u8]) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        tokio::spawn(async move {
            let (mut stream, _peer) = listener.accept().await.expect("accept");
            let mut buf = [0_u8; 4096];
            let _ = stream.read(&mut buf).await;
            let _ = stream.write_all(response_bytes).await;
            let _ = stream.shutdown().await;
        });
        addr
    }

    async fn get(addr: SocketAddr) -> reqwest::Response {
        reqwest::Client::new()
            .get(format!("http://{addr}/"))
            .send()
            .await
            .expect("send")
    }

    #[tokio::test]
    async fn ensure_success_passes_2xx_through() {
        let addr = one_shot(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi").await;
        let resp = get(addr).await;
        let out = ensure_success(resp, "test").await.expect("2xx ok");
        assert_eq!(out.status(), 200);
    }

    #[tokio::test]
    async fn ensure_success_passes_201_through() {
        let addr = one_shot(b"HTTP/1.1 201 Created\r\nContent-Length: 0\r\n\r\n").await;
        let resp = get(addr).await;
        let out = ensure_success(resp, "test").await.expect("2xx ok");
        assert_eq!(out.status(), 201);
    }

    #[tokio::test]
    async fn ensure_success_maps_401_to_auth() {
        let addr = one_shot(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n").await;
        let resp = get(addr).await;
        let err = ensure_success(resp, "test").await.expect_err("401 fails");
        assert!(matches!(err, ApiError::Auth));
    }

    #[tokio::test]
    async fn ensure_success_maps_403_to_auth() {
        let addr = one_shot(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n").await;
        let resp = get(addr).await;
        let err = ensure_success(resp, "test").await.expect_err("403 fails");
        assert!(matches!(err, ApiError::Auth));
    }

    #[tokio::test]
    async fn ensure_success_maps_429_with_retry_after() {
        let addr = one_shot(
            b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 30\r\nContent-Length: 0\r\n\r\n",
        )
        .await;
        let resp = get(addr).await;
        let err = ensure_success(resp, "test").await.expect_err("429 fails");
        match err {
            ApiError::RateLimited {
                operation,
                retry_after_secs,
            } => {
                assert_eq!(operation, "test");
                assert_eq!(retry_after_secs, Some(30));
            }
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ensure_success_maps_429_without_retry_after() {
        let addr = one_shot(b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\n\r\n").await;
        let resp = get(addr).await;
        let err = ensure_success(resp, "test").await.expect_err("429 fails");
        match err {
            ApiError::RateLimited {
                retry_after_secs, ..
            } => {
                assert_eq!(retry_after_secs, None);
            }
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ensure_success_extracts_message_field_from_500_body() {
        let addr = one_shot(
            b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 27\r\n\r\n{\"message\":\"boom happened\"}",
        )
        .await;
        let resp = get(addr).await;
        let err = ensure_success(resp, "op").await.expect_err("500 fails");
        match err {
            ApiError::Server {
                operation,
                status,
                message,
            } => {
                assert_eq!(operation, "op");
                assert_eq!(status, 500);
                assert_eq!(message, "boom happened");
            }
            other => panic!("expected Server, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ensure_success_extracts_error_field_when_no_message() {
        let addr = one_shot(
            b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 22\r\n\r\n{\"error\":\"alt naming\"}",
        )
        .await;
        let resp = get(addr).await;
        let err = ensure_success(resp, "op").await.expect_err("500 fails");
        match err {
            ApiError::Server { message, .. } => assert_eq!(message, "alt naming"),
            other => panic!("expected Server, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ensure_success_falls_back_to_status_reason_for_non_json_body() {
        let addr =
            one_shot(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 14\r\n\r\nplain text err").await;
        let resp = get(addr).await;
        let err = ensure_success(resp, "op").await.expect_err("502 fails");
        match err {
            ApiError::Server { message, .. } => assert_eq!(message, "502 Bad Gateway"),
            other => panic!("expected Server, got {other:?}"),
        }
    }

    #[derive(serde::Deserialize)]
    struct Person {
        name: String,
    }

    #[tokio::test]
    async fn decode_json_round_trips_valid_body() {
        let addr =
            one_shot(b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\n\r\n{\"name\":\"alice\"}").await;
        let resp = get(addr).await;
        let person: Person = decode_json(resp, "test").await.expect("decode ok");
        assert_eq!(person.name, "alice");
    }

    #[tokio::test]
    async fn decode_json_maps_malformed_to_bad_response() {
        let addr = one_shot(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\nnot a json!").await;
        let resp = get(addr).await;
        let err = decode_json::<serde_json::Value>(resp, "op")
            .await
            .expect_err("malformed fails");
        match err {
            ApiError::BadResponse { operation, .. } => assert_eq!(operation, "op"),
            other => panic!("expected BadResponse, got {other:?}"),
        }
    }

    #[test]
    fn extract_body_message_prefers_message_over_error() {
        let body = r#"{"message": "preferred", "error": "fallback"}"#;
        assert_eq!(extract_body_message(body), Some("preferred".to_string()));
    }

    #[test]
    fn extract_body_message_falls_back_to_error() {
        let body = r#"{"error": "only one"}"#;
        assert_eq!(extract_body_message(body), Some("only one".to_string()));
    }

    #[test]
    fn extract_body_message_returns_none_for_non_json() {
        assert_eq!(extract_body_message("plain text"), None);
    }

    #[test]
    fn extract_body_message_returns_none_when_field_missing() {
        assert_eq!(extract_body_message(r#"{"foo": "bar"}"#), None);
    }

    #[test]
    fn extract_body_message_returns_none_when_field_not_string() {
        assert_eq!(extract_body_message(r#"{"message": 42}"#), None);
    }
}
