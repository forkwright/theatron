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

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    feature = "menus",
    feature = "global-hotkeys"
))]
use dioxus_core::{consume_context, spawn, use_hook};
use tokio::sync::broadcast::{self, error::RecvError};

/// Type alias for the tray-icon broadcast sender installed in
/// [`crate::launch_cfg_with_props`].
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
type TrayIconSender = tokio::sync::broadcast::Sender<tray_icon::TrayIconEvent>;

/// Type alias for the tray-menu broadcast sender installed in
/// [`crate::launch_cfg_with_props`].
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
type TrayMenuSender = tokio::sync::broadcast::Sender<tray_icon::menu::MenuEvent>;

/// Type alias for the app-menu broadcast sender installed in
/// [`crate::launch_cfg_with_props_and_menu`].
#[cfg(all(
    feature = "menus",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
type AppMenuSender = tokio::sync::broadcast::Sender<muda::MenuEvent>;

/// Type alias for the global-hotkey broadcast sender installed in
/// [`crate::launch_cfg_with_props`].
#[cfg(all(
    feature = "global-hotkeys",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
type GlobalHotKeySender = tokio::sync::broadcast::Sender<global_hotkey::GlobalHotKeyEvent>;

async fn run_event_handler<T>(
    mut rx: broadcast::Receiver<T>,
    mut handler: impl FnMut(&T) + 'static,
    handler_name: &'static str,
) where
    T: Clone + 'static,
{
    loop {
        match rx.recv().await {
            Ok(event) => handler(&event),
            Err(RecvError::Lagged(n)) => {
                tracing::warn!(
                    target: "mekhane",
                    "{handler_name} handler lagged, {n} event(s) dropped"
                );
            }
            Err(RecvError::Closed) => break,
        }
    }
}

/// Return the global hotkey manager installed by [`crate::launch`].
///
/// Use this hook to register [`global_hotkey::hotkey::HotKey`] values
/// before subscribing with [`crate::use_global_hotkey_event_handler`].
///
/// # Panics
///
/// Panics if [`crate::launch`] (or one of its variants) was not used
/// to start the app — the manager will be missing from dioxus context.
#[cfg(all(
    feature = "global-hotkeys",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
pub fn use_global_hotkey_manager() -> std::sync::Arc<global_hotkey::GlobalHotKeyManager> {
    use_hook(consume_context::<std::sync::Arc<global_hotkey::GlobalHotKeyManager>>)
}

/// Register a handler that runs every time a tray-icon click / move /
/// enter / leave event is dispatched. The handler closure is owned by
/// the calling component; on unmount the underlying task is cancelled.
///
/// # Handler lifetime
///
/// The handler closure is installed at component mount and is not
/// updated on subsequent re-renders. Capture a `Signal<T>`
/// (interior-mutable) rather than a computed local value to observe
/// reactive state changes inside the handler.
///
/// # Panics
///
/// Panics if [`crate::launch`] (or one of its variants) was not used
/// to start the app — the broadcast sender will be missing from
/// dioxus context.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub fn use_tray_icon_event_handler(handler: impl FnMut(&tray_icon::TrayIconEvent) + 'static) {
    let tx = use_hook(consume_context::<TrayIconSender>);
    use_hook(move || {
        spawn(run_event_handler(tx.subscribe(), handler, "tray_icon"));
    });
}

/// Register a handler that runs every time a tray-icon menu item is
/// selected. The handler closure is owned by the calling component;
/// on unmount the underlying task is cancelled.
///
/// # Handler lifetime
///
/// The handler closure is installed at component mount and is not
/// updated on subsequent re-renders. Capture a `Signal<T>`
/// (interior-mutable) rather than a computed local value to observe
/// reactive state changes inside the handler.
///
/// # Panics
///
/// Panics if [`crate::launch`] (or one of its variants) was not used
/// to start the app — the broadcast sender will be missing from
/// dioxus context.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub fn use_tray_menu_event_handler(handler: impl FnMut(&tray_icon::menu::MenuEvent) + 'static) {
    let tx = use_hook(consume_context::<TrayMenuSender>);
    use_hook(move || {
        spawn(run_event_handler(tx.subscribe(), handler, "tray_menu"));
    });
}

/// Register a handler that runs every time a top-of-window application
/// menu item is selected. The handler closure is owned by the calling
/// component; on unmount the underlying task is cancelled.
///
/// # Handler lifetime
///
/// The handler closure is installed at component mount and is not
/// updated on subsequent re-renders. Capture a `Signal<T>`
/// (interior-mutable) rather than a computed local value to observe
/// reactive state changes inside the handler.
///
/// # Panics
///
/// Panics if [`crate::launch_cfg_with_props_and_menu`] was not used
/// to start the app — the broadcast sender will be missing from
/// dioxus context.
#[cfg(all(
    feature = "menus",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
pub fn use_app_menu_event_handler(handler: impl FnMut(&muda::MenuEvent) + 'static) {
    let tx = use_hook(consume_context::<AppMenuSender>);
    use_hook(move || {
        spawn(run_event_handler(tx.subscribe(), handler, "app_menu"));
    });
}

