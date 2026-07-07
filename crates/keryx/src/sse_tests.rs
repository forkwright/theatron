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

    /// Like [`Self::new`], but takes raw byte chunks instead of
    /// `&str` — lets tests place a chunk boundary at an arbitrary
    /// byte offset, including inside a multi-byte UTF-8 sequence.
    fn from_bytes(chunks: Vec<Vec<u8>>) -> Self {
        Self {
            chunks: chunks.into_iter().map(Bytes::from).collect(),
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

/// Drain a synchronous `SseStream` (all chunks are immediately
/// available so no actual async scheduling is needed) into its
/// parsed events.
fn collect_from_stream<S, E>(mut sse: SseStream<S>) -> Vec<SseEvent>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: std::fmt::Display,
{
    let mut events = Vec::new();
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);

    loop {
        match Pin::new(&mut sse).poll_next(&mut cx) {
            Poll::Ready(Some(Ok(event))) => events.push(event),
            Poll::Ready(Some(Err(err))) => panic!("unexpected transport error: {err}"),
            Poll::Ready(None) => break,
            Poll::Pending => panic!("unexpected Pending from synchronous stream"),
        }
    }
    events
}

/// Collect all events from a list of string chunks.
fn collect_events(chunks: Vec<&str>) -> Vec<SseEvent> {
    collect_from_stream(SseStream::new(ChunkStream::new(chunks)))
}

/// Collect all events from a list of raw byte chunks — used for
/// tests that split a multi-byte UTF-8 sequence across a chunk
/// boundary, where individual chunks are not necessarily valid
/// UTF-8 on their own.
fn collect_events_bytes(chunks: Vec<Vec<u8>>) -> Vec<SseEvent> {
    collect_from_stream(SseStream::new(ChunkStream::from_bytes(chunks)))
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
fn id_field_with_embedded_nul_discarded() {
    // WHATWG SSE spec: an id field value containing U+0000 NULL
    // must be ignored, not set as the last event ID.
    let events = collect_events(vec!["id: abc\0def\ndata: test\n\n"]);
    assert_eq!(events.len(), 1, "expected exactly one event");
    assert!(
        events[0].id.is_none(),
        "NUL-containing id must be discarded per WHATWG SSE spec"
    );
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
fn multibyte_char_split_across_chunk_boundary_decodes_intact() {
    // Regression for #174: decoding each network chunk in
    // isolation via `from_utf8_lossy` corrupts a multi-byte
    // character whose bytes straddle a chunk boundary into
    // U+FFFD replacement characters.
    //
    // é = U+00E9, UTF-8 bytes [0xC3, 0xA9]. Split the payload so
    // the chunk boundary lands between those two bytes.
    let mut chunk1 = b"data: caf".to_vec();
    chunk1.push(0xC3);
    let mut chunk2 = vec![0xA9];
    chunk2.extend_from_slice(b"\n\n");

    let events = collect_events_bytes(vec![chunk1, chunk2]);
    assert_eq!(events.len(), 1, "expected exactly one event");
    assert_eq!(events[0].data, "café");
    assert!(
        !events[0].data.contains('\u{FFFD}'),
        "split multi-byte char must decode intact, got {:?}",
        events[0].data
    );
}

#[test]
fn four_byte_emoji_split_across_chunk_boundary_decodes_intact() {
    // 😀 = U+1F600, UTF-8 bytes [0xF0, 0x9F, 0x98, 0x80]. Split
    // after the first two bytes — well inside the sequence.
    let mut chunk1 = b"data: ".to_vec();
    chunk1.extend_from_slice(&[0xF0, 0x9F]);
    let mut chunk2 = vec![0x98, 0x80];
    chunk2.extend_from_slice(b"\n\n");

    let events = collect_events_bytes(vec![chunk1, chunk2]);
    assert_eq!(events.len(), 1, "expected exactly one event");
    assert_eq!(events[0].data, "😀");
    assert!(
        !events[0].data.contains('\u{FFFD}'),
        "split 4-byte char must decode intact, got {:?}",
        events[0].data
    );
}

#[test]
fn genuinely_invalid_utf8_byte_is_replaced_lossily() {
    // 0xFF is not a valid UTF-8 lead byte anywhere — this is not
    // a chunk-boundary artifact, so it must still be replaced.
    let chunk = [b"data: a".as_slice(), &[0xFF], b"b\n\n".as_slice()].concat();
    let events = collect_events_bytes(vec![chunk]);
    assert_eq!(events.len(), 1, "expected exactly one event");
    assert_eq!(events[0].data, "a\u{FFFD}b");
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
fn crlf_line_endings_split_across_chunks() {
    let events = collect_events(vec!["data: one\r", "\ndata: two\r", "\n\r", "\n"]);
    assert_eq!(events.len(), 1, "expected exactly one event");
    assert_eq!(events[0].data, "one\ntwo");
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

#[test]
fn unterminated_final_line_is_processed_not_dropped() {
    // Regression for #182.1: no trailing newline at all (not even
    // the one `flush_partial_event_on_stream_end` has). Per the
    // WHATWG SSE spec an unterminated final line is still
    // processed as a line when the stream ends, not discarded.
    let events = collect_events(vec!["data: no newline at all"]);
    assert_eq!(
        events.len(),
        1,
        "unterminated final line must still flush as an event"
    );
    assert_eq!(events[0].data, "no newline at all");
}

#[test]
fn unterminated_final_blank_marker_still_flushes_prior_data() {
    // A prior complete `data:` line plus a final unterminated
    // blank-ish fragment: the accumulated data must not be lost.
    let events = collect_events(vec!["data: first\ndata: second"]);
    assert_eq!(events.len(), 1, "expected exactly one event");
    assert_eq!(events[0].data, "first\nsecond");
}

#[test]
fn line_buffer_overflow_yields_typed_error() {
    // Regression for #182.2: a single line that never terminates
    // must not grow the buffer without bound.
    let huge = "a".repeat(MAX_BUFFER_BYTES + 1);
    let stream = ChunkStream::new(vec![huge.as_str()]);
    let mut sse = SseStream::new(stream);
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);

    match Pin::new(&mut sse).poll_next(&mut cx) {
        Poll::Ready(Some(Err(SseError::BufferOverflow { buffer, limit }))) => {
            assert_eq!(buffer, "line");
            assert_eq!(limit, MAX_BUFFER_BYTES);
        }
        other => panic!("expected BufferOverflow, got {other:?}"),
    }
}

#[test]
fn event_data_buffer_overflow_yields_typed_error() {
    // A single `data:` line whose value alone exceeds the limit —
    // the event-payload buffer, not just the line buffer, must be
    // bounded too.
    let mut chunk = String::from("data: ");
    chunk.push_str(&"a".repeat(MAX_BUFFER_BYTES + 1));
    chunk.push('\n');
    let stream = ChunkStream::new(vec![chunk.as_str()]);
    let mut sse = SseStream::new(stream);
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);

    match Pin::new(&mut sse).poll_next(&mut cx) {
        Poll::Ready(Some(Err(SseError::BufferOverflow { buffer, limit }))) => {
            assert_eq!(buffer, "event");
            assert_eq!(limit, MAX_BUFFER_BYTES);
        }
        other => panic!("expected BufferOverflow, got {other:?}"),
    }
}

/// Helper: byte stream that yields its chunks, then a transport
/// error. Used to verify mid-stream failures surface as `Err`
/// items rather than a silent clean EOF.
struct FailingStream {
    chunks: Vec<Bytes>,
    index: usize,
    failed: bool,
}

impl FailingStream {
    fn new(chunks: Vec<&str>) -> Self {
        Self {
            chunks: chunks
                .into_iter()
                .map(|s| Bytes::from(s.to_string()))
                .collect(),
            index: 0,
            failed: false,
        }
    }
}

impl Stream for FailingStream {
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
        } else if this.failed {
            Poll::Ready(None)
        } else {
            this.failed = true;
            Poll::Ready(Some(Err(std::io::Error::other("connection reset"))))
        }
    }
}

#[test]
fn mid_stream_transport_error_surfaces_as_err_not_eof() {
    let stream = FailingStream::new(vec!["data: first\n\n", "data: par"]);
    let mut sse = SseStream::new(stream);
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);

    // First item: the complete event parsed before the failure.
    match Pin::new(&mut sse).poll_next(&mut cx) {
        Poll::Ready(Some(Ok(event))) => assert_eq!(event.data, "first"),
        other => panic!("expected first event, got {other:?}"),
    }

    // Second item: the transport error — NOT a silent clean EOF.
    match Pin::new(&mut sse).poll_next(&mut cx) {
        Poll::Ready(Some(Err(SseError::Transport { message }))) => {
            assert!(
                message.contains("connection reset"),
                "error must carry the underlying message, got {message:?}"
            );
        }
        other => panic!("expected transport error, got {other:?}"),
    }

    // Third item: "data: par" never received a terminating
    // newline, but per the WHATWG SSE spec an unterminated final
    // line is still processed as a line rather than discarded —
    // it salvages as an event once the stream is known to have
    // ended (regression for #182.1).
    match Pin::new(&mut sse).poll_next(&mut cx) {
        Poll::Ready(Some(Ok(event))) => assert_eq!(event.data, "par"),
        other => panic!("expected salvaged unterminated-line event, got {other:?}"),
    }

    // Stream terminates after the salvage.
    assert!(
        matches!(Pin::new(&mut sse).poll_next(&mut cx), Poll::Ready(None)),
        "stream must terminate after salvaging the unterminated line"
    );
}

#[test]
fn buffered_partial_event_flushes_after_transport_error() {
    // A complete `data:` line buffered before the failure is
    // salvaged on the poll after the error is observed.
    let stream = FailingStream::new(vec!["data: salvage\n"]);
    let mut sse = SseStream::new(stream);
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);

    match Pin::new(&mut sse).poll_next(&mut cx) {
        Poll::Ready(Some(Err(SseError::Transport { .. }))) => {}
        other => panic!("expected transport error first, got {other:?}"),
    }
    match Pin::new(&mut sse).poll_next(&mut cx) {
        Poll::Ready(Some(Ok(event))) => assert_eq!(event.data, "salvage"),
        other => panic!("expected flushed partial event, got {other:?}"),
    }
    assert!(matches!(
        Pin::new(&mut sse).poll_next(&mut cx),
        Poll::Ready(None)
    ));
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
    let event = result
        .expect("Ok within deadline")
        .expect("Some event")
        .expect("Ok event");
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
