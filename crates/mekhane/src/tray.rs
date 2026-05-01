//! System-tray-icon helpers.
//!
//! Re-exports the upstream [`tray_icon`] crate (which itself re-exports
//! the [`muda`] menu vocabulary at [`tray_icon::menu`]) plus two thin
//! convenience wrappers — [`init_tray_icon`] and [`default_tray_icon`].
//! Pure passthrough; no upstream source modification.
//!
//! For tray-event delivery into dioxus components, use
//! [`crate::use_tray_icon_event_handler`] and
//! [`crate::use_tray_menu_event_handler`].

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub use tray_icon::*; // kanon:ignore RUST/barrel-reexport -- intentional wholesale re-export of upstream tray_icon API

/// Build and return a tray icon. The returned [`tray_icon::TrayIcon`]
/// must be kept alive (e.g., stashed in a hook) for the OS to keep
/// rendering it.
///
/// On Linux/Windows, passing `icon: None` produces an icon-less tray
/// slot (functional but visually empty). On macOS the OS requires an
/// icon — pass `Some(...)` from the consumer.
///
/// # Panics
///
/// Panics if the OS rejects the tray-icon builder (rare; usually a
/// session-bus or permission failure).
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[expect(
    clippy::expect_used,
    reason = "tray icon builder failure is an unrecoverable OS-level error; documented in public API"
)]
#[must_use]
pub fn init_tray_icon(
    menu: tray_icon::menu::Menu,
    icon: Option<tray_icon::Icon>,
) -> tray_icon::TrayIcon {
    let mut builder = tray_icon::TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false);
    if let Some(icon) = icon {
        builder = builder.with_icon(icon);
    }
    builder.build().expect("tray icon builder failed") // kanon:ignore RUST/expect -- unrecoverable OS-level error; documented in public API
}

/// Returns a default tray menu containing only a "Quit" item dispatched
/// by the OS.
///
/// If the OS rejects the menu-item append (rare; usually a session-bus
/// failure on Linux), the returned menu is empty and a `tracing::warn`
/// is logged. Consumers needing a non-empty guarantee should build
/// their own menu and inspect the result.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[must_use]
pub fn default_tray_icon() -> tray_icon::menu::Menu {
    let menu = tray_icon::menu::Menu::new();
    if let Err(e) = menu.append_items(&[&tray_icon::menu::PredefinedMenuItem::quit(None)]) {
        tracing::warn!(
            target: "mekhane",
            "default_tray_icon: failed to append Quit item: {e}"
        );
    }
    menu
}

#[cfg(all(
    feature = "default-icon",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
mod icon {
    use snafu::prelude::*;

    /// Errors that can occur when building a [`tray_icon::Icon`] from raw
    /// image bytes.
    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub))]
    #[non_exhaustive]
    pub enum DefaultIconError {
        /// The provided bytes could not be decoded as an image.
        #[snafu(display("image decode failed: {source}"))]
        Decode {
            /// Underlying image crate error.
            source: image::ImageError,
        },

        /// The decoded RGBA8 buffer was rejected by
        /// `tray_icon::Icon::from_rgba`.
        #[snafu(display("tray icon build failed: {source}"))]
        Build {
            /// Underlying tray-icon error.
            source: tray_icon::BadIcon,
        },
    }

    /// Load a tray (or window) icon from raw image bytes.
    ///
    /// Pass [`include_bytes!("icon.png")`](include_bytes) (or any PNG byte
    /// slice). The `image` crate decodes to RGBA8; the resulting buffer
    /// feeds [`tray_icon::Icon::from_rgba`].
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes cannot be decoded as an image, or if
    /// the resulting RGBA8 buffer is rejected by
    /// [`tray_icon::Icon::from_rgba`] (rare; usually a zero-dimension
    /// image).
    pub fn default_icon(bytes: &[u8]) -> Result<tray_icon::Icon, DefaultIconError> {
        let img =
            image::load_from_memory(bytes).map_err(|source| DefaultIconError::Decode { source })?;
        let rgba = img.to_rgba8();
        let (width, height) = (rgba.width(), rgba.height());
        tray_icon::Icon::from_rgba(rgba.into_raw(), width, height)
            .map_err(|source| DefaultIconError::Build { source })
    }
}

#[cfg(all(
    feature = "default-icon",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
pub use icon::{DefaultIconError, default_icon};

#[cfg(test)]
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn default_tray_icon_returns_menu() {
        // Constructs the default tray menu without panicking.
        // We don't assert on the populated state — the OS may reject
        // PredefinedMenuItem::quit on a CI environment without a
        // session bus — but the function must not panic.
        let _ = default_tray_icon();
    }

    #[cfg(feature = "default-icon")]
    #[test]
    fn default_icon_decodes_valid_png() {
        // Build a 1×1 RGBA PNG dynamically via the `image` crate so the
        // fixture is guaranteed to round-trip with whatever decoder
        // version the workspace happens to be on. Hand-rolled byte
        // arrays bit-rot when image-crate's PNG decoder tightens its
        // chunk validation.
        use std::io::Cursor;

        let img = image::ImageBuffer::from_pixel(1, 1, image::Rgba([255u8, 0, 0, 255]));
        let mut bytes: Vec<u8> = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .expect("encode 1x1 PNG");

        let icon = default_icon(&bytes);
        assert!(
            icon.is_ok(),
            "default_icon should decode a valid 1x1 PNG: {:?}",
            icon.err()
        );
    }

    #[cfg(feature = "default-icon")]
    #[test]
    fn default_icon_rejects_invalid_bytes() {
        let result = default_icon(b"not a png");
        assert!(
            result.is_err(),
            "default_icon should reject invalid image bytes"
        );
    }
}