/// Register a handler that runs every time a globally registered hotkey
/// is pressed or released. The handler closure is owned by the calling
/// component; on unmount the underlying task is cancelled.
///
/// Consumers must first register hotkeys via
/// [`global_hotkey::GlobalHotKeyManager::register`]; this hook only
/// delivers the events.
///
/// # Handler lifetime
///
/// The handler closure is installed at component mount and is not
/// updated on subsequent re-renders. Capture a `Signal<T>`
/// (interior-mutable) rather than a computed local value to observe
/// reactive state changes inside the handler.
///
/// # Panics
///
/// Panics if [`crate::launch`] (or one of its variants) was not used
/// to start the app — the broadcast sender will be missing from
/// dioxus context.
#[cfg(all(
    feature = "global-hotkeys",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
pub fn use_global_hotkey_event_handler(
    handler: impl FnMut(&global_hotkey::GlobalHotKeyEvent) + 'static,
) {
    let tx = use_hook(consume_context::<GlobalHotKeySender>);
    use_hook(move || {
        spawn(run_event_handler(tx.subscribe(), handler, "global_hotkey"));
    });
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicU32, Ordering},
        },
        time::Duration,
    };

    use dioxus::prelude::*;
    use dioxus_core::VirtualDom;

    use super::*;

    #[tokio::test]
    async fn receive_loop_exits_when_sender_closes() {
        let (tx, rx) = broadcast::channel::<u32>(1);
        drop(tx);

        let handled = Arc::new(AtomicU32::new(0));
        let handled_by_loop = Arc::clone(&handled);

        tokio::time::timeout(
            Duration::from_secs(1),
            run_event_handler(
                rx,
                move |_| {
                    handled_by_loop.fetch_add(1, Ordering::SeqCst);
                },
                "test",
            ),
        )
        .await
        .expect("closed receiver loop should finish");

        assert_eq!(
            handled.load(Ordering::SeqCst),
            0,
            "closed channel should not deliver events"
        );
    }

    #[tokio::test]
    async fn lagged_receiver_reports_gap_and_continues() {
        let (tx, mut rx) = broadcast::channel(1);

        tx.send(1).expect("receiver should still exist");
        tx.send(2).expect("receiver should still exist");
        tx.send(3).expect("receiver should still exist");

        match rx.recv().await {
            Err(RecvError::Lagged(2)) => {}
            other => panic!("expected two lagged events, got {other:?}"),
        }

        assert_eq!(
            rx.recv().await.expect("subscription should continue"),
            3,
            "receiver should continue with the newest retained event"
        );
    }

    #[tokio::test]
    async fn receive_loop_continues_after_lagged_event() {
        let (tx, rx) = broadcast::channel(1);
        let handled_count = Arc::new(AtomicU32::new(0));
        let handled_count_by_loop = Arc::clone(&handled_count);
        let last_event = Arc::new(AtomicU32::new(0));
        let last_event_by_loop = Arc::clone(&last_event);

        tx.send(1).expect("receiver should still exist");
        tx.send(2).expect("receiver should still exist");
        tx.send(3).expect("receiver should still exist");
        drop(tx);

        run_event_handler(
            rx,
            move |event| {
                handled_count_by_loop.fetch_add(1, Ordering::SeqCst);
                last_event_by_loop.store(*event, Ordering::SeqCst);
            },
            "test",
        )
        .await;

        assert_eq!(
            handled_count.load(Ordering::SeqCst),
            1,
            "lagged loop should handle only the retained event"
        );
        assert_eq!(
            last_event.load(Ordering::SeqCst),
            3,
            "lagged loop should warn, continue, and handle the retained event"
        );
    }

    #[tokio::test]
    async fn broadcast_fans_out_to_multiple_subscribers() {
        let (tx, mut first) = broadcast::channel(4);
        let mut second = tx.subscribe();

        tx.send(10).expect("first receiver should still exist");
        tx.send(20).expect("first receiver should still exist");

        assert_eq!(first.recv().await.expect("first event"), 10);
        assert_eq!(first.recv().await.expect("second event"), 20);
        assert_eq!(second.recv().await.expect("first event"), 10);
        assert_eq!(second.recv().await.expect("second event"), 20);
    }

    #[derive(Clone)]
    struct HookProbe {
        render_value: Arc<AtomicU32>,
        installed_value: Arc<AtomicU32>,
    }

    fn hook_probe_component() -> Element {
        let probe = dioxus_core::consume_context::<HookProbe>();
        let render_value = probe.render_value.fetch_add(1, Ordering::SeqCst) + 1;

        dioxus_core::use_hook({
            let installed_value = Arc::clone(&probe.installed_value);
            move || {
                installed_value.store(render_value, Ordering::SeqCst);
            }
        });

        rsx! { div {} }
    }

    #[test]
    fn use_hook_initializer_runs_only_on_mount() {
        let probe = HookProbe {
            render_value: Arc::new(AtomicU32::new(0)),
            installed_value: Arc::new(AtomicU32::new(0)),
        };
        let render_value = Arc::clone(&probe.render_value);
        let installed_value = Arc::clone(&probe.installed_value);
        let mut dom = VirtualDom::new(hook_probe_component).with_root_context(probe);

        dom.rebuild_in_place();
        assert_eq!(
            installed_value.load(Ordering::SeqCst),
            1,
            "first render should install the first closure"
        );

        dom.mark_all_dirty();
        dom.render_immediate_to_vec();

        assert!(
            render_value.load(Ordering::SeqCst) >= 2,
            "component must re-render and pass a second closure"
        );
        assert_eq!(
            installed_value.load(Ordering::SeqCst),
            1,
            "use_hook must not invoke the replacement closure on rerender"
        );
    }
}
