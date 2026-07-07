//! Desktop notifications via [`notify-rust`].
//!
//! [`send`] dispatches a [`NotificationRequest`] to the platform
//! notification service (libnotify/dbus on Linux, `NSUserNotification`
//! on macOS, Toast on Windows). Returns a [`NotificationHandle`]
//! wrapping the underlying [`notify_rust::NotificationHandle`] so
//! callers can later close the notification.
//!
//! Sending a notification is OS-side I/O; it cannot be unit-tested
//! in CI. The pure-logic portion ([`NotificationRequest`] builder
//! semantics) is covered.
//!
//! [`notify-rust`]: https://docs.rs/notify-rust

use std::sync::mpsc;
use std::time::Duration;

use snafu::{ResultExt, Snafu};

/// Upper bound on how long [`send`] waits for the platform
/// notification service to respond before giving up. Guards against
/// a wedged dbus session (or equivalent platform service) hanging the
/// caller indefinitely — [`notify_rust::Notification::show`] is a
/// synchronous call with no built-in timeout of its own (its
/// `timeout` builder method controls how long the *shown*
/// notification stays on screen, not how long dispatch may block).
pub const DISPATCH_TIMEOUT: Duration = Duration::from_secs(5);

/// Errors from the notifications subsystem.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum NotificationError {
    /// Failed to dispatch the notification to the OS.
    #[snafu(display("failed to send desktop notification: {source}"))]
    Send {
        /// Underlying notify-rust error.
        source: notify_rust::error::Error,
    },

    /// The platform notification service did not respond within
    /// [`DISPATCH_TIMEOUT`]. The background dispatch thread may
    /// still be blocked; the caller is unblocked regardless.
    #[snafu(display("desktop notification dispatch timed out after {timeout:?}"))]
    Timeout {
        /// The configured timeout that elapsed.
        timeout: Duration,
    },
}

/// Request payload for [`send`].
///
/// Use [`NotificationRequest::new`] then chain [`with_icon`] /
/// [`with_body`] mutators to populate.
///
/// [`with_icon`]: NotificationRequest::with_icon
/// [`with_body`]: NotificationRequest::with_body
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NotificationRequest<'a> {
    /// Notification title (the prominent line).
    pub title: String,
    /// Notification body (the secondary text).
    pub body: String,
    /// Optional icon. Bytes are interpreted as a path-or-name string by
    /// `notify-rust`; for true raw image bytes the platform support
    /// varies, so we treat this as an opaque blob the caller manages.
    pub icon: Option<&'a [u8]>, // kanon:ignore RUST/indexing-slicing -- type annotation, not runtime indexing
}

impl<'a> NotificationRequest<'a> {
    /// Construct a new request with the given title. Body is empty,
    /// icon is unset.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: String::new(),
            icon: None,
        }
    }

    /// Set the body text.
    #[must_use]
    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    /// Set the icon bytes (interpreted by the platform — typically a
    /// path or a freedesktop icon name on Linux).
    #[must_use]
    pub fn with_icon(mut self, icon: &'a [u8]) -> Self {
        // kanon:ignore RUST/indexing-slicing -- type annotation, not runtime indexing
        self.icon = Some(icon);
        self
    }
}

/// Handle to a posted notification — wraps
/// [`notify_rust::NotificationHandle`].
///
/// Drop to release the handle without explicit close. Call
/// [`NotificationHandle::close`] to dismiss the notification.
pub struct NotificationHandle {
    inner: notify_rust::NotificationHandle,
}

impl NotificationHandle {
    /// Close the notification (dismisses it from the notification
    /// center on platforms that support that).
    pub fn close(self) {
        self.inner.close();
    }
}

impl std::fmt::Debug for NotificationHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotificationHandle").finish_non_exhaustive()
    }
}

/// Run `dispatch` (blocking I/O) on a background thread and wait at
/// most `timeout` for it to finish.
///
/// Guards a caller against dispatch logic that never returns — a
/// wedged dbus session, in [`send`]'s case. On timeout, the
/// background thread is left running (it may eventually finish and
/// its result is simply dropped by the disconnected channel); the
/// calling thread is unblocked regardless.
///
/// Generic over the dispatched value so the timeout mechanism itself
/// is unit testable without a real notification daemon (no daemon
/// exists in CI — see the module doc); [`send`] instantiates this
/// with the actual `notify_rust` call.
fn dispatch_with_timeout<T, F>(timeout: Duration, dispatch: F) -> Result<T, NotificationError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, notify_rust::error::Error> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        if tx.send(dispatch()).is_err() {
            // NOTE: a send error means the receiver already gave up
            // (timed out) and dropped rx — nobody is listening, so
            // the result has nowhere to go.
        }
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result.context(SendSnafu),
        Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {
            Err(TimeoutSnafu { timeout }.build())
        }
    }
}

