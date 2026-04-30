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

    rsx! { div { "tray smoke" } }
}

fn main() {
    mekhane::launch(app);
}
