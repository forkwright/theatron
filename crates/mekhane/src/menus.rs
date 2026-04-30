//! Top-of-window application menus via [`muda`].
//!
//! Re-exports the upstream [`muda`] crate so consumers don't need to
//! pull it in separately, and provides the (consumer-side) install
//! sites for `muda::Menu::init_for_*`. Mekhane itself does NOT install
//! the menu onto the dioxus_native window — reaching the underlying
//! `winit` window handle without forking dioxus_native is not
//! currently possible. Composition over fork.
//!
//! ## Wiring
//!
//! 1. Build a [`muda::Menu`] in the consumer.
//! 2. Hand it to [`muda::Menu::init_for_gtk_window`] /
//!    [`muda::Menu::init_for_hwnd`] / [`muda::Menu::init_for_nsapp`]
//!    once you have the window handle (e.g., from a winit
//!    `WindowEvent::Resumed`-flavored callback if you've taken over
//!    the event loop).
//! 3. Call [`crate::launch`] (or its variants) to start the app —
//!    when the `menus` feature is enabled, the launcher installs the
//!    fan-out menu-event handler that publishes every
//!    [`muda::MenuEvent`] to both the tray-menu and app-menu
//!    broadcasts.
//! 4. In any dioxus component, call
//!    [`crate::use_app_menu_event_handler`] to subscribe.
//!
//! See `crate` top-level docs for the muda single-slot caveat and
//! why both broadcasts receive every menu event.

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub use muda::*;
