//! Empty state — placeholder for views with no content.
//!
//! Per DESIGN-TOKENS.md component anatomy:
//! - Structure: optional icon + title + optional message + optional action
//! - Size: fills the parent container (centered)
//! - Token use: `--text-muted` for icon and message, `--text-primary` for title
//! - Padding: `--space-6` block, centered alignment
//! - Text: title `--text-lg --weight-medium`, message `--text-sm`
//!
//! References (folds in kanon discussion docket #40):
//! - Linear empty states: large icon + title + secondary text + CTA button
//! - GitHub repo empty: octicon + title + descriptive prose + action link
//! - Fly.io app dashboard: glyph + headline + small body + button
//! - Sourcehut empty repo: minimal — title + one-line message + clone instructions

use dioxus::prelude::*;

const EMPTY_STATE_STYLE: &str = "\
    display: flex; \
    flex-direction: column; \
    align-items: center; \
    justify-content: center; \
    gap: var(--space-3); \
    padding: var(--space-6); \
    text-align: center; \
    color: var(--text-muted);\
";

const ICON_STYLE: &str = "\
    font-size: var(--text-3xl); \
    color: var(--text-muted); \
    line-height: 1;\
";

const TITLE_STYLE: &str = "\
    font-size: var(--text-lg); \
    font-weight: var(--weight-medium); \
    color: var(--text-primary); \
    margin: 0;\
";

const MESSAGE_STYLE: &str = "\
    font-size: var(--text-sm); \
    color: var(--text-muted); \
    margin: 0; \
    max-width: 40ch;\
";

/// Placeholder shown when a view has no content to render.
///
/// Use cases: a queue with zero items, a search with zero results,
/// a fresh app launch before any data has loaded. The optional
/// `action` slot accepts an `Element` (typically a button or link)
/// that the consumer wires to the corrective action ("Refresh",
/// "Connect", "Add the first item").
///
/// Per DESIGN-TOKENS.md component anatomy. See module docs for the
/// reference inventory.
///
/// # Accessibility
///
/// - **Role**: `status` — conveys a non-error state to assistive
///   technology. Empty states are informational, not alerts.
/// - **Name**: The `title` prop provides the accessible name; if
///   `message` is set, it appears as the accessible description.
/// - **Live region**: Not a live region. Empty states render
///   statically; if a view transitions between empty and populated
///   states dynamically, the consumer must mark the surrounding
///   container as a live region.
/// - **Consumer responsibility**: If `action` contains an
///   interactive element (button, link), the consumer ensures it
///   is keyboard-focusable and labelled.
#[component]
pub fn EmptyState(
    /// Headline text — the accessible name of the empty state.
    title: String,
    /// Optional secondary explanatory text (e.g. "Connect a server
    /// to see activity here").
    #[props(default)]
    message: Option<String>,
    /// Optional decorative icon, rendered above the title (Unicode
    /// glyph or short string). Marked `aria-hidden` since the
    /// title carries the semantic content.
    #[props(default)]
    icon: Option<String>,
    /// Optional interactive slot rendered below the message
    /// (typically a button or link).
    #[props(default)]
    action: Option<Element>,
) -> Element {
    rsx! {
        div {
            style: EMPTY_STATE_STYLE,
            role: "status",
            "aria-label": "{title}",
            if let Some(icon) = icon {
                div {
                    style: ICON_STYLE,
                    "aria-hidden": "true",
                    "{icon}"
                }
            }
            h2 {
                style: TITLE_STYLE,
                "{title}"
            }
            if let Some(msg) = message {
                p {
                    style: MESSAGE_STYLE,
                    "{msg}"
                }
            }
            if let Some(action) = action {
                {action}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state_renders_with_title_only() {
        let mut vdom = VirtualDom::new_with_props(
            EmptyState,
            EmptyStateProps {
                title: "Nothing here yet".to_string(),
                message: None,
                icon: None,
                action: None,
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(html.contains("Nothing here yet"), "title in output: {html}");
        assert!(
            html.contains(r#"role="status""#),
            "role attr in output: {html}"
        );
        assert!(
            html.contains(r#"aria-label="Nothing here yet""#),
            "aria-label in output: {html}"
        );
    }

    #[test]
    fn empty_state_renders_message_when_provided() {
        let mut vdom = VirtualDom::new_with_props(
            EmptyState,
            EmptyStateProps {
                title: "No sessions".to_string(),
                message: Some("Start a new session from the sidebar.".to_string()),
                icon: None,
                action: None,
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(html.contains("No sessions"), "title: {html}");
        assert!(
            html.contains("Start a new session from the sidebar."),
            "message: {html}"
        );
    }

    #[test]
    fn empty_state_renders_icon_with_aria_hidden() {
        let mut vdom = VirtualDom::new_with_props(
            EmptyState,
            EmptyStateProps {
                title: "Empty".to_string(),
                message: None,
                icon: Some("\u{1F4ED}".to_string()),
                action: None,
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        // The icon's surrounding div carries aria-hidden=true so
        // assistive tech ignores it (the title carries the meaning).
        assert!(html.contains("\u{1F4ED}"), "icon glyph in output: {html}");
        assert!(
            html.contains(r#"aria-hidden="true""#),
            "icon aria-hidden in output: {html}"
        );
    }

    #[test]
    fn empty_state_omits_message_section_when_none() {
        let mut vdom = VirtualDom::new_with_props(
            EmptyState,
            EmptyStateProps {
                title: "Nothing".to_string(),
                message: None,
                icon: None,
                action: None,
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        // No <p> tag should appear when message is None.
        assert!(!html.contains("<p"), "no <p> when message is None: {html}");
    }

    #[test]
    fn empty_state_renders_action_when_provided() {
        let mut vdom = VirtualDom::new_with_props(
            EmptyState,
            EmptyStateProps {
                title: "Disconnected".to_string(),
                message: None,
                icon: None,
                action: Some(rsx! { button { "Reconnect" } }),
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(html.contains("Reconnect"), "action button text: {html}");
        assert!(html.contains("<button"), "button tag in output: {html}");
    }
}
