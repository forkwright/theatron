//! Owned SSE (Server-Sent Events) parser for reqwest response streams.

use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream;
use snafu::Snafu;

/// Maximum size, in bytes, that [`SseStream`]'s internal line buffer
/// or in-progress event payload may grow to before a terminating
/// delimiter (newline for a line, blank line for an event) arrives.
///
/// Guards against a malformed or hostile server that never
/// terminates a line/event, which would otherwise grow these buffers
/// without bound. 8 `MiB` comfortably covers any legitimate SSE
/// payload (large JSON deltas, base64-encoded chunks) the fleet's SSE
/// feeds emit.
const MAX_BUFFER_BYTES: usize = 8 * 1024 * 1024;

/// Error yielded by [`SseStream`] when the underlying byte stream
/// fails mid-stream.
///
/// Distinguishes transport failure (connection drop, TLS reset,
/// network partition) from clean end-of-stream, so consumers can
/// treat a truncated feed as a failure instead of a complete
/// response.
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum SseError {
    /// The underlying byte stream yielded a transport-level error.
    ///
    /// Events fully parsed before the failure were already yielded;
    /// after this error, any partially-accumulated event is flushed
    /// on the next poll and the stream then terminates.
    #[snafu(display("SSE transport error: {message}"))]
    Transport {
        /// Rendered underlying transport error.
        message: String,
    },

    /// A line or event buffer exceeded [`MAX_BUFFER_BYTES`] without a
    /// terminating newline / blank-line delimiter ever arriving —
    /// most likely a malformed or hostile server that never closes a
    /// line.
    #[snafu(display("SSE {buffer} buffer exceeded {limit} bytes without a delimiter"))]
    BufferOverflow {
        /// Which buffer overflowed: `"line"` (a single unterminated
        /// line grew past the limit) or `"event"` (accumulated
        /// `data:` lines grew past the limit without a blank-line
        /// delimiter).
        buffer: &'static str,
        /// The configured maximum buffer size in bytes
        /// ([`MAX_BUFFER_BYTES`]).
        limit: usize,
    },
}

/// A parsed SSE event from the wire protocol.
// kanon:ignore TOPOLOGY/shallow-struct -- SseEvent is the parsed wire-protocol record: its fields ARE the protocol surface, and construction happens only inside the parser; a smart constructor would add nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// The `event:` field. Defaults to `"message"` per the SSE spec.
    pub event: String,
    /// The `data:` field(s), concatenated with newlines for multi-line data.
    pub data: String,
    /// The `id:` field, if present.
    pub id: Option<String>,
    /// The `retry:` field in milliseconds, if present.
    pub retry: Option<u64>,
}

/// Transforms a byte stream into a stream of parsed SSE events.
///
/// Handles the full SSE wire protocol: `data:`, `event:`, `id:`, `retry:`,
/// comment lines (`:` prefix), multi-line `data:` fields (concatenated with
/// newlines), and blank-line event delimiters.
///
/// Yields `Result<`[`SseEvent`]`, `[`SseError`]`>`: parsed events as
/// `Ok`, and a mid-stream transport failure as a single `Err` item —
/// truncation is observable rather than presenting as a clean
/// end-of-stream. Only genuine end-of-stream (the inner stream
/// returning `None`) terminates silently.
pub struct SseStream<S> {
    stream: S,
    buf: String,
    // WHY: network chunk boundaries never align with UTF-8 character
    // boundaries. `pending_bytes` holds the trailing bytes of an
    // incomplete multi-byte sequence from the previous chunk so it
    // can be completed by the next chunk instead of being decoded
    // (and corrupted into U+FFFD) in isolation.
    pending_bytes: Vec<u8>,
    discard_next_lf: bool,
    done: bool,

    current_event: Option<String>,
    current_data: String,
    current_id: Option<String>,
    current_retry: Option<u64>,
    has_data: bool,
}

