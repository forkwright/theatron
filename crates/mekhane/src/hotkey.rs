//! Re-export of the upstream [`global_hotkey`] crate.
//!
//! Available when the `global-hotkeys` feature is enabled. Consumers
//! use this to construct `global_hotkey::hotkey::HotKey` definitions
//! and register them with the [`global_hotkey::GlobalHotKeyManager`]
//! provided as a dioxus context by [`crate::launch_cfg_with_props`].
//!
//! Most consumers should call [`crate::use_global_hotkey_manager`] from
//! a component, register the desired hotkey once, and subscribe to
//! events with [`crate::use_global_hotkey_event_handler`]:
//!
//! ```no_run
//! use dioxus::prelude::*;
//! use mekhane::{
//!     hotkey::{
//!         hotkey::{Code, HotKey, Modifiers},
//!         GlobalHotKeyEvent,
//!     },
//!     use_global_hotkey_event_handler, use_global_hotkey_manager,
//! };
//!
//! fn hotkeys() -> Element {
//!     let manager = use_global_hotkey_manager();
//!     use_hook(move || {
//!         let hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::KeyK);
//!         if let Err(error) = manager.register(hotkey) {
//!             let _ = error;
//!         }
//!     });
//!
//!     use_global_hotkey_event_handler(|event: &GlobalHotKeyEvent| {
//!         let _ = event;
//!     });
//!
//!     rsx! { div {} }
//! }
//! ```
//!
//! Lower-level code can retrieve the same manager directly from dioxus
//! context:
//!
//! ```no_run
//! use std::sync::Arc;
//!
//! use dioxus_core::{consume_context, use_hook};
//! use mekhane::hotkey::GlobalHotKeyManager;
//!
//! let _manager = use_hook(consume_context::<Arc<GlobalHotKeyManager>>);
//! ```

pub use global_hotkey::*; // kanon:ignore RUST/barrel-reexport -- intentional wholesale re-export of upstream global_hotkey API
