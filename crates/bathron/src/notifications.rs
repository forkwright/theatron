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

#[cfg(not(test))]
use snafu::ResultExt;
use snafu::Snafu;

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
}

/// Request payload for [`send`].
///
/// Use [`NotificationRequest::new`] then chain [`with_icon`] /
/// [`with_body`] mutators to populate.
///
/// [`with_icon`]: NotificationRequest::with_icon
/// [`with_body`]: NotificationRequest::with_body
#[derive(Debug, Clone)]
pub struct NotificationRequest<'a> {
    /// Notification title (the prominent line).
    pub title: String,
    /// Notification body (the secondary text).
    pub body: String,
    /// Optional icon. Bytes are interpreted as a path-or-name string by
    /// `notify-rust`; for true raw image bytes the platform support
    /// varies, so we treat this as an opaque blob the caller manages.
    pub icon: Option<&'a [u8]>,
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

/// Send a desktop notification.
///
/// # Errors
///
/// Returns [`NotificationError::Send`] if the platform notification
/// service rejects the request (unavailable dbus session,
/// permissions denial, malformed payload).
#[cfg(not(test))]
pub fn send(req: NotificationRequest<'_>) -> Result<NotificationHandle, NotificationError> {
    let mut n = notify_rust::Notification::new();
    n.summary(&req.title);
    if !req.body.is_empty() {
        n.body(&req.body);
    }
    if let Some(icon) = req.icon {
        // notify-rust takes &str; if the caller passed UTF-8 bytes
        // representing a path or freedesktop name, honor it. Otherwise
        // skip silently — raw image bytes aren't part of the desktop
        // notification protocol on most platforms.
        if let Ok(icon_str) = std::str::from_utf8(icon) {
            n.icon(icon_str);
        }
    }
    let inner = n.show().context(SendSnafu)?;
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
}
