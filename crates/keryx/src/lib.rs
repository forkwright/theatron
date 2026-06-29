//! κῆρυξ (keryx, herald/messenger) — HTTP client base, owned SSE
//! wire-protocol parser, API error classification, and URL path
//! helpers for fleet desktop apps.
//!
//! ## Modules
//!
//! - [`sse`] — owned SSE wire-protocol parser. Wraps any
//!   `Stream<Item = Result<Bytes, _>>` and yields parsed
//!   [`SseEvent`] results. Used by chalkeion + future fleet
//!   desktop surfaces to consume kanon-server SSE feeds without
//!   parser duplication.
//! - [`error`] — generic API-layer error type ([`ApiError`]) covering
//!   transport, non-2xx, auth, and invalid-token failure modes for
//!   any fleet HTTP client built on keryx.
//! - [`response`] — response classification ([`ensure_success`](response::ensure_success))
//!   and decode helpers ([`decode_json`](response::decode_json)) that
//!   classify `reqwest::Response` values into [`ApiError`] variants
//!   without per-consumer status-table boilerplate.
//! - [`url`] — URL helpers ([`encode_path_segment`](url::encode_path_segment))
//!   for endpoint construction. RFC 3986 unreserved-character
//!   passthrough, `%XX` uppercase-hex for everything else.

#![deny(missing_docs, clippy::all, clippy::pedantic)]

pub mod error;
pub mod response;
pub mod sse;
pub mod url;

pub use error::{ApiError, Result};
pub use sse::{SseError, SseEvent, SseStream};

#[cfg(test)]
mod smoke_tests {
    use std::net::SocketAddr;

    use bytes::Bytes;
    use futures_util::StreamExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn one_shot(response_bytes: &'static [u8]) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        tokio::spawn(async move {
            let (mut stream, _peer) = listener.accept().await.expect("accept");
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf).await;
            let _ = stream.write_all(response_bytes).await;
            let _ = stream.shutdown().await;
        });
        addr
    }

    #[tokio::test]
    async fn public_modules_smoke() {
        assert_eq!(crate::url::encode_path_segment("a/b"), "a%2Fb");
        assert_eq!(
            crate::url::join_base_path("http://example.test", "v1"),
            "http://example.test/v1"
        );

        assert!(!crate::error::ApiError::Auth.is_retryable());

        let stream = futures_util::stream::empty::<std::result::Result<Bytes, std::io::Error>>();
        let mut sse = crate::sse::SseStream::new(stream);
        assert!(sse.next().await.is_none());

        let addr = one_shot(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n").await;
        let response = reqwest::Client::new()
            .get(format!("http://{addr}/"))
            .send()
            .await
            .expect("send");
        let response = crate::response::ensure_success(response, "smoke")
            .await
            .expect("204 succeeds");
        assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);
    }
}
