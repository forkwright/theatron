//! Re-export of the upstream [`global_hotkey`] crate.
//!
//! Available when the `global-hotkeys` feature is enabled. Consumers
//! use this to construct [`hotkey::HotKey`] definitions and register
//! them with the [`GlobalHotKeyManager`] that [`crate::launch`] (or
//! any of its variants) provides as a dioxus context. Inside a
//! component, retrieve the manager with
//! [`crate::use_global_hotkey_manager`]:
//!
//! ```no_run
//! use mekhane::hotkey::hotkey::{Code, HotKey};
//!
//! # fn component() {
//! let manager = mekhane::use_global_hotkey_manager();
//! dioxus_core::use_hook(move || {
//!     let hotkey = HotKey::new(None, Code::KeyK);
//!     if let Err(e) = manager.register(hotkey) {
//!         tracing::warn!("hotkey registration failed: {e}");
//!     }
//! });
//! # }
//! ```
//!
//! Receive the triggered events with
//! [`crate::use_global_hotkey_event_handler`].

pub use global_hotkey::*; // kanon:ignore RUST/barrel-reexport -- intentional wholesale re-export of upstream global_hotkey API
