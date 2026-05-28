//! Toast notification component (extracted from aletheia/proskenion).
//!
//! API redesign for skeue:
//! - `Toast` and `ToastId` types are now defined here (canonical)
//! - `ToastDispatcher` trait replaces aletheia's `use_toast` hook —
//!   consumers provide their own state container implementing it
//! - Action dispatch is now a generic `EventHandler<ToastAction>`
//!   callback — no more `crate::state::navigation::NavAction` dep

use std::time::Duration;

use dioxus::prelude::*;

/// Severity / visual register for a toast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToastSeverity {
    /// Informational notice — neutral tone.
    Info,
    /// Successful operation — confirmation tone.
    Success,
    /// Caution worth surfacing but not a failure.
    Warning,
    /// Functional failure that the user should know about.
    Error,
}

impl ToastSeverity {
    /// CSS color token name for this severity (foreground).
    #[must_use]
    pub fn css_color(&self) -> &'static str {
        match self {
            Self::Info => "var(--status-info)",
            Self::Success => "var(--status-success)",
            Self::Warning => "var(--status-warning)",
            // WHY: status-error (functional failure) not aima (vital/blood
            // dye). Per DESIGN-TOKENS.md: aima is for things that demand
            // immediate response (vital state); status-error is for "this
            // thing failed" (functional failure). Toasts report failures.
            Self::Error => "var(--status-error)",
        }
    }

    /// CSS color token name for this severity (background tint).
    #[must_use]
    pub fn css_bg(&self) -> &'static str {
        match self {
            Self::Info => "var(--status-info-bg)",
            Self::Success => "var(--status-success-bg)",
            Self::Warning => "var(--status-warning-bg)",
            Self::Error => "var(--status-error-bg)",
        }
    }
}

/// Opaque toast identifier. Wrapper newtype so consumers can't conflate
/// with arbitrary u64 ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToastId(pub u64);

/// Opaque caller-defined action identifier. Wrapper newtype so consumers
/// can't conflate with arbitrary strings; theatron does not interpret it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToastActionId(pub String);

/// An action attached to a toast (e.g. "Open file", "Undo").
#[derive(Clone, Debug, PartialEq)]
pub struct ToastAction {
    /// Display label rendered on the action button.
    pub label: String,
    /// Caller-defined action identifier. theatron does not interpret it.
    pub action_id: ToastActionId,
}

/// A toast notification.
#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    /// Unique identifier for this toast instance.
    pub id: ToastId,
    /// Visual register / severity classification.
    pub severity: ToastSeverity,
    /// Headline shown in the toast.
    pub title: String,
    /// Optional secondary text beneath the title.
    pub body: Option<String>,
    /// Optional clickable action to attach.
    pub action: Option<ToastAction>,
    /// If set, the toast auto-dismisses after this duration.
    pub auto_dismiss: Option<Duration>,
}

const TOAST_STYLE: &str = "\
    display: flex; \
    flex-direction: column; \
    gap: var(--space-1); \
    padding: var(--space-3) var(--space-4); \
    border-radius: var(--radius-lg); \
    border-left: 4px solid; \
    min-width: 300px; \
    max-width: 400px; \
    box-shadow: var(--shadow-float, 0 4px 16px rgb(18 17 15 / 0.16)); \
    animation: toast-enter 350ms cubic-bezier(0.16, 1, 0.3, 1); \
    position: relative;\
";

const TITLE_STYLE: &str = "\
    font-size: var(--text-base); \
    font-weight: var(--weight-semibold);\
";

const BODY_STYLE: &str = "\
    font-size: var(--text-sm); \
    opacity: 0.85;\
";

const DISMISS_STYLE: &str = "\
    position: absolute; \
    top: var(--space-2); \
    right: var(--space-2); \
    background: none; \
    border: none; \
    color: inherit; \
    opacity: 0.6; \
    cursor: pointer; \
    transition: background-color var(--transition-quick), color var(--transition-quick); \
    font-size: var(--text-base); \
    padding: var(--space-1) var(--space-2); \
    min-width: 24px; \
    min-height: 24px; \
    display: flex; \
    align-items: center; \
    justify-content: center;\
";

const ACTION_STYLE: &str = "\
    background: var(--bg-surface-bright); \
    border: 1px solid var(--border); \
    border-radius: var(--radius-sm); \
    color: inherit; \
    cursor: pointer; \
    transition: background-color var(--transition-quick); \
    font-size: var(--text-sm); \
    padding: var(--space-1) var(--space-3); \
    align-self: flex-start; \
    margin-top: var(--space-1);\
";

