//! Re-export of the upstream [`global_hotkey`] crate.
//!
//! Available when the `global-hotkeys` feature is enabled. Consumers
//! use this to construct [`global_hotkey::HotKey`] definitions and
//! register them with the [`global_hotkey::GlobalHotKeyManager`]
//! provided as a dioxus context by [`crate::launch_cfg_with_props`].

pub use global_hotkey::*;