impl<S, E> SseStream<S>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: std::fmt::Display,
{
    /// Create a new SSE stream parser wrapping the given byte stream.
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            buf: String::new(),
            pending_bytes: Vec::new(),
            discard_next_lf: false,
            done: false,
            current_event: None,
            current_data: String::new(),
            current_id: None,
            current_retry: None,
            has_data: false,
        }
    }

    /// Process a single SSE line. Returns `Some(SseEvent)` on blank-line
    /// delimiter when accumulated data exists.
    fn process_line(&mut self, line: &str) -> Option<SseEvent> {
        if line.is_empty() {
            if !self.has_data {
                return None;
            }

            // WHY: trailing newline added by multi-line concatenation must be
            // stripped; the SSE spec says "append data + LF" per data: line,
            // but the final LF is not part of the event data.
            if self.current_data.ends_with('\n') {
                self.current_data.pop();
            }

            let event = SseEvent {
                event: self
                    .current_event
                    .take()
                    .unwrap_or_else(|| "message".to_string()),
                data: std::mem::take(&mut self.current_data),
                id: self.current_id.take(),
                retry: self.current_retry.take(),
            };
            self.has_data = false;
            return Some(event);
        }

        // Comment lines start with ':'
        if line.starts_with(':') {
            return None;
        }

        let (field, value) = if let Some((field, rest)) = line.split_once(':') {
            // WHY: SSE spec says "if value starts with a space, remove it"
            let value = rest.strip_prefix(' ').unwrap_or(rest);
            (field, value)
        } else {
            (line, "")
        };

        match field {
            "data" => {
                self.current_data.push_str(value);
                self.current_data.push('\n');
                self.has_data = true;
            }
            "event" => {
                self.current_event = Some(value.to_string());
            }
            "id" => {
                // WHY: WHATWG SSE spec — an id field value containing
                // U+0000 NULL must be ignored, not stored.
                if !value.contains('\0') {
                    self.current_id = Some(value.to_string());
                }
            }
            "retry" => {
                if let Ok(ms) = value.parse::<u64>() {
                    self.current_retry = Some(ms);
                }
            }
            _ => {
                // NOTE: unknown fields are ignored per the SSE spec
            }
        }

        None
    }

    fn push_chunk(&mut self, chunk: &str) {
        for ch in chunk.chars() {
            if self.discard_next_lf {
                self.discard_next_lf = false;
                if ch == '\n' {
                    continue;
                }
            }

            if ch == '\r' {
                self.buf.push('\n');
                self.discard_next_lf = true;
            } else {
                self.buf.push(ch);
            }
        }
    }

    /// Decode a raw byte chunk incrementally, holding back any
    /// trailing incomplete UTF-8 sequence to prepend to the next
    /// chunk before decoding.
    ///
    /// WHY: a network `Bytes` chunk boundary does not align with a
    /// UTF-8 character boundary. Decoding each chunk in isolation via
    /// `String::from_utf8_lossy` corrupts any multi-byte character
    /// (é, emoji, CJK) whose bytes straddle two chunks into
    /// replacement characters. Buffering the undecoded remainder and
    /// combining it with the next chunk's bytes fixes this. Bytes
    /// that are genuinely invalid (not merely incomplete) are still
    /// replaced lossily, one U+FFFD per invalid sequence.
    fn decode_chunk(&mut self, bytes: &[u8]) {
        self.pending_bytes.extend_from_slice(bytes);

        loop {
            match std::str::from_utf8(&self.pending_bytes) {
                Ok(valid) => {
                    let owned = valid.to_string();
                    self.push_chunk(&owned);
                    self.pending_bytes.clear();
                    return;
                }
                Err(err) => {
                    let valid_up_to = err.valid_up_to();
                    if let Some(valid_bytes) = self.pending_bytes.get(..valid_up_to)
                        && let Ok(valid_str) = std::str::from_utf8(valid_bytes)
                        && !valid_str.is_empty()
                    {
                        let owned = valid_str.to_string();
                        self.push_chunk(&owned);
                    }

                    match err.error_len() {
                        None => {
                            // WHY: an incomplete sequence at the end of
                            // the buffer — hold the remaining bytes to
                            // combine with the next chunk rather than
                            // decoding it (and corrupting it) now.
                            self.pending_bytes.drain(..valid_up_to);
                            return;
                        }
                        Some(bad_len) => {
                            // WHY: a genuinely invalid byte sequence
                            // (not merely incomplete) — replace it
                            // lossily and keep decoding the remainder.
                            self.push_chunk("\u{FFFD}");
                            self.pending_bytes
                                .drain(..valid_up_to.saturating_add(bad_len));
                        }
                    }
                }
            }
        }
    }

    /// Flush any bytes held back as an incomplete trailing UTF-8
    /// sequence, replacing them lossily.
    ///
    /// Called when the underlying stream ends — no further bytes
    /// will ever arrive to complete the sequence, so holding it back
    /// any longer would silently drop it.
    fn flush_pending_bytes(&mut self) {
        if self.pending_bytes.is_empty() {
            return;
        }
        let owned = String::from_utf8_lossy(&self.pending_bytes).into_owned();
        self.pending_bytes.clear();
        self.push_chunk(&owned);
    }

    /// Await the next event with a deadline.
    ///
    /// Returns:
    ///
    /// - `Ok(Some(Ok(event)))` — got an event before the deadline.
    /// - `Ok(Some(Err(e)))` — the underlying stream failed mid-stream;
    ///   see [`SseError`].
    /// - `Ok(None)` — the underlying stream terminated cleanly.
    /// - `Err(`[`tokio::time::error::Elapsed`]`)` — the deadline fired
    ///   before the next event arrived. The stream remains internally
    ///   consistent and may be polled again (e.g. with a longer
    ///   timeout, or as a normal `StreamExt::next` call), though bytes
    ///   may already have been consumed from the underlying source
    ///   into the internal buffer.
    ///
    /// Useful for keep-alive / liveness detection on SSE feeds where a
    /// stalled stream is a real condition (server crashed mid-stream,
    /// network partition) and the consumer wants to react instead of
    /// blocking indefinitely.
    ///
    /// Companion to the higher-level disconnect/backoff policy that
    /// remains consumer-side: this helper handles the per-poll deadline,
    /// the consumer decides whether `Elapsed` triggers a reconnect, a
    /// telemetry counter, or a UI signal.
    ///
    /// # Errors
    ///
    /// Returns [`tokio::time::error::Elapsed`] if `deadline` expires
    /// before the underlying byte stream yields enough bytes to
    /// produce the next [`SseEvent`] (or to terminate). The error
    /// carries no data; the stream's internal parse state is
    /// consistent and further polling is safe, but bytes may have
    /// been consumed from the underlying source into the internal
    /// buffer.
    pub async fn next_with_timeout(
        &mut self,
        deadline: std::time::Duration,
    ) -> Result<Option<Result<SseEvent, SseError>>, tokio::time::error::Elapsed> {
        use futures_util::StreamExt;

        tokio::time::timeout(deadline, self.next()).await
    }
}

