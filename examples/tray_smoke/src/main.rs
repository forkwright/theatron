//! Compile-time smoke test for `mekhane`'s public API.
//!
//! This example exists to catch regressions in the
//! `mekhane::launch` + `use_tray_icon_event_handler` +
//! `use_tray_menu_event_handler` chain. It builds + passes clippy in
//! CI but is not meant to be `cargo run` — running it would open a
//! real desktop window and tray icon.
//!
//! If this stops compiling, the breakage is in `mekhane`'s public
//! surface and downstream consumers (proskenion, future chalkeion)
//! will also break.

#![cfg_attr(all(not(test), target_os = "windows"), windows_subsystem = "windows")]

use dioxus::prelude::*;
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

fn app() -> Element {
    use_hook(|| init_tray_icon(default_tray_icon(), None));

    use_tray_icon_event_handler(|event: &TrayIconEvent| {
        // Real consumers would route to focus/show/hide here; smoke
        // test just exercises the type signature.
        let _ = event;
    });

    use_tray_menu_event_handler(|event: &MenuEvent| {
        let _ = event;
    });

    #[cfg(feature = "menus")]
    use_app_menu_event_handler(|event: &AppMenuEvent| {
        let _ = event;
    });

    #[cfg(feature = "global-hotkeys")]
    use_global_hotkey_event_handler(|event: &GlobalHotKeyEvent| {
        let _ = event;
    });

    rsx! { div { "tray smoke" } }
}

fn main() {
    #[cfg(feature = "menus")]
    {
        let menu = Menu::new();
        mekhane::launch_cfg_with_props_and_menu(app, (), vec![], vec![], Some(menu));
    }
    #[cfg(not(feature = "menus"))]
    mekhane::launch(app);
}
