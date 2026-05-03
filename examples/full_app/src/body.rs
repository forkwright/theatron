//! UI layout for the full_app example.
//!
//! Demonstrates skeue components (StatusPill, MetricTile) and the
//! gramma::highlight_code wiring into a custom Dioxus renderer.

use dioxus::prelude::*;

use bathron::settings::Settings;
use gramma::{HighlightedSpan, highlight_code};
use skeue::{DeltaDirection, DeltaTone, MetricDelta, MetricTile, StatusPill, StatusPillKind};
use themelion::{ThemeMode, ThemeToggle};

#[expect(non_snake_case)]
pub(crate) fn Body() -> Element {
    // Open settings inside the component for writes. The initial read
    // happened in `main` so the app starts with the correct theme.
    let settings = use_hook(|| Settings::open("theatron-full-app").ok());
    let settings_for_toggle = settings.clone();

    // Highlight a small Rust snippet with gramma to show the syntect
    // wiring. The output is renderer-agnostic Vec<Vec<HighlightedSpan>>;
    // below we map it into Dioxus spans manually.
    let highlighted = use_hook(|| highlight_code(RUST_SNIPPET, "rust"));

    rsx! {
        div { class: "container",
            div { class: "header",
                h1 { "theatron " span { class: "version", {themelion::version()} } }
                ThemeToggle {
                    on_change: move |mode: ThemeMode| {
                        if let Some(ref s) = settings_for_toggle {
                            let _ = s.set("theme", &mode.label());
                        }
                    }
                }
            }

            div { class: "components",
                StatusPill {
                    kind: StatusPillKind::Success,
                    label: "connected".to_string(),
                    icon: Some("\u{25CF}".to_string()),
                }
                MetricTile {
                    value: "42".to_string(),
                    label: "Active sessions".to_string(),
                    delta: Some(MetricDelta {
                        direction: DeltaDirection::Up,
                        label: "+12%".to_string(),
                        tone: DeltaTone::Good,
                    }),
                }
            }

            div { class: "highlight",
                h2 { "gramma::highlight_code output" }
                div { class: "code-block",
                    for (i , line) in highlighted.iter().enumerate() {
                        div { key: "{i}", class: "code-line",
                            for (j , span) in line.iter().enumerate() {
                                span { key: "{j}", style: "{span_style(span)}", "{span.text}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn span_style(span: &HighlightedSpan) -> String {
    let mut style = format!("color: {};", span.color);
    if span.bold {
        style.push_str(" font-weight: bold;");
    }
    if span.italic {
        style.push_str(" font-style: italic;");
    }
    style
}

const RUST_SNIPPET: &str = r#"fn greet(name: &str) -> String {
    format!("hello, {name}")
}"#;

pub(crate) const CSS: &str = r#"
:root,
[data-theme="dark"] {
    --bg: #12110f;
    --bg-surface: #1a1816;
    --bg-surface-dim: #141312;
    --text-primary: #e8e6e3;
    --text-secondary: #b8b4ae;
    --text-muted: #8a8680;
    --border: #2e2b27;
    --accent: #9A7B4F;

    --status-success: #7AA582;
    --status-success-bg: #182018;
    --status-warning: #B07840;
    --status-warning-bg: #221C14;
    --status-error: #B85052;
    --status-error-bg: #2A1818;
    --status-info: #6F8A9B;
    --status-info-bg: #1A2028;

    --aima: #B85052;
    --aima-bg: #2A1818;
    --aporia: #7AA582;
    --aporia-bg: #182018;
    --thanatochromia: #6F5B8A;
    --thanatochromia-bg: #1F1A2A;
    --natural: #B07840;
    --natural-bg: #221C14;

    --space-1: 4px;
    --space-2: 8px;
    --space-3: 12px;
    --space-4: 16px;

    --radius-md: 4px;
    --radius-full: 9999px;
    --radius-lg: 8px;

    --text-xs: 0.75rem;
    --text-sm: 0.833rem;
    --text-2xl: 1.728rem;

    --weight-medium: 500;
    --weight-bold: 700;

    --leading-tight: 1.25;
    --leading-normal: 1.5;

    --font-body: ui-sans-serif, system-ui, sans-serif;
    --font-mono: ui-monospace, SFMono-Regular, monospace;

    --code-bg: #1a1816;
    --code-lang: #8a8680;

    --transition-quick: 150ms ease;
}
html, body, #main { padding: 0; margin: 0; background: var(--bg); color: var(--text-primary); }
body { font-family: var(--font-body); }
.container { padding: var(--space-4); display: flex; flex-direction: column; gap: var(--space-4); }
.header { display: flex; align-items: center; justify-content: space-between; }
h1 { color: var(--accent); margin: 0; }
.version { color: var(--text-muted); font-family: monospace; font-size: var(--text-sm); margin-left: var(--space-2); }
.components { display: flex; gap: var(--space-3); flex-wrap: wrap; align-items: flex-start; }
.highlight h2 { color: var(--text-secondary); font-size: var(--text-sm); margin: 0 0 var(--space-2) 0; }
.code-block { background: var(--code-bg); border: 1px solid var(--border); border-radius: var(--radius-lg); padding: var(--space-3); font-family: var(--font-mono); font-size: var(--text-sm); line-height: var(--leading-normal); overflow-x: auto; }
.code-line { white-space: pre; }
"#;
