//! Compile-time example for registering and receiving a global hotkey.

#![cfg_attr(all(not(test), target_os = "windows"), windows_subsystem = "windows")]

#[cfg(all(
    feature = "global-hotkeys",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
use dioxus::prelude::*;
#[cfg(all(
    feature = "global-hotkeys",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
use mekhane::{
    hotkey::{
        GlobalHotKeyEvent,
        hotkey::{Code, HotKey, Modifiers},
    },
    use_global_hotkey_event_handler, use_global_hotkey_manager,
};

#[cfg(all(
    feature = "global-hotkeys",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
fn app() -> Element {
    let manager = use_global_hotkey_manager();
    use_hook(move || {
        let hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::KeyK);
        if let Err(error) = manager.register(hotkey) {
            let _ = error;
        }
    });

    use_global_hotkey_event_handler(|event: &GlobalHotKeyEvent| {
        let _ = event;
    });

    rsx! { div { "hotkey registration" } }
}

#[cfg(all(
    feature = "global-hotkeys",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
fn main() {
    mekhane::launch(app);
}

#[cfg(not(all(
    feature = "global-hotkeys",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
)))]
fn main() {}
