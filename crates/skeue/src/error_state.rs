//! Error state — placeholder shown when a view's data fetch failed.
//!
//! Per DESIGN-TOKENS.md component anatomy:
//! - Structure: optional icon + title + optional message + optional action
//! - Size: fills the parent container (centered)
//! - Token use: `--status-error` for the icon and title accent,
//!   `--text-primary` for the title text, `--text-muted` for the
//!   message, `--bg-surface` panel background
//! - Padding: `--space-6` block, centered alignment
//! - Text: title `--text-lg --weight-medium`, message `--text-sm`
//!
//! Sibling component to [`EmptyState`](crate::EmptyState) (no-data
//! state) and [`Spinner`](crate::Spinner) (loading state). Together
//! the three cover the asynchronous-view triad.
//!
//! References (folds in kanon discussion docket #40):
//! - GitHub error pages: octicon + title + secondary text + retry
//!   link
//! - Linear error toasts: red border + label + retry CTA
//! - Sourcehut build failure pages: minimal — title + log link
//! - Fly.io operation failure: glyph + headline + diagnostic body

use dioxus::prelude::*;

const ERROR_STATE_STYLE: &str = "\
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
    color: var(--status-error); \
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
    max-width: 60ch; \
    word-break: break-word;\
";

/// Placeholder shown when a view's data fetch failed.
///
/// Use cases: an SSE stream that errored, a CI run fetch that
/// returned 5xx, a search that timed out. The optional `action`
/// slot accepts an `Element` (typically a "Retry" button) that
/// the consumer wires to the corrective action.
///
/// Per DESIGN-TOKENS.md component anatomy. See module docs for the
/// reference inventory.
///
/// # Accessibility
///
/// - **Role**: `alert` — conveys an error state to assistive
///   technology with appropriate priority. Distinct from
///   [`EmptyState`](crate::EmptyState)'s `status` role
///   (informational, not error).
/// - **Live region**: `aria-live=assertive` so screen readers
///   announce the error promptly. Errors interrupt the user's
///   flow; `polite` would be too quiet.
/// - **Name**: The `title` prop provides the accessible name; if
///   `message` is set, it appears as the accessible description.
/// - **Consumer responsibility**: If `action` contains an
///   interactive element (button, link), the consumer ensures it
///   is keyboard-focusable and labelled. Operators should be
///   able to recover from the error via keyboard alone.
#[component]
pub fn ErrorState(
    /// Headline text — the accessible name of the error state
    /// (e.g. "Could not load runs", "Connection lost").
    title: String,
    /// Optional secondary explanatory text (e.g. the underlying
    /// error message, or "Check your network and try again").
    #[props(default)]
    message: Option<String>,
    /// Optional decorative icon, rendered above the title (Unicode
    /// glyph or short string). Marked `aria-hidden` since the
    /// title carries the semantic content. Defaults to a stop-sign
    /// glyph when None.
    #[props(default)]
    icon: Option<String>,
    /// Optional interactive slot rendered below the message
    /// (typically a "Retry" button).
    #[props(default)]
    action: Option<Element>,
) -> Element {
    let icon_text = icon.unwrap_or_else(|| "\u{26A0}".to_string()); // ⚠ warning sign
    // WHY: aria-describedby must reference a DOM id unique to this instance --
    // ScopeId is stable across re-renders and unique per mounted component, so
    // two ErrorState instances on one page never collide over the same id.
    let message_id = format!("error-state-message-{}", dioxus::core::current_scope_id().0);
    let describedby = message.is_some().then(|| message_id.clone());
    rsx! {
        div {
            style: ERROR_STATE_STYLE,
            role: "alert",
            "aria-live": "assertive",
            "aria-label": title.clone(),
            "aria-describedby": describedby,
            div {
                style: ICON_STYLE,
                "aria-hidden": "true",
                {icon_text}
            }
            h2 {
                style: TITLE_STYLE,
                {title.clone()}
            }
            if let Some(msg) = message {
                p {
                    id: message_id.clone(),
                    style: MESSAGE_STYLE,
                    {msg}
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
    fn error_state_renders_with_title_only() {
        let mut vdom = VirtualDom::new_with_props(
            ErrorState,
            ErrorStateProps {
                title: "Could not load runs".to_string(),
                message: None,
                icon: None,
                action: None,
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(html.contains("Could not load runs"), "title: {html}");
        assert!(
            html.contains(r#"role="alert""#),
            "alert role in output: {html}"
        );
        assert!(
            html.contains(r#"aria-live="assertive""#),
            "assertive live in output: {html}"
        );
    }

    #[test]
    fn error_state_falls_back_to_warning_icon_when_none() {
        let mut vdom = VirtualDom::new_with_props(
            ErrorState,
            ErrorStateProps {
                title: "Failed".to_string(),
                message: None,
                icon: None,
                action: None,
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(
            html.contains("\u{26A0}"),
            "default warning glyph in output: {html}"
        );
    }

    #[test]
    fn error_state_uses_provided_icon() {
        let mut vdom = VirtualDom::new_with_props(
            ErrorState,
            ErrorStateProps {
                title: "Network error".to_string(),
                message: None,
                icon: Some("\u{1F4F6}".to_string()),
                action: None,
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(html.contains("\u{1F4F6}"), "custom icon: {html}");
        assert!(
            !html.contains("\u{26A0}"),
            "custom icon replaces default: {html}"
        );
    }

    #[test]
    fn error_state_renders_message_when_provided() {
        let mut vdom = VirtualDom::new_with_props(
            ErrorState,
            ErrorStateProps {
                title: "Connection lost".to_string(),
                message: Some("Check your network and try again.".to_string()),
                icon: None,
                action: None,
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(
            html.contains("Check your network and try again."),
            "message: {html}"
        );
    }

    #[test]
    fn error_state_omits_message_section_when_none() {
        let mut vdom = VirtualDom::new_with_props(
            ErrorState,
            ErrorStateProps {
                title: "Failed".to_string(),
                message: None,
                icon: None,
                action: None,
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(!html.contains("<p"), "no <p> when message is None: {html}");
    }

    /// Regression test for issue #184.2: the doc claimed `message` becomes
    /// the accessible description, but no `aria-describedby` wiring existed.
    #[test]
    fn error_state_wires_aria_describedby_to_message_id() {
        let mut vdom = VirtualDom::new_with_props(
            ErrorState,
            ErrorStateProps {
                title: "Connection lost".to_string(),
                message: Some("Check your network and try again.".to_string()),
                icon: None,
                action: None,
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        let id_marker = "id=\"";
        let id_start = html
            .find(id_marker)
            .map(|i| i + id_marker.len())
            .expect("message <p> should carry an id");
        let id_end = html[id_start..]
            .find('"')
            .map(|i| id_start + i)
            .expect("id attribute value should be closed");
        let message_id = &html[id_start..id_end];
        assert!(
            html.contains(&format!("aria-describedby=\"{message_id}\"")),
            "expected aria-describedby to reference message id {message_id} in {html}"
        );
    }

    #[test]
    fn error_state_omits_aria_describedby_when_no_message() {
        let mut vdom = VirtualDom::new_with_props(
            ErrorState,
            ErrorStateProps {
                title: "Failed".to_string(),
                message: None,
                icon: None,
                action: None,
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(
            !html.contains("aria-describedby"),
            "no aria-describedby without a message: {html}"
        );
    }

    #[test]
    fn error_state_renders_action_when_provided() {
        let mut vdom = VirtualDom::new_with_props(
            ErrorState,
            ErrorStateProps {
                title: "Disconnected".to_string(),
                message: None,
                icon: None,
                action: Some(rsx! { button { "Retry" } }),
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(html.contains("Retry"), "action button text: {html}");
        assert!(html.contains("<button"), "button tag: {html}");
    }
}
