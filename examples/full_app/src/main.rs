//! Full theatron consumer example.
//!
//! Demonstrates how the six desktop-bound crates compose in a single
//! runnable Dioxus app:
//!
//! - `themelion` — ThemeProvider + ThemeToggle with Signal binding.
//! - `mekhane` — launch, tray icon, tray menu, app menu (`menus` feature),
//!   global hotkey (`global-hotkeys` feature), default icon (`default-icon`
//!   feature).
//! - `bathron` — settings persistence (theme mode survives restart) and
//!   a desktop notification fired from the hotkey handler.
//! - `skeue` — StatusPill and MetricTile in the layout.
//! - `gramma` — syntect-backed code highlighting wired into a custom
//!   renderer so consumers see the span-to-Dioxus mapping.
//! - `keryx` — stubbed SSE stream that attempts to connect to a
//!   non-existent endpoint and logs the expected ApiError.
//!
//! Settings load happens in `main` before the Dioxus runtime starts,
//! because bathron's Settings I/O is blocking and the app component
//! should receive the already-resolved initial state as a prop.
//!
//! Tray-icon init goes through `use_hook` so it runs exactly once per
//! process, not on every re-render. The tray icon is an OS resource that
//! leaks for the lifetime of the process.

#![cfg_attr(all(not(test), target_os = "windows"), windows_subsystem = "windows")]

use dioxus::prelude::*;

// themelion
use themelion::{ThemeMode, ThemeProvider};

// mekhane
use mekhane::tray::{TrayIconEvent, default_tray_icon, init_tray_icon, menu::MenuEvent};
use mekhane::{use_tray_icon_event_handler, use_tray_menu_event_handler};

#[cfg(feature = "menus")]
use mekhane::tray::menu::{Menu, MenuEvent as AppMenuEvent};
#[cfg(feature = "menus")]
use mekhane::use_app_menu_event_handler;

#[cfg(feature = "global-hotkeys")]
use mekhane::hotkey::GlobalHotKeyEvent;
#[cfg(feature = "global-hotkeys")]
use mekhane::use_global_hotkey_event_handler;

// keryx
use keryx::{ApiError, SseStream};

// bathron
#[cfg(feature = "global-hotkeys")]
use bathron::notifications::{NotificationRequest, send as notify};
use bathron::settings::Settings;

mod body;
use body::Body;

/// App props so the initial theme can be injected from `main`.
#[derive(Clone, Copy, Default)]
struct AppProps {
    initial_theme: Option<ThemeMode>,
}

fn app(props: AppProps) -> Element {
    // Tray init runs once per process via use_hook. The returned TrayIcon
    // is leaked intentionally — the OS keeps it alive until the process exits.
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    use_hook(|| {
        let _ = init_tray_icon(default_tray_icon(), None);
    });

    // Subscribe to tray-icon clicks. Real consumers route these to
    // focus / show / hide window actions.
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    use_tray_icon_event_handler(|event: &TrayIconEvent| {
        tracing::info!("tray icon event: {event:?}");
    });

    // Subscribe to tray-menu selections.
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    use_tray_menu_event_handler(|event: &MenuEvent| {
        tracing::info!("tray menu event: {event:?}");
    });

    // Subscribe to top-of-window application menu selections.
    #[cfg(feature = "menus")]
    use_app_menu_event_handler(|event: &AppMenuEvent| {
        tracing::info!("app menu event: {event:?}");
    });

    // Subscribe to global hotkeys. On trigger, send a desktop notification
    // to demonstrate bathron's notifications surface.
    #[cfg(feature = "global-hotkeys")]
    use_global_hotkey_event_handler(|event: &GlobalHotKeyEvent| {
        tracing::info!("global hotkey event: {event:?}");
        let req = NotificationRequest::new("Hotkey triggered").with_body(format!("{event:?}"));
        if let Err(e) = notify(req) {
            tracing::warn!("notification failed (expected in headless): {e}");
        }
    });

    // Stub SSE consumer: attempts to reach a non-existent endpoint so the
    // error path (ApiError) is exercised without needing a real server.
    use_future(move || async move {
        if let Err(e) = stub_sse_watch().await {
            tracing::info!("sse connect failed (expected): {e}");
        }
    });

    rsx! {
        style { {body::CSS} }
        ThemeProvider {
            initial_mode: props.initial_theme,
            Body {}
        }
    }
}

/// Attempt to open an SSE stream against a dead endpoint.
///
/// Demonstrates the keryx connect pattern: build a reqwest client,
/// wrap the byte stream in `SseStream`, and handle `ApiError` when
/// the server is unreachable.
async fn stub_sse_watch() -> Result<(), ApiError> {
    let client = reqwest::Client::new();
    let resp = client
        .get("http://localhost:9999/events")
        .send()
        .await
        .map_err(|source| ApiError::Http {
            operation: "sse_connect",
            source,
        })?;
    let mut sse = SseStream::new(resp.bytes_stream());
    use futures_util::StreamExt;
    while let Some(event) = sse.next().await {
        tracing::info!("sse event: {} = {}", event.event, event.data);
    }
    Ok(())
}

fn main() {
    // Load the persisted theme before the Dioxus runtime starts.
    // Blocking I/O here keeps the app component free of side effects.
    let initial_theme = Settings::open("theatron-full-app")
        .ok()
        .and_then(|s| s.get::<String>("theme").ok().flatten())
        .map(|label| match label.as_str() {
            "Dark" => ThemeMode::Dark,
            "Light" => ThemeMode::Light,
            _ => ThemeMode::System,
        });

    let props = AppProps { initial_theme };

    #[cfg(feature = "menus")]
    {
        let menu = Menu::new();
        mekhane::launch_cfg_with_props_and_menu(app, props, vec![], vec![], Some(menu));
    }
    #[cfg(not(feature = "menus"))]
    mekhane::launch_cfg_with_props(app, props, vec![], vec![]);
}
