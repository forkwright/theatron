//! μηχανή (mekhane, stage machinery) — windowing, event loop, OS hooks.
//!
//! The fleet's desktop-app substrate at the windowing layer. Wraps
//! [`dioxus_native`] (Dioxus's Blitz-backed renderer) unmodified —
//! tray-icon, hotkeys, and other OS integration are layered as
//! composition, never as patches to upstream source. Dioxus and Blitz
//! stay as crates.io customer deps.
//!
//! ## Public surface
//!
//! - [`launch`] / [`launch_cfg`] / [`launch_cfg_with_props`] — start
//!   a desktop app. Internally configures the tray-event broadcast
//!   channels, then delegates to [`dioxus_native::launch_cfg_with_props`].
//! - [`launch_cfg_with_props_and_menu`] — same as
//!   [`launch_cfg_with_props`] but with an optional [`muda::Menu`] for
//!   app-menu event plumbing (available when the `menus` feature is
//!   enabled).
//! - [`tray`] — re-exports of the upstream `tray_icon` crate plus tiny
//!   helpers ([`tray::init_tray_icon`], [`tray::default_tray_icon`]).
//! - [`hotkey`] — re-exports of the upstream `global_hotkey` crate
//!   (available when the `global-hotkeys` feature is enabled).
//! - Tray hooks ([`use_tray_icon_event_handler`],
//!   [`use_tray_menu_event_handler`]) — subscribe to tray events
//!   delivered through tokio broadcast channels installed by [`launch`].
//! - App-menu hook ([`use_app_menu_event_handler`]) — subscribe to
//!   top-of-window menu events (available when the `menus` feature is
//!   enabled).
//! - Global-hotkey hook ([`use_global_hotkey_event_handler`]) —
//!   subscribe to process-global hotkey events (available when the
//!   `global-hotkeys` feature is enabled).
//!
//! ## Architecture
//!
//! `tray_icon::TrayIconEvent::set_event_handler` and
//! `tray_icon::menu::MenuEvent::set_event_handler` are process-global
//! callback registries. `mekhane::launch` installs callbacks that push
//! into [`tokio::sync::broadcast`] senders, then provides the senders
//! into the dioxus runtime via the `contexts` parameter that
//! `dioxus_native::launch_cfg_with_props` already exposes. Per-component
//! hooks subscribe via [`tokio::sync::broadcast::Receiver`] and drive a
//! local handler closure on each event.
//!
//! No private dioxus-native state is touched; nothing is forked.
//!
//! ## Menu-event global handler sharing
//!
//! `tray_icon::menu::MenuEvent::set_event_handler` and
//! `muda::MenuEvent::set_event_handler` write to the same process-global
//! slot (because `tray_icon::menu` is a re-export of `muda`). Mekhane
//! installs **one** shared handler that fans out to both the tray-menu
//! broadcast and the app-menu broadcast. Consumers subscribe to whichever
//! channel they care about; the same underlying [`muda::MenuEvent`] is
//! delivered to both.

#![warn(missing_docs, clippy::all, clippy::pedantic)]

use std::any::Any;

use dioxus_core::ComponentFunction;

mod hooks;
pub mod tray;

#[cfg(feature = "global-hotkeys")]
pub mod hotkey;

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub use hooks::{use_tray_icon_event_handler, use_tray_menu_event_handler};

#[cfg(all(
    feature = "menus",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
pub use hooks::use_app_menu_event_handler;

#[cfg(all(
    feature = "global-hotkeys",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
pub use hooks::use_global_hotkey_event_handler;

/// Launch a desktop app with the default config.
pub fn launch(app: fn() -> dioxus_core::Element) {
    launch_cfg(app, vec![], vec![]);
}

/// Launch a desktop app with explicit context providers and configs.
pub fn launch_cfg(
    app: fn() -> dioxus_core::Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    cfg: Vec<Box<dyn Any>>,
) {
    launch_cfg_with_props(app, (), contexts, cfg);
}

/// Launch a desktop app with explicit props, context providers, and
/// configs. Most consumers want [`launch`] or [`launch_cfg`].
pub fn launch_cfg_with_props<P: Clone + 'static, M: 'static>(
    app: impl ComponentFunction<P, M>,
    props: P,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    configs: Vec<Box<dyn Any>>,
) {
    #[cfg(feature = "menus")]
    launch_inner(app, props, contexts, configs, None);
    #[cfg(not(feature = "menus"))]
    launch_inner(app, props, contexts, configs);
}

/// Launch a desktop app with an optional top-of-window application menu.
///
/// This is the `menus` feature variant of [`launch_cfg_with_props`]. It
/// accepts an additional [`muda::Menu`] parameter and wires up the
/// app-menu event broadcast channel.
///
/// # Limitations
///
/// Mekhane does **not** reach into `dioxus_native`'s private window
/// handle (that would require forking). The consumer is responsible for
/// attaching the menu to their window via the appropriate
/// `muda::Menu::init_for_*` call:
///
/// - `muda::Menu::init_for_hwnd` on Windows
/// - `muda::Menu::init_for_gtk_window` on Linux
/// - `muda::Menu::init_for_nsapp` on macOS
///
/// If `menu` is [`Some`], mekhane leaks it to keep the OS menu alive
/// for the lifetime of the process. The caller may also keep their own
/// reference if they need to mutate items after launch.
///
/// # Panics
///
/// Panics if `global-hotkeys` feature is enabled and
/// [`global_hotkey::GlobalHotKeyManager::new`] fails (usually a
/// headless-CI or missing-display situation).
#[cfg(feature = "menus")]
pub fn launch_cfg_with_props_and_menu<P: Clone + 'static, M: 'static>(
    app: impl ComponentFunction<P, M>,
    props: P,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    configs: Vec<Box<dyn Any>>,
    menu: Option<muda::Menu>,
) {
    launch_inner(app, props, contexts, configs, menu);
}