/// Render a single toast notification.
///
/// `on_dismiss` and `on_action` are caller-provided event handlers. The
/// generic API replaces aletheia's `use_toast` hook + navigation parser.
///
/// # Accessibility
///
/// - **Role**: `status` (polite live region); `alert` for
///   `severity="error"` (assertive live region).
/// - **Name**: The toast `title` provides the accessible name.
/// - **Live region**: `aria-live` is set to `polite` for info/success/warning
///   and `assertive` for error severity.
/// - **Keyboard navigation**: The dismiss button is focusable and has an
///   `aria-label`.
/// - **Consumer responsibility**: None.
#[component]
pub fn ToastItem(
    toast: Toast,
    on_dismiss: EventHandler<ToastId>,
    on_action: EventHandler<ToastAction>,
) -> Element {
    let toast_id = toast.id;
    let color = toast.severity.css_color();
    let bg = toast.severity.css_bg();

    // WHY: Auto-dismiss timer. use_future ties the future's lifetime to
    // this component instance — when the toast unmounts (manual dismiss,
    // route change, container hide), the future is cancelled so the
    // callback never fires on a detached component. spawn() would leak
    // (the task outlives the component). tokio::time::sleep yields the
    // executor; std::thread::sleep would block the worker thread (this
    // was an earlier W1-spike regression caught by QA wave 1 #04 + #11).
    let auto_dismiss = toast.auto_dismiss;
    use_future(move || async move {
        if let Some(duration) = auto_dismiss {
            tokio::time::sleep(duration).await;
            on_dismiss.call(toast_id);
        }
    });

    let (role, aria_live) = match toast.severity {
        ToastSeverity::Error => ("alert", "assertive"),
        _ => ("status", "polite"),
    };
    rsx! {
        div {
            role: "{role}",
            aria_live: "{aria_live}",
            aria_atomic: "true",
            style: "{TOAST_STYLE} background: {bg}; border-color: {color}; color: var(--text-primary);",
            button {
                style: "{DISMISS_STYLE}",
                aria_label: "Dismiss notification",
                onclick: move |_| on_dismiss.call(toast_id),
                "\u{2715}"
            }
            div { style: "{TITLE_STYLE}", "{toast.title}" }
            if let Some(ref body) = toast.body {
                div { style: "{BODY_STYLE}", "{body}" }
            }
            if let Some(ref action) = toast.action {
                {
                    let action_clone = action.clone();
                    rsx! {
                        button {
                            style: "{ACTION_STYLE}",
                            onclick: move |_| {
                                on_action.call(action_clone.clone());
                                on_dismiss.call(toast_id);
                            },
                            "{action.label}"
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod ssr_tests {
    use dioxus_core::VirtualDom;
    use dioxus_ssr::render;

    use super::*;

    #[test]
    fn info_toast_renders_status_polite() {
        #[component]
        fn Wrapper() -> Element {
            let toast = Toast {
                id: ToastId(1),
                severity: ToastSeverity::Info,
                title: "Saved".to_string(),
                body: None,
                action: None,
                auto_dismiss: None,
            };
            rsx! {
                ToastItem {
                    toast,
                    on_dismiss: |_| {},
                    on_action: |_| {},
                }
            }
        }
        let mut dom = VirtualDom::new(Wrapper);
        dom.rebuild_in_place();
        let html = render(&dom);
        assert!(
            html.contains("role=\"status\""),
            "expected role=status in {html}"
        );
        assert!(
            html.contains("aria-live=\"polite\""),
            "expected aria-live=polite in {html}"
        );
        assert!(
            html.contains("aria-atomic=\"true\""),
            "expected aria-atomic=true in {html}"
        );
        assert!(
            html.contains("aria-label=\"Dismiss notification\""),
            "expected dismiss aria-label in {html}"
        );
    }

    #[test]
    fn error_toast_renders_alert_assertive() {
        #[component]
        fn Wrapper() -> Element {
            let toast = Toast {
                id: ToastId(2),
                severity: ToastSeverity::Error,
                title: "Failed".to_string(),
                body: None,
                action: None,
                auto_dismiss: None,
            };
            rsx! {
                ToastItem {
                    toast,
                    on_dismiss: |_| {},
                    on_action: |_| {},
                }
            }
        }
        let mut dom = VirtualDom::new(Wrapper);
        dom.rebuild_in_place();
        let html = render(&dom);
        assert!(
            html.contains("role=\"alert\""),
            "expected role=alert in {html}"
        );
        assert!(
            html.contains("aria-live=\"assertive\""),
            "expected aria-live=assertive in {html}"
        );
    }
}
