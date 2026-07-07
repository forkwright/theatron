//! κῆρυξ (keryx, herald/messenger) — HTTP client base for fleet
//! desktop apps: response classification, owned SSE wire-protocol
//! parsing, API error types, and URL path helpers.
//!
//! ## Modules
//!
//! - [`sse`] — owned SSE wire-protocol parser. Wraps any
//!   `Stream<Item = Result<Bytes, _>>` and yields
//!   `Result<`[`SseEvent`]`, `[`SseError`]`>` items, so mid-stream
//!   transport failures are observable instead of presenting as a
//!   clean end-of-stream. Used by chalkeion + future fleet
//!   desktop surfaces to consume kanon-server SSE feeds without
//!   parser duplication.
//! - [`error`] — generic API-layer error type ([`ApiError`]) covering
//!   transport, non-2xx, auth, and invalid-token failure modes for
//!   any fleet HTTP client built on keryx.
//! - [`response`] — response classification ([`ensure_success`](response::ensure_success))
//!   and decode helpers ([`decode_json`](response::decode_json)) that
//!   make the [`ApiError`] variants reachable from `reqwest::Response`
//!   without per-consumer status-table boilerplate.
//! - [`url`] — URL helpers ([`encode_path_segment`](url::encode_path_segment))
//!   for endpoint construction. RFC 3986 unreserved-character
//!   passthrough, `%XX` uppercase-hex for everything else.
//!
//! ## TLS
//!
//! keryx builds reqwest with `rustls-no-provider` (the canonical fleet
//! TLS stanza): no crypto provider is linked implicitly. Applications
//! must install one before the first TLS connection, exactly once:
//!
//! ```ignore
//! let _ = rustls::crypto::ring::default_provider().install_default();
//! ```
//!
//! Plain-`http` connections need no provider. Skipping the install and
//! then dialing an `https` endpoint panics inside rustls at connection
//! time.

#![deny(missing_docs, clippy::all, clippy::pedantic)]

pub mod error;
pub mod response;
pub mod sse;
pub mod url;

pub use error::{ApiError, Result};
pub use sse::{SseError, SseEvent, SseStream};

/// Install the ring `CryptoProvider` for tests that build a
/// `reqwest::Client` — under `rustls-no-provider`, construction panics
/// without one. `install_default` returns `Err` when a provider is
/// already installed (any test may run first); that is the desired
/// idempotence, not a failure.
#[cfg(test)]
pub(crate) fn install_test_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

#[cfg(test)]
mod smoke_tests {
    use futures_util::StreamExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Cross-module smoke: exercises one real code path from each of
    /// the four public modules (`url`, `error`, `sse`, `response`).
    #[tokio::test]
    async fn public_api_smoke() {
        // url: percent-encoding + slash-boundary join.
        assert_eq!(crate::url::encode_path_segment("a/b"), "a%2Fb");
        assert_eq!(
            crate::url::join_base_path("http://x/", "/v1"),
            "http://x/v1"
        );

        // error: retryability classification.
        assert!(!crate::ApiError::Auth.is_retryable());

        // sse: an empty byte stream terminates cleanly with no events.
        let empty = futures_util::stream::empty::<
            std::result::Result<bytes::Bytes, std::convert::Infallible>,
        >();
        let mut sse = crate::SseStream::new(empty);
        assert!(
            sse.next().await.is_none(),
            "empty stream must yield no events"
        );

        // response: 2xx passes through ensure_success unchanged.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        tokio::spawn(async move {
            let (mut stream, _peer) = listener.accept().await.expect("accept");
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf).await;
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await;
            let _ = stream.shutdown().await;
        });
        crate::install_test_crypto_provider();
        let resp = reqwest::Client::new()
            .get(format!("http://{addr}/"))
            .send()
            .await
            .expect("send");
        let ok = crate::response::ensure_success(resp, "smoke")
            .await
            .expect("2xx passes through");
        assert_eq!(ok.status(), 200);
    }
}
