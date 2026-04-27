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
pub use tray_icon::*;

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
    builder.build().expect("tray icon builder failed")
}

/// Returns a default tray menu containing only a "Quit" item dispatched
/// by the OS.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[must_use]
pub fn default_tray_icon() -> tray_icon::menu::Menu {
    let menu = tray_icon::menu::Menu::new();
    let _ = menu.append_items(&[&tray_icon::menu::PredefinedMenuItem::quit(None)]);
    menu
}
