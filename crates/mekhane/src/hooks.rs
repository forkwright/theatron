//! Per-component tray-event hooks.
//!
//! Subscribe to the tokio broadcast channels installed by
//! [`crate::launch_cfg_with_props`] via dioxus context. Each hook
//! spawns one task per component instance; the task is cancelled
//! automatically when the component unmounts (dioxus drops scope-bound
//! tasks).
//!
//! ## Lagged subscribers
//!
//! The broadcast channels are sized at 64 events. If a subscriber's
//! handler closure blocks long enough that 64 events queue up,
//! [`tokio::sync::broadcast::Receiver::recv`] returns
//! `Err(RecvError::Lagged(n))`. The hooks log the lag count via
//! [`tracing::warn`] and continue running — events that overflowed
//! the window are dropped, but the subscription survives.
//!
//! Tray events are user-driven (clicks, menu selections), so the
//! 64-event window is comfortable in practice. If a consumer hits
//! sustained Lagged warnings, the handler is too slow — move work
//! onto a separate task.

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
use dioxus_core::{consume_context, spawn, use_hook};

/// Type alias for the tray-icon broadcast sender installed in
/// [`crate::launch_cfg_with_props`].
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
type TrayIconSender = tokio::sync::broadcast::Sender<tray_icon::TrayIconEvent>;

/// Type alias for the tray-menu broadcast sender installed in
/// [`crate::launch_cfg_with_props`].
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
type TrayMenuSender = tokio::sync::broadcast::Sender<tray_icon::menu::MenuEvent>;

/// Register a handler that runs every time a tray-icon click / move /
/// enter / leave event is dispatched. The handler closure is owned by
/// the calling component; on unmount the underlying task is cancelled.
///
/// # Panics
///
/// Panics if [`crate::launch`] (or one of its variants) was not used
/// to start the app — the broadcast sender will be missing from
/// dioxus context.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub fn use_tray_icon_event_handler(mut handler: impl FnMut(&tray_icon::TrayIconEvent) + 'static) {
    let tx = use_hook(consume_context::<TrayIconSender>);
    use_hook(move || {
        let mut rx = tx.subscribe();
        spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => handler(&event),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        // Slow handler — n events dropped from the broadcast
                        // window. Subscription is still healthy; keep going.
                        tracing::warn!(
                            target: "mekhane",
                            "tray_icon handler lagged, {n} event(s) dropped"
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    });
}

/// Register a handler that runs every time a tray-icon menu item is
/// selected. The handler closure is owned by the calling component;
/// on unmount the underlying task is cancelled.
///
/// # Panics
///
/// Panics if [`crate::launch`] (or one of its variants) was not used
/// to start the app — the broadcast sender will be missing from
/// dioxus context.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub fn use_tray_menu_event_handler(mut handler: impl FnMut(&tray_icon::menu::MenuEvent) + 'static) {
    let tx = use_hook(consume_context::<TrayMenuSender>);
    use_hook(move || {
        let mut rx = tx.subscribe();
        spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => handler(&event),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(
                            target: "mekhane",
                            "tray_menu handler lagged, {n} event(s) dropped"
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    });
}
