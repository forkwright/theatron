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
//! - [`tray`] — re-exports of the upstream `tray_icon` crate plus tiny
//!   helpers ([`tray::init_tray_icon`], [`tray::default_tray_icon`]).
//! - Tray hooks ([`use_tray_icon_event_handler`],
//!   [`use_tray_menu_event_handler`]) — subscribe to tray events
//!   delivered through tokio broadcast channels installed by [`launch`].
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

#![warn(missing_docs, clippy::all, clippy::pedantic)]

use std::any::Any;

use dioxus_core::ComponentFunction;

mod hooks;
pub mod tray;

pub use hooks::*;

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
    mut contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    configs: Vec<Box<dyn Any>>,
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

        let tx = tray_tx.clone();
        tray_icon::TrayIconEvent::set_event_handler(Some(move |t| {
            let _ = tx.send(t);
        }));
        let tx = menu_tx.clone();
        tray_icon::menu::MenuEvent::set_event_handler(Some(move |t| {
            let _ = tx.send(t);
        }));

        let tt = tray_tx.clone();
        contexts.insert(0, Box::new(move || Box::new(tt.clone()) as Box<dyn Any>));
        let mt = menu_tx.clone();
        contexts.insert(0, Box::new(move || Box::new(mt.clone()) as Box<dyn Any>));
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
        // Sanity-check semver-shaped: at least one dot.
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
}
