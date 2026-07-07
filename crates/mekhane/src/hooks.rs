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

/// Type alias for the tray-icon broadcast sender installed in
/// [`crate::launch_cfg_with_props`].
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
type TrayIconSender = tokio::sync::broadcast::Sender<tray_icon::TrayIconEvent>;

/// Distinct wrapper around the tray-menu broadcast sender installed in
/// [`crate::launch_cfg_with_props`].
///
/// WHY: `tray_icon::menu::MenuEvent` is a re-export of `muda::MenuEvent`
/// (see the crate-level "Menu-event global handler sharing" doc), so a
/// bare `type` alias to `broadcast::Sender<tray_icon::menu::MenuEvent>`
/// is the identical Rust type as a `broadcast::Sender<muda::MenuEvent>`
/// alias — both monomorphize to the same `TypeId`. dioxus's
/// `consume_context`/`provide_context` resolve by `TypeId` against a
/// per-scope `Vec<Box<dyn Any>>`; `provide_context` overwrites the
/// first entry whose *concrete type* matches, so providing both
/// aliased senders would leave only one entry, and both
/// `use_tray_menu_event_handler` and `use_app_menu_event_handler` would
/// silently resolve to the same channel. This newtype gives the
/// tray-menu channel its own `TypeId`, distinct from [`AppMenuSender`].
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[derive(Clone, Debug)]
pub(crate) struct TrayMenuSender(tokio::sync::broadcast::Sender<tray_icon::menu::MenuEvent>);

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
impl TrayMenuSender {
    /// Wrap a tray-menu broadcast sender for type-distinct context storage.
    pub(crate) fn new(tx: tokio::sync::broadcast::Sender<tray_icon::menu::MenuEvent>) -> Self {
        Self(tx)
    }
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
impl std::ops::Deref for TrayMenuSender {
    type Target = tokio::sync::broadcast::Sender<tray_icon::menu::MenuEvent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Distinct wrapper around the app-menu broadcast sender installed in
/// [`crate::launch_cfg_with_props_and_menu`].
///
/// See [`TrayMenuSender`] for why this must be a distinct type rather
/// than a `type` alias to `broadcast::Sender<muda::MenuEvent>`.
#[cfg(all(
    feature = "menus",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
#[derive(Clone, Debug)]
pub(crate) struct AppMenuSender(tokio::sync::broadcast::Sender<muda::MenuEvent>);

#[cfg(all(
    feature = "menus",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
impl AppMenuSender {
    /// Wrap an app-menu broadcast sender for type-distinct context storage.
    pub(crate) fn new(tx: tokio::sync::broadcast::Sender<muda::MenuEvent>) -> Self {
        Self(tx)
    }
}

#[cfg(all(
    feature = "menus",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
impl std::ops::Deref for AppMenuSender {
    type Target = tokio::sync::broadcast::Sender<muda::MenuEvent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Type alias for the global-hotkey broadcast sender installed in
/// [`crate::launch_cfg_with_props`].
#[cfg(all(
    feature = "global-hotkeys",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
type GlobalHotKeySender = tokio::sync::broadcast::Sender<global_hotkey::GlobalHotKeyEvent>;

/// Drive `handler` with every event received on `rx` until the channel
/// closes. Shared receive loop behind all mekhane event hooks.
///
/// - `Ok(event)` — invoke the handler.
/// - `Lagged(n)` — the handler was too slow and `n` events fell out of
///   the broadcast window; warn via [`tracing`] and keep receiving
///   (the subscription stays healthy).
/// - `Closed` — every sender dropped; return, ending the task.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
async fn forward_events<T: Clone>(
    mut rx: tokio::sync::broadcast::Receiver<T>,
    mut handler: impl FnMut(&T),
    source: &'static str,
) {
    loop {
        match rx.recv().await {
            Ok(event) => handler(&event),
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!(
                    target: "mekhane",
                    "{source} handler lagged, {n} event(s) dropped"
                );
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
}

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
pub fn use_tray_icon_event_handler(handler: impl FnMut(&tray_icon::TrayIconEvent) + 'static) {
    let tx = use_hook(consume_context::<TrayIconSender>);
    use_hook(move || {
        spawn(forward_events(tx.subscribe(), handler, "tray_icon"));
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
pub fn use_tray_menu_event_handler(handler: impl FnMut(&tray_icon::menu::MenuEvent) + 'static) {
    let tx = use_hook(consume_context::<TrayMenuSender>);
    use_hook(move || {
        spawn(forward_events(tx.subscribe(), handler, "tray_menu"));
    });
}

/// Register a handler that runs every time a top-of-window application
/// menu item is selected. The handler closure is owned by the calling
/// component; on unmount the underlying task is cancelled.
///
/// # Panics
///
/// Panics if [`crate::launch`] (or one of its variants) was not used
/// to start the app — the app-menu broadcast sender will be missing
/// from dioxus context.
#[cfg(all(
    feature = "menus",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
pub fn use_app_menu_event_handler(handler: impl FnMut(&muda::MenuEvent) + 'static) {
    let tx = use_hook(consume_context::<AppMenuSender>);
    use_hook(move || {
        spawn(forward_events(tx.subscribe(), handler, "app_menu"));
    });
}

/// Register a handler that runs every time a globally registered hotkey
/// is pressed or released. The handler closure is owned by the calling
/// component; on unmount the underlying task is cancelled.
///
/// Consumers must first register hotkeys via the
/// [`global_hotkey::GlobalHotKeyManager`] returned by
/// [`use_global_hotkey_manager`]; this hook only delivers the events.
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
        spawn(forward_events(tx.subscribe(), handler, "global_hotkey"));
    });
}

/// Retrieve the process-global [`global_hotkey::GlobalHotKeyManager`]
/// installed by the launchers, for registering (and unregistering)
/// hotkeys:
///
/// ```no_run
/// use mekhane::hotkey::hotkey::{Code, HotKey};
///
/// # fn component() {
/// let manager = mekhane::use_global_hotkey_manager();
/// dioxus_core::use_hook(move || {
///     let hotkey = HotKey::new(None, Code::KeyK);
///     if let Err(e) = manager.register(hotkey) {
///         tracing::warn!("hotkey registration failed: {e}");
///     }
/// });
/// # }
/// ```
///
/// Pair with [`use_global_hotkey_event_handler`] to receive the
/// triggered events.
///
/// # Panics
///
/// Panics if [`crate::launch`] (or one of its variants) was not used
/// to start the app — the manager will be missing from dioxus context.
#[cfg(all(
    feature = "global-hotkeys",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
#[must_use]
pub fn use_global_hotkey_manager() -> std::sync::Arc<global_hotkey::GlobalHotKeyManager> {
    use_hook(consume_context::<std::sync::Arc<global_hotkey::GlobalHotKeyManager>>)
}

#[cfg(test)]
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
mod tests {
    use std::time::Duration;

    use super::forward_events;

    /// Collects every event a `forward_events` handler sees. Drain the
    /// returned receiver with `try_iter().collect()` after the loop
    /// terminates.
    fn sink() -> (std::sync::mpsc::Receiver<u32>, impl FnMut(&u32)) {
        let (tx, rx) = std::sync::mpsc::channel();
        (rx, move |e: &u32| {
            tx.send(*e).expect("collector receiver alive");
        })
    }

    #[tokio::test]
    async fn forward_events_delivers_then_terminates_on_close() {
        let (tx, rx) = tokio::sync::broadcast::channel::<u32>(8);
        tx.send(1).expect("send with live receiver");
        tx.send(2).expect("send with live receiver");
        drop(tx);

        let (seen, handler) = sink();
        tokio::time::timeout(Duration::from_secs(5), forward_events(rx, handler, "test"))
            .await
            .expect("loop must return once every sender is dropped (Closed)");

        assert_eq!(seen.try_iter().collect::<Vec<_>>(), vec![1, 2]);
    }

    #[tokio::test]
    async fn forward_events_survives_lag_and_keeps_receiving() {
        // Capacity 1: three unpolled sends force Lagged(2) on first
        // recv; the loop must warn-and-continue, then deliver the one
        // surviving event before terminating on Closed.
        let (tx, rx) = tokio::sync::broadcast::channel::<u32>(1);
        tx.send(1).expect("send with live receiver");
        tx.send(2).expect("send with live receiver");
        tx.send(3).expect("send with live receiver");
        drop(tx);

        let (seen, handler) = sink();
        tokio::time::timeout(Duration::from_secs(5), forward_events(rx, handler, "test"))
            .await
            .expect("loop must survive Lagged and terminate on Closed");

        assert_eq!(
            seen.try_iter().collect::<Vec<_>>(),
            vec![3],
            "the subscription must stay healthy after a lag and deliver surviving events"
        );
    }

    #[tokio::test]
    async fn forward_events_fans_out_to_every_subscriber_in_order() {
        let (tx, rx_a) = tokio::sync::broadcast::channel::<u32>(8);
        let rx_b = tx.subscribe();
        for v in [10, 20, 30] {
            tx.send(v).expect("send with live receivers");
        }
        drop(tx);

        let (seen_a, handler_a) = sink();
        let (seen_b, handler_b) = sink();
        tokio::time::timeout(Duration::from_secs(5), async {
            forward_events(rx_a, handler_a, "test_a").await;
            forward_events(rx_b, handler_b, "test_b").await;
        })
        .await
        .expect("both loops must drain and terminate");

        assert_eq!(seen_a.try_iter().collect::<Vec<_>>(), vec![10, 20, 30]);
        assert_eq!(seen_b.try_iter().collect::<Vec<_>>(), vec![10, 20, 30]);
    }

    /// The hooks rely on `use_hook`'s mount-only semantic: the
    /// subscription task is spawned exactly once per component
    /// instance, never on re-render.
    #[test]
    fn use_hook_initializer_runs_only_on_mount() {
        use std::sync::atomic::{AtomicU32, Ordering};

        static CALLS: AtomicU32 = AtomicU32::new(0);

        fn app() -> dioxus_core::Element {
            dioxus_core::use_hook(|| CALLS.fetch_add(1, Ordering::SeqCst));
            dioxus_core::VNode::empty()
        }

        let mut vdom = dioxus_core::VirtualDom::new(app);
        vdom.rebuild_in_place();
        vdom.mark_dirty(dioxus_core::ScopeId::APP);
        vdom.render_immediate(&mut dioxus_core::NoOpMutations);

        assert_eq!(
            CALLS.load(Ordering::SeqCst),
            1,
            "use_hook initializer must run once on mount, not on re-render"
        );
    }

    /// Regression test for the `TrayMenuSender`/`AppMenuSender` type
    /// collision (see the WHY on [`super::TrayMenuSender`]).
    ///
    /// Providing both as root contexts and then consuming each by type
    /// must resolve each to its OWN underlying channel. Before the
    /// newtype fix, both were bare aliases to the identical
    /// `broadcast::Sender<muda::MenuEvent>` monomorphization; dioxus's
    /// `provide_context` overwrites the first context entry whose
    /// concrete type matches, so the second `with_root_context` call
    /// would have silently replaced the first — both
    /// `consume_context::<TrayMenuSender>()` and
    /// `consume_context::<AppMenuSender>()` would then resolve to the
    /// SAME (app-menu) channel.
    #[cfg(feature = "menus")]
    #[test]
    fn tray_menu_and_app_menu_contexts_resolve_independently() {
        use std::cell::RefCell;

        use super::{AppMenuSender, TrayMenuSender, consume_context};

        thread_local! {
            static RESOLVED: RefCell<Option<(TrayMenuSender, AppMenuSender)>> =
                const { RefCell::new(None) };
        }

        fn app() -> dioxus_core::Element {
            let tray_menu = consume_context::<TrayMenuSender>();
            let app_menu = consume_context::<AppMenuSender>();
            RESOLVED.with(|cell| *cell.borrow_mut() = Some((tray_menu, app_menu)));
            dioxus_core::VNode::empty()
        }

        let (tray_tx, _) = tokio::sync::broadcast::channel::<tray_icon::menu::MenuEvent>(4);
        let (app_tx, _) = tokio::sync::broadcast::channel::<muda::MenuEvent>(4);

        let mut vdom = dioxus_core::VirtualDom::new(app)
            .with_root_context(TrayMenuSender::new(tray_tx.clone()))
            .with_root_context(AppMenuSender::new(app_tx.clone()));
        vdom.rebuild_in_place();

        let (resolved_tray, resolved_app) = RESOLVED
            .with(|cell| cell.borrow_mut().take())
            .expect("component must run and resolve both contexts on first render");

        assert!(
            resolved_tray.same_channel(&tray_tx),
            "TrayMenuSender context must resolve to the tray-menu channel"
        );
        assert!(
            !resolved_tray.same_channel(&app_tx),
            "TrayMenuSender context must not collide with the app-menu channel"
        );
        assert!(
            resolved_app.same_channel(&app_tx),
            "AppMenuSender context must resolve to the app-menu channel"
        );
        assert!(
            !resolved_app.same_channel(&tray_tx),
            "AppMenuSender context must not collide with the tray-menu channel"
        );
    }
}
