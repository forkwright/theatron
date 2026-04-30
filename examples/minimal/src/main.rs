//! Smallest possible theatron-based desktop app.
//!
//! Demonstrates the seed shape of any theatron consumer:
//! - Dioxus 0.7 component tree
//! - dioxus-native (Blitz) renderer
//! - CSS custom properties from kanon DESIGN-TOKENS.md vocabulary

#![cfg_attr(all(not(test), target_os = "windows"), windows_subsystem = "windows")]

use dioxus::prelude::*;
use dioxus_native::launch;

fn app() -> Element {
    rsx! {
        style { {CSS} }
        div {
            class: "container",
            h1 { "theatron " span { class: "version", {themelion::version()} } }
            p { "minimal example — Dioxus 0.7 + Blitz native renderer" }
            div { class: "tokens",
                span { class: "tile aima", "aima" }
                span { class: "tile aporia", "aporia" }
                span { class: "tile thanat", "thanatochromia" }
                span { class: "tile natural", "natural" }
            }
        }
    }
}

const CSS: &str = r#"
:root,
[data-theme="dark"] {
    --bg: #12110f;
    --bg-surface: #1a1816;
    --text-primary: #e8e6e3;
    --text-muted: #8a8680;
    --border: #2e2b27;
    --accent: #9A7B4F;
    --aima: #B85052;
    --aima-bg: #2A1818;
    --aporia: #7AA582;
    --aporia-bg: #182018;
    --thanatochromia: #6F5B8A;
    --thanatochromia-bg: #1F1A2A;
    --natural: #B07840;
    --natural-bg: #221C14;
    --space-2: 8px;
    --space-3: 12px;
    --space-4: 16px;
    --radius-md: 4px;
    --text-sm: 0.833rem;
    --weight-medium: 500;
}
html, body, #main { padding: 0; margin: 0; background: var(--bg); color: var(--text-primary); }
body { font-family: ui-sans-serif, system-ui, sans-serif; }
.container { padding: var(--space-4); }
h1 { color: var(--accent); margin: 0 0 var(--space-3) 0; }
.version { color: var(--text-muted); font-family: monospace; font-size: var(--text-sm); margin-left: var(--space-2); }
p { color: var(--text-muted); margin: 0 0 var(--space-4) 0; }
.tokens { display: flex; gap: var(--space-3); flex-wrap: wrap; }
.tile { padding: var(--space-2) var(--space-3); border-radius: var(--radius-md); font-weight: var(--weight-medium); font-size: var(--text-sm); }
.aima { background: var(--aima-bg); color: var(--aima); border: 1px solid var(--aima); }
.aporia { background: var(--aporia-bg); color: var(--aporia); border: 1px solid var(--aporia); }
.thanat { background: var(--thanatochromia-bg); color: var(--thanatochromia); border: 1px solid var(--thanatochromia); }
.natural { background: var(--natural-bg); color: var(--natural); border: 1px solid var(--natural); }
"#;

fn main() {
    launch(app);
}
