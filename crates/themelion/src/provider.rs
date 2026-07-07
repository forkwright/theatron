//! Dioxus theme components: `ThemeProvider` + `ThemeToggle` (feature `dioxus`).

use dioxus::prelude::*;

use crate::theme::ThemeMode;

/// Root theme provider.
///
/// Wraps the component tree with a `div[data-theme]` so CSS custom properties
/// in `themes.css` activate. Provides `Signal<ThemeMode>` as context for
/// descendant components (including `ThemeToggle`).
#[component]
pub fn ThemeProvider(children: Element, initial_mode: Option<ThemeMode>) -> Element {
    let mode = use_signal(|| initial_mode.unwrap_or(ThemeMode::System));
    use_context_provider(|| mode);
    let resolved = use_memo(move || mode().resolve());

    rsx! {
        div {
            "data-theme": resolved().as_str(),
            style: "display: contents",
            {children}
        }
    }
}

const TOGGLE_STYLE: &str = "\
    display: inline-flex; \
    align-items: center; \
    gap: var(--space-2); \
    padding: var(--space-1) var(--space-3); \
    border: 1px solid var(--border); \
    border-radius: var(--radius-md); \
    background: var(--bg-surface); \
    color: var(--text-secondary); \
    font-family: var(--font-body); \
    font-size: var(--text-sm); \
    cursor: pointer; \
    transition: \
        border-color var(--transition-quick), \
        color var(--transition-quick), \
        background-color var(--transition-quick);\
";

/// A button that cycles through theme modes (Dark → Light → System → Dark).
///
/// Reads `Signal<ThemeMode>` from context (provided by [`ThemeProvider`])
/// and advances to the next mode on click. After the mode is advanced,
/// fires `on_change` with the new mode — consumers wire this to their own
/// persistence layer (proskenion writes to settings.toml; chalkeion to
/// its own state dir).
///
/// The callback is optional — pass `EventHandler::default()` (or omit
/// in shorthand form) for surfaces that don't persist.
#[component]
pub fn ThemeToggle(#[props(default)] on_change: EventHandler<ThemeMode>) -> Element {
    let mut mode = use_context::<Signal<ThemeMode>>();
    let current = mode();
    let icon = current.icon();
    let label = current.label();

    rsx! {
        button {
            r#type: "button",
            onclick: move |_| {
                let next = mode().next();
                mode.set(next);
                on_change.call(next);
            },
            title: "Theme: {label}",
            "aria-label": "Switch theme, current: {label}",
            style: TOGGLE_STYLE,
            span { {icon} }
            span { {label} }
        }
    }
}
