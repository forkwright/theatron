//! Owned SSE (Server-Sent Events) parser for reqwest response streams.

use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream;

/// A parsed SSE event from the wire protocol.
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
pub struct SseStream<S> {
    stream: S,
    buf: String,
    done: bool,

    current_event: Option<String>,
    current_data: String,
    current_id: Option<String>,
    current_retry: Option<u64>,
    has_data: bool,
    skip_lf_after_cr: bool,
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
            done: false,
            current_event: None,
            current_data: String::new(),
            current_id: None,
            current_retry: None,
            has_data: false,
            skip_lf_after_cr: false,
        }
    }

    fn next_line(&mut self) -> Option<String> {
        if self.skip_lf_after_cr {
            match self.buf.as_bytes().first().copied() {
                Some(b'\n') => {
                    self.buf.drain(..1);
                    self.skip_lf_after_cr = false;
                }
                Some(_) => {
                    self.skip_lf_after_cr = false;
                }
                None => return None,
            }
        }

        let (pos, terminator, next) = {
            let bytes = self.buf.as_bytes();
            let pos = bytes
                .iter()
                .position(|&byte| matches!(byte, b'\r' | b'\n'))?;
            let terminator = bytes.get(pos).copied()?;
            let next = bytes.get(pos + 1).copied();
            (pos, terminator, next)
        };

        let terminator_len = if terminator == b'\r' && next == Some(b'\n') {
            2
        } else {
            if terminator == b'\r' && next.is_none() {
                self.skip_lf_after_cr = true;
            }
            1
        };

        let mut remainder = self.buf.split_off(pos);
        let line = std::mem::take(&mut self.buf);
        remainder.drain(..terminator_len);
        self.buf = remainder;
        Some(line)
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
                self.current_id = Some(value.to_string());
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

    /// Await the next event with a deadline.
    ///
    /// Returns:
    ///
    /// - `Ok(Some(event))` — got an event before the deadline.
    /// - `Ok(None)` — the underlying stream terminated cleanly.
    /// - `Err(`[`tokio::time::error::Elapsed`]`)` — the deadline fired
    ///   before the next event arrived. The stream is unchanged and may
    ///   be polled again (e.g. with a longer timeout, or as a normal
    ///   `StreamExt::next` call).
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
    /// carries no data; the stream is left in its prior state.
    pub async fn next_with_timeout(
        &mut self,
        deadline: std::time::Duration,
    ) -> Result<Option<SseEvent>, tokio::time::error::Elapsed> {
        use futures_util::StreamExt;

        tokio::time::timeout(deadline, self.next()).await
    }
}

impl<S, E> Stream for SseStream<S>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: std::fmt::Display,
{
    type Item = SseEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            // Process any complete lines already in the buffer.
            while let Some(line) = this.next_line() {
                if let Some(event) = this.process_line(&line) {
                    return Poll::Ready(Some(event));
                }
            }