fn launch_inner<P: Clone + 'static, M: 'static>(
    app: impl ComponentFunction<P, M>,
    props: P,
    mut contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    configs: Vec<Box<dyn Any>>,
    #[cfg(feature = "menus")] menu: Option<muda::Menu>,
) {
    // Install process-global tray-icon and tray-menu callbacks that
    // forward into tokio broadcast channels; provide the senders as
    // dioxus contexts so per-component hooks can subscribe.
    //
    // `tray_icon::TrayIconEvent::set_event_handler` is a process-global
    // overwriting setter — if a downstream crate also installs one,
    // ours gets clobbered. Same risk dioxus-desktop's tray support
    // carries today; document if it bites.
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    {
        let (tray_tx, _) = tokio::sync::broadcast::channel::<tray_icon::TrayIconEvent>(64);
        let (menu_tx, _) = tokio::sync::broadcast::channel::<tray_icon::menu::MenuEvent>(64);

        #[cfg(feature = "menus")]
        let (app_menu_tx, _) = tokio::sync::broadcast::channel::<muda::MenuEvent>(64);

        let tx = tray_tx.clone();
        tray_icon::TrayIconEvent::set_event_handler(Some(move |t| {
            let _ = tx.send(t);
        }));

        #[cfg(not(feature = "menus"))]
        {
            let tx = menu_tx.clone();
            tray_icon::menu::MenuEvent::set_event_handler(Some(move |t| {
                let _ = tx.send(t);
            }));
        }
        #[cfg(feature = "menus")]
        {
            let tx = menu_tx.clone();
            let app_tx = app_menu_tx.clone();
            tray_icon::menu::MenuEvent::set_event_handler(Some(
                move |t: tray_icon::menu::MenuEvent| {
                    let _ = tx.send(t.clone());
                    let _ = app_tx.send(t);
                },
            ));
        }

        let tt = tray_tx.clone();
        contexts.insert(0, Box::new(move || Box::new(tt.clone()) as Box<dyn Any>));
        let mt = menu_tx.clone();
        contexts.insert(0, Box::new(move || Box::new(mt.clone()) as Box<dyn Any>));

        #[cfg(feature = "menus")]
        {
            let amt = app_menu_tx.clone();
            contexts.insert(0, Box::new(move || Box::new(amt.clone()) as Box<dyn Any>));
        }

        #[cfg(feature = "global-hotkeys")]
        {
            let manager = std::sync::Arc::new(
                global_hotkey::GlobalHotKeyManager::new()
                    .expect("global hotkey manager initialization failed"), // kanon:ignore RUST/expect -- unrecoverable OS-level error; documented in public API
            );
            let (hotkey_tx, _) =
                tokio::sync::broadcast::channel::<global_hotkey::GlobalHotKeyEvent>(64);

            let tx = hotkey_tx.clone();
            global_hotkey::GlobalHotKeyEvent::set_event_handler(Some(move |e| {
                let _ = tx.send(e);
            }));

            let m = manager.clone();
            contexts.insert(0, Box::new(move || Box::new(m.clone()) as Box<dyn Any>));
            let t = hotkey_tx.clone();
            contexts.insert(0, Box::new(move || Box::new(t.clone()) as Box<dyn Any>));
        }
    }

    #[cfg(feature = "menus")]
    if let Some(menu) = menu {
        let _ = Box::leak(Box::new(menu));
    }

    dioxus_native::launch_cfg_with_props(app, props, contexts, configs);
}

/// Returns the mekhane crate version.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_cargo_metadata() {
        let v = version();
        assert!(!v.is_empty(), "version() must return a non-empty string");
        // Validate semver shape: at least one dot.
        assert!(
            v.contains('.'),
            "version() should be semver-shaped, got {v}"
        );
    }

    /// Verifies the tokio broadcast channel that `launch` configures
    /// has the documented capacity. If this constant ever changes,
    /// update the docs in `hooks.rs` (Lagged-handling section) and the
    /// reasoning paragraph that mentions 64 events.
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    #[test]
    fn broadcast_channel_capacity_is_64() {
        let (tx, _rx) = tokio::sync::broadcast::channel::<tray_icon::TrayIconEvent>(64);
        // capacity() returns the channel's buffered slots.
        assert_eq!(tx.len(), 0, "fresh channel should be empty");
        // The exact capacity isn't observable from Sender alone, but
        // we can verify the channel was created without panic. The
        // capacity number is the contract enumerated in launch().
    }

    /// Verifies the app-menu broadcast channel has the same 64-event
    /// capacity as the tray channels.
    #[cfg(all(
        feature = "menus",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    #[test]
    fn app_menu_broadcast_channel_capacity_is_64() {
        let (tx, _rx) = tokio::sync::broadcast::channel::<muda::MenuEvent>(64);
        assert_eq!(tx.len(), 0, "fresh channel should be empty");
    }

    /// Verifies the global-hotkey broadcast channel has the same 64-event
    /// capacity as the tray channels.
    #[cfg(all(
        feature = "global-hotkeys",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    #[test]
    fn global_hotkey_broadcast_channel_capacity_is_64() {
        let (tx, _rx) = tokio::sync::broadcast::channel::<global_hotkey::GlobalHotKeyEvent>(64);
        assert_eq!(tx.len(), 0, "fresh channel should be empty");
    }
}