impl<S, E> Stream for SseStream<S>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: std::fmt::Display,
{
    type Item = Result<SseEvent, SseError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            // Process any complete lines already in the buffer.
            while let Some(pos) = this.buf.find('\n') {
                let line = this.buf.get(..pos).unwrap_or_default().to_string();
                this.buf.drain(..=pos);

                if let Some(event) = this.process_line(&line) {
                    return Poll::Ready(Some(Ok(event)));
                }
            }

            if this.buf.len() > MAX_BUFFER_BYTES {
                this.done = true;
                return Poll::Ready(Some(Err(SseError::BufferOverflow {
                    buffer: "line",
                    limit: MAX_BUFFER_BYTES,
                })));
            }
            if this.current_data.len() > MAX_BUFFER_BYTES {
                this.done = true;
                return Poll::Ready(Some(Err(SseError::BufferOverflow {
                    buffer: "event",
                    limit: MAX_BUFFER_BYTES,
                })));
            }

            if this.done {
                // WHY: WHATWG SSE spec — an unterminated final line at
                // end-of-stream is still processed as a line, not
                // silently discarded merely because no more bytes will
                // ever arrive to terminate it.
                if !this.buf.is_empty() {
                    let line = std::mem::take(&mut this.buf);
                    if let Some(event) = this.process_line(&line) {
                        return Poll::Ready(Some(Ok(event)));
                    }
                }

                // Flush any remaining partial event when the stream ends.
                if this.has_data {
                    if this.current_data.ends_with('\n') {
                        this.current_data.pop();
                    }
                    let event = SseEvent {
                        event: this
                            .current_event
                            .take()
                            .unwrap_or_else(|| "message".to_string()),
                        data: std::mem::take(&mut this.current_data),
                        id: this.current_id.take(),
                        retry: this.current_retry.take(),
                    };
                    this.has_data = false;
                    return Poll::Ready(Some(Ok(event)));
                }
                return Poll::Ready(None);
            }

            match Pin::new(&mut this.stream).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    this.decode_chunk(&bytes);
                }
                Poll::Ready(Some(Err(source))) => {
                    // WHY: a mid-stream transport failure must be
                    // observable — mapping it to end-of-stream would
                    // present truncation as a complete response.
                    this.done = true;
                    this.flush_pending_bytes();
                    return Poll::Ready(Some(Err(SseError::Transport {
                        message: source.to_string(),
                    })));
                }
                Poll::Ready(None) => {
                    this.done = true;
                    this.flush_pending_bytes();
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "test: indices are asserted valid by len checks above each access"
)]
#[path = "sse_tests.rs"]
mod tests;