/// Send a desktop notification.
///
/// Dispatch is bounded by [`DISPATCH_TIMEOUT`] so a wedged
/// notification daemon cannot hang the caller indefinitely.
///
/// # Errors
///
/// - [`NotificationError::Send`] if the platform notification
///   service rejects the request (unavailable dbus session,
///   permissions denial, malformed payload).
/// - [`NotificationError::Timeout`] if the platform notification
///   service does not respond within [`DISPATCH_TIMEOUT`].
#[cfg(not(test))]
pub fn send(req: NotificationRequest<'_>) -> Result<NotificationHandle, NotificationError> {
    let NotificationRequest { title, body, icon } = req;

    let mut n = notify_rust::Notification::new();
    n.summary(&title);
    if !body.is_empty() {
        n.body(&body);
    }
    if let Some(icon_bytes) = icon {
        // notify-rust takes &str; if the caller passed UTF-8 bytes
        // representing a path or freedesktop name, honor it. Otherwise
        // skip silently — raw image bytes aren't part of the desktop
        // notification protocol on most platforms.
        if let Ok(icon_str) = std::str::from_utf8(icon_bytes) {
            n.icon(icon_str);
        }
    }

    let inner = dispatch_with_timeout(DISPATCH_TIMEOUT, move || n.show())?;
    Ok(NotificationHandle { inner })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_builder_defaults() {
        let req = NotificationRequest::new("hello");
        assert_eq!(req.title, "hello");
        assert!(req.body.is_empty());
        assert!(req.icon.is_none());
    }

    #[test]
    fn request_builder_with_body() {
        let req = NotificationRequest::new("hi").with_body("there");
        assert_eq!(req.body, "there");
    }

    #[test]
    fn request_builder_with_icon() {
        let icon: &[u8] = b"firefox";
        let req = NotificationRequest::new("hi").with_icon(icon);
        assert_eq!(req.icon, Some(icon));
    }

    #[test]
    fn request_builder_chains() {
        let icon: &[u8] = b"dialog-information";
        let req = NotificationRequest::new("title")
            .with_body("body")
            .with_icon(icon);
        assert_eq!(req.title, "title");
        assert_eq!(req.body, "body");
        assert_eq!(req.icon, Some(icon));
    }

    // Regression tests for #185.3: notification dispatch must be
    // bounded by a timeout instead of able to hang the caller
    // indefinitely. send() itself can't be exercised in CI (no
    // notification daemon), so these drive the extracted timeout
    // mechanism directly with controlled dispatch closures.

    #[test]
    fn dispatch_with_timeout_returns_ok_when_dispatch_is_fast() {
        let result = dispatch_with_timeout(Duration::from_secs(1), || Ok(42_u32));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn dispatch_with_timeout_errors_when_dispatch_hangs() {
        // NOTE: the dispatch closure owns both ends of its own
        // channel and blocks on recv() — since it also holds the
        // sender, recv() can never return, so this deadlocks
        // deterministically instead of sleeping a fixed duration to
        // simulate a hang. The blocked helper thread is leaked
        // (acceptable — see dispatch_with_timeout's doc).
        let result: Result<u32, NotificationError> =
            dispatch_with_timeout(Duration::from_millis(20), || {
                let (_never_sent_tx, never_sent_rx) = mpsc::channel::<()>();
                let _blocked_forever = never_sent_rx.recv();
                Ok(0)
            });
        assert!(
            matches!(result, Err(NotificationError::Timeout { .. })),
            "got {result:?}"
        );
    }

    #[test]
    fn timeout_error_displays_the_configured_duration() {
        let err = NotificationError::Timeout {
            timeout: Duration::from_secs(5),
        };
        let display = err.to_string();
        assert!(display.contains("timed out"), "got {display}");
        assert!(display.contains("5s"), "got {display}");
    }

    // INVARIANT: NotificationError must stay Send + Sync so it
    // crosses thread / await boundaries cleanly. Verified at compile
    // time.
    const fn assert_send_sync<T: Send + Sync>() {}
    const _: () = assert_send_sync::<NotificationError>();
}