            if this.done {
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
                    return Poll::Ready(Some(event));
                }
                return Poll::Ready(None);
            }

            match Pin::new(&mut this.stream).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    // SAFETY: SSE is a text protocol; invalid UTF-8 is
                    // replaced rather than causing a hard failure.
                    this.buf.push_str(&String::from_utf8_lossy(&bytes));
                }
                Poll::Ready(Some(Err(_)) | None) => {
                    this.done = true;
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
mod tests {
    use super::*;

    /// Helper: creates a byte stream from a list of string chunks.
    struct ChunkStream {
        chunks: Vec<Bytes>,
        index: usize,
    }

    impl ChunkStream {
        fn new(chunks: Vec<&str>) -> Self {
            Self {
                chunks: chunks
                    .into_iter()
                    .map(|s| Bytes::from(s.to_string()))
                    .collect(),
                index: 0,
            }
        }
    }

    impl Stream for ChunkStream {
        type Item = Result<Bytes, std::io::Error>;

        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let this = self.get_mut();
            if this.index < this.chunks.len() {
                #[expect(
                    clippy::indexing_slicing,
                    reason = "bounds checked by the if-guard above"
                )]
                let chunk = this.chunks[this.index].clone();
                this.index += 1;
                Poll::Ready(Some(Ok(chunk)))
            } else {
                Poll::Ready(None)
            }
        }
    }

    /// Collect all events from a chunk stream synchronously (all chunks are
    /// immediately available so no actual async scheduling is needed).
    fn collect_events(chunks: Vec<&str>) -> Vec<SseEvent> {
        let stream = ChunkStream::new(chunks);
        let mut sse = SseStream::new(stream);
        let mut events = Vec::new();
        let waker = std::task::Waker::noop();
        let mut cx = Context::from_waker(waker);

        loop {
            match Pin::new(&mut sse).poll_next(&mut cx) {
                Poll::Ready(Some(event)) => events.push(event),
                Poll::Ready(None) => break,
                Poll::Pending => panic!("unexpected Pending from synchronous stream"),
            }
        }
        events
    }

    #[test]
    fn single_data_event() {
        let events = collect_events(vec!["data: hello\n\n"]);
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert_eq!(events[0].event, "message");
        assert_eq!(events[0].data, "hello");
        assert!(events[0].id.is_none());
    }

    #[test]
    fn multi_line_data_concatenated_with_newline() {
        let events = collect_events(vec!["data: line1\ndata: line2\ndata: line3\n\n"]);
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert_eq!(events[0].data, "line1\nline2\nline3");
    }

    #[test]
    fn event_field_overrides_default() {
        let events = collect_events(vec!["event: custom\ndata: payload\n\n"]);
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert_eq!(events[0].event, "custom");
        assert_eq!(events[0].data, "payload");
    }

    #[test]
    fn comment_lines_skipped() {
        let events = collect_events(vec![": this is a comment\ndata: real\n\n"]);
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert_eq!(events[0].data, "real");
    }

    #[test]
    fn empty_data_event() {
        let events = collect_events(vec!["data:\n\n"]);
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert_eq!(events[0].data, "");
    }

    #[test]
    fn blank_lines_without_data_produce_no_event() {
        let events = collect_events(vec!["\n\n\n"]);
        assert!(
            events.is_empty(),
            "blank lines without data should not emit events"
        );
    }

    #[test]
    fn id_field_captured() {
        let events = collect_events(vec!["id: 42\ndata: test\n\n"]);
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert_eq!(events[0].id.as_deref(), Some("42"));
    }

    #[test]
    fn retry_field_parsed() {
        let events = collect_events(vec!["retry: 3000\ndata: test\n\n"]);
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert_eq!(events[0].retry, Some(3000));
    }

    #[test]
    fn retry_non_numeric_ignored() {
        let events = collect_events(vec!["retry: abc\ndata: test\n\n"]);
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert!(
            events[0].retry.is_none(),
            "non-numeric retry should be ignored"
        );
    }

    #[test]
    fn multiple_events_in_one_chunk() {
        let events = collect_events(vec!["event: a\ndata: first\n\nevent: b\ndata: second\n\n"]);
        assert_eq!(events.len(), 2, "expected two events");
        assert_eq!(events[0].event, "a");
        assert_eq!(events[0].data, "first");
        assert_eq!(events[1].event, "b");
        assert_eq!(events[1].data, "second");
    }

    #[test]
    fn data_split_across_chunks() {
        let events = collect_events(vec!["data: hel", "lo\n\n"]);
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn crlf_line_endings() {
        let events = collect_events(vec!["data: hello\r\n\r\n"]);
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn cr_line_endings() {
        let events = collect_events(vec!["data: hello\r\r"]);
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn event_and_data_combo() {
        let events = collect_events(vec!["event: turn_start\ndata: {\"id\":1}\n\n"]);
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert_eq!(events[0].event, "turn_start");
        assert_eq!(events[0].data, r#"{"id":1}"#);
    }

    #[test]
    fn data_with_no_space_after_colon() {
        let events = collect_events(vec!["data:no-space\n\n"]);
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert_eq!(events[0].data, "no-space");
    }

    #[test]
    fn unknown_fields_ignored() {
        let events = collect_events(vec!["foo: bar\ndata: ok\n\n"]);
        assert_eq!(events.len(), 1, "expected exactly one event");
        assert_eq!(events[0].data, "ok");
    }

    #[test]
    fn flush_partial_event_on_stream_end() {
        let events = collect_events(vec!["data: partial\n"]);
        assert_eq!(events.len(), 1, "partial event should flush on stream end");
        assert_eq!(events[0].data, "partial");
    }

    /// Helper: byte stream that never emits (returns Pending forever).
    /// Used to verify `next_with_timeout` fires Elapsed when nothing
    /// arrives.
    struct NeverStream;

    impl Stream for NeverStream {
        type Item = Result<Bytes, std::io::Error>;

        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Pending
        }
    }

    #[tokio::test]
    async fn next_with_timeout_returns_event_when_in_time() {
        let stream = ChunkStream::new(vec!["data: hello\n\n"]);
        let mut sse = SseStream::new(stream);
        let result = sse
            .next_with_timeout(std::time::Duration::from_millis(50))
            .await;
        let event = result.expect("Ok within deadline").expect("Some event");
        assert_eq!(event.data, "hello");
    }

    #[tokio::test]
    async fn next_with_timeout_returns_none_when_stream_terminates_cleanly() {
        // Empty chunk list → ChunkStream returns Ready(None) immediately
        // → SseStream's terminal flush yields no event (no buffered data).
        let stream = ChunkStream::new(vec![]);
        let mut sse = SseStream::new(stream);
        let result = sse
            .next_with_timeout(std::time::Duration::from_millis(50))
            .await;
        assert!(
            matches!(result, Ok(None)),
            "expected Ok(None) for cleanly-terminated stream, got {result:?}"
        );
    }

    #[tokio::test]
    async fn next_with_timeout_returns_elapsed_when_stream_stalls() {
        let mut sse = SseStream::new(NeverStream);
        let result = sse
            .next_with_timeout(std::time::Duration::from_millis(20))
            .await;
        assert!(
            result.is_err(),
            "expected Err(Elapsed) when stream never emits, got {result:?}"
        );
    }

    #[tokio::test]
    async fn next_with_timeout_stream_remains_polluble_after_elapsed() {
        // After a timeout fires, the stream should still be usable —
        // the helper doesn't consume the stream's state.
        let stream = ChunkStream::new(vec!["data: late\n\n"]);
        let mut sse = SseStream::new(stream);
        // First call: deadline's so short the (immediate) Ready may
        // race the timer, but either Ok(Some(_)) or Err(Elapsed) is
        // valid. The interesting assertion is the second call:
        let _ = sse
            .next_with_timeout(std::time::Duration::from_micros(1))
            .await;
        // Second call with a generous deadline should observe the
        // event (or stream end if the first call already drained it).
        let result = sse
            .next_with_timeout(std::time::Duration::from_millis(50))
            .await;
        assert!(
            result.is_ok(),
            "stream should be polluble after a prior Elapsed, got {result:?}"
        );
    }
}
