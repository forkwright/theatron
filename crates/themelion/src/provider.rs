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
///
/// WHY: mounted outside a [`ThemeProvider`] ancestor there is no
/// `Signal<ThemeMode>` context to read or cycle. Rather than panic
/// (the library-code no-panic standard), this degrades to an empty
/// render — a misuse of the component still leaves the rest of the
/// host app's render tree intact (#188).
#[component]
pub fn ThemeToggle(#[props(default)] on_change: EventHandler<ThemeMode>) -> Element {
    let Some(mut mode) = try_use_context::<Signal<ThemeMode>>() else {
        return rsx! {};
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    // WHY: `ThemeToggle`'s `on_change` prop defaults via `Callback::new`,
    // which calls `Runtime::current()` — evaluating `rsx! { ThemeToggle {} }`
    // at a test's top level (no active Dioxus runtime) panics before the
    // component under test even runs. Wrapping construction in an app
    // function defers evaluation into `rebuild_in_place()`, which is
    // inside the runtime, matching how these components are actually used.
    fn app_without_provider() -> Element {
        rsx! {
            ThemeToggle {}
        }
    }

    fn app_with_provider() -> Element {
        rsx! {
            ThemeProvider {
                ThemeToggle {}
            }
        }
    }

    #[test]
    fn theme_toggle_outside_provider_does_not_panic() {
        // WHY (#188): `ThemeToggle` previously called `use_context`, which
        // panics without an ancestor `ThemeProvider`. Dioxus's own
        // component rendering can swallow a panic before it propagates
        // out of `rebuild_in_place`, so asserting on the HTML output
        // alone cannot distinguish "no panic" from "panic caught
        // internally." Install a panic hook for the scope of this render
        // to observe the underlying panic directly, then assert none
        // occurred.
        use std::panic::{self, AssertUnwindSafe};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let panicked = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&panicked);
        let previous_hook = panic::take_hook();
        panic::set_hook(Box::new(move |_| flag.store(true, Ordering::SeqCst)));

        let render_result = panic::catch_unwind(AssertUnwindSafe(|| {
            let mut vdom = VirtualDom::new(app_without_provider);
            vdom.rebuild_in_place();
            dioxus_ssr::render(&vdom)
        }));

        panic::set_hook(previous_hook);

        assert!(
            !panicked.load(Ordering::SeqCst),
            "ThemeToggle must not panic when rendered outside ThemeProvider (#188)"
        );
        let html = render_result.expect("render must not unwind past rebuild_in_place");
        assert!(
            html.trim().is_empty(),
            "expected an empty render outside ThemeProvider, got: {html}"
        );
    }

    #[test]
    fn theme_toggle_inside_provider_still_renders_the_button() {
        let mut vdom = VirtualDom::new(app_with_provider);
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(html.contains("<button"), "expected button markup: {html}");
        assert!(
            html.contains("Switch theme"),
            "expected aria-label text: {html}"
        );
    }

    #[test]
    fn theme_provider_applies_data_theme_attribute() {
        fn app() -> Element {
            rsx! {
                ThemeProvider {
                    initial_mode: Some(ThemeMode::Dark),
                    span { "content" }
                }
            }
        }
        let mut vdom = VirtualDom::new(app);
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(
            html.contains(r#"data-theme="dark""#),
            "expected data-theme attribute: {html}"
        );
    }
}
