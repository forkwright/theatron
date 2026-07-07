//! System-tray-icon helpers.
//!
//! Re-exports the upstream [`tray_icon`] crate (which itself re-exports
//! the `muda` menu vocabulary at [`tray_icon::menu`]) plus two thin
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
    use std::io::Cursor;

    use snafu::prelude::*;

    /// Errors that can occur when building a [`tray_icon::Icon`] from raw
    /// image bytes.
    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub))]
    #[non_exhaustive]
    pub enum DefaultIconError {
        /// The provided bytes could not be decoded as an image, or the
        /// image header declared dimensions above `MAX_ICON_DIMENSION`.
        #[snafu(display("image decode failed: {source}"))]
        Decode {
            /// Underlying image crate error. A dimension-cap violation
            /// surfaces as `image::ImageError::Limits`.
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

    /// Maximum tray/window icon width and height, in pixels, accepted by
    /// [`default_icon`].
    ///
    /// SECURITY: enforced via `image::Limits` on the decoder before the
    /// RGBA8 pixel buffer is allocated. Tray icons are small OS-chrome
    /// elements (typically 16-256px); without this cap, an image header
    /// claiming dimensions far beyond that would size the allocation off
    /// attacker-controlled header fields before decoding ever fails.
    const MAX_ICON_DIMENSION: u32 = 4096;

    /// Load a tray (or window) icon from raw image bytes.
    ///
    /// Pass [`include_bytes!("icon.png")`](include_bytes) (or any PNG byte
    /// slice). The `image` crate decodes to RGBA8; the resulting buffer
    /// feeds [`tray_icon::Icon::from_rgba`].
    ///
    /// The image header's declared width and height are checked against
    /// `MAX_ICON_DIMENSION` (4096px) before the pixel buffer is
    /// allocated, so a crafted header cannot force an oversized
    /// allocation.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes cannot be decoded as an image, if
    /// the header declares width or height above `MAX_ICON_DIMENSION`,
    /// or if the resulting RGBA8 buffer is rejected by
    /// [`tray_icon::Icon::from_rgba`] (rare; usually a zero-dimension
    /// image).
    pub fn default_icon(bytes: &[u8]) -> Result<tray_icon::Icon, DefaultIconError> {
        let mut reader = image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|source| DefaultIconError::Decode {
                source: image::ImageError::from(source),
            })?;
        // NOTE: `image::Limits` is `#[non_exhaustive]`, so it cannot be
        // struct-literal constructed outside the `image` crate (even
        // with `..Default::default()`) -- mutate the public fields on a
        // default-constructed value instead.
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(MAX_ICON_DIMENSION);
        limits.max_image_height = Some(MAX_ICON_DIMENSION);
        reader.limits(limits);
        let img = reader
            .decode()
            .map_err(|source| DefaultIconError::Decode { source })?;
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
    fn default_tray_icon_returns_menu_with_at_most_quit_item() {
        // Documented contract: a menu containing the single Quit item,
        // or an empty menu when the OS rejects the append (headless CI
        // without a session bus). Anything else is a regression.
        let menu = default_tray_icon();
        let items = menu.items();
        assert!(
            items.len() <= 1,
            "default menu must hold at most the Quit item, got {} items",
            items.len()
        );
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

    /// Security regression: a crafted image header claiming huge
    /// dimensions must be rejected via the dimension-limit path (before
    /// the RGBA8 buffer is allocated), not merely "some decode error".
    ///
    /// Builds a valid, tiny PNG, then splices a crafted IHDR
    /// width/height declaring dimensions far above the 4096px cap,
    /// recomputing the chunk CRC so the splice is a plausible header
    /// (not corrupt framing) — the only thing wrong with the input is
    /// the claimed dimensions.
    #[cfg(feature = "default-icon")]
    #[test]
    fn default_icon_rejects_oversized_header_dimensions() {
        // PNG layout: [8-byte signature][4-byte length][4-byte type =
        // "IHDR"][4-byte width][4-byte height][5 more IHDR bytes][4-byte
        // CRC over type+data]. IHDR is always the first chunk.
        const SIGNATURE_LEN: usize = 8;
        const LENGTH_FIELD_LEN: usize = 4;
        const CHUNK_TYPE_LEN: usize = 4;
        const IHDR_DATA_LEN: usize = 13;

        use std::io::Cursor;

        let img = image::ImageBuffer::from_pixel(1, 1, image::Rgba([255u8, 0, 0, 255]));
        let mut bytes: Vec<u8> = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .expect("encode 1x1 PNG");

        let width_offset = SIGNATURE_LEN + LENGTH_FIELD_LEN + CHUNK_TYPE_LEN;
        let oversized: u32 = 1_000_000; // far above the 4096px cap
        bytes[width_offset..width_offset + 4].copy_from_slice(&oversized.to_be_bytes());
        bytes[width_offset + 4..width_offset + 8].copy_from_slice(&oversized.to_be_bytes());

        let ihdr_start = SIGNATURE_LEN + LENGTH_FIELD_LEN;
        let ihdr_len = CHUNK_TYPE_LEN + IHDR_DATA_LEN;
        let crc = crc32(&bytes[ihdr_start..ihdr_start + ihdr_len]);
        let crc_offset = ihdr_start + ihdr_len;
        bytes[crc_offset..crc_offset + 4].copy_from_slice(&crc.to_be_bytes());

        let result = default_icon(&bytes);
        assert!(
            result.is_err(),
            "default_icon must reject a header claiming {oversized}x{oversized} pixels"
        );
        let err = result.expect_err("checked is_err above");
        assert!(
            matches!(
                &err,
                DefaultIconError::Decode {
                    source: image::ImageError::Limits(_),
                }
            ),
            "rejection must be the dimension-limit path, not a generic decode failure: {err:?}"
        );
    }

    /// Minimal CRC-32/ISO-HDLC (the PNG chunk checksum) so the oversized-
    /// header test above can craft a structurally valid chunk without
    /// pulling in a CRC dependency for one test.
    #[cfg(feature = "default-icon")]
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for &byte in bytes {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                let mask = if crc & 1 == 0 { 0 } else { 0xEDB8_8320 };
                crc = (crc >> 1) ^ mask;
            }
        }
        !crc
    }
}
