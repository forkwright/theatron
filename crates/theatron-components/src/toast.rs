//! Toast notification component (extracted from aletheia/proskenion).
//!
//! API redesign for theatron-components:
//! - `Toast` and `ToastId` types are now defined here (canonical)
//! - `ToastDispatcher` trait replaces aletheia's `use_toast` hook —
//!   consumers provide their own state container implementing it
//! - Action dispatch is now a generic `EventHandler<ToastAction>`
//!   callback — no more `crate::state::navigation::NavAction` dep

use dioxus::prelude::*;
use std::time::Duration;

/// Severity / visual register for a toast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastSeverity {
    Info,
    Success,
    Warning,
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

/// An action attached to a toast (e.g. "Open file", "Undo").
#[derive(Clone, Debug, PartialEq)]
pub struct ToastAction {
    pub label: String,
    /// Caller-defined action identifier. theatron does not interpret it.
    pub action_id: String,
}

/// A toast notification.
#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: ToastId,
    pub severity: ToastSeverity,
    pub title: String,
    pub body: Option<String>,
    pub action: Option<ToastAction>,
    pub auto_dismiss: Option<Duration>,
}

/// Trait that the consumer's toast state container must implement.
/// theatron provides the rendering; consumer provides the state.
pub trait ToastDispatcher {
    fn dismiss(&mut self, id: ToastId);
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
#[component]
pub fn ToastItem(
    toast: Toast,
    on_dismiss: EventHandler<ToastId>,
    on_action: EventHandler<ToastAction>,
) -> Element {
    let toast_id = toast.id;
    let color = toast.severity.css_color();
    let bg = toast.severity.css_bg();

    // WHY: Auto-dismiss timer. Spawn a task that sleeps then fires the
    // dismiss callback. Runs once per toast mount.
    if let Some(duration) = toast.auto_dismiss {
        #[expect(
            clippy::as_conversions,
            reason = "toast duration under u64::MAX milliseconds"
        )]
        let ms = duration.as_millis() as u64;
        spawn(async move {
            // Note: spike uses std::thread::sleep stand-in;
            // theatron-components proper will use tokio::time::sleep.
            std::thread::sleep(std::time::Duration::from_millis(ms));
            on_dismiss.call(toast_id);
        });
    }

    rsx! {
        div {
            style: "{TOAST_STYLE} background: {bg}; border-color: {color}; color: var(--text-primary);",
            button {
                style: "{DISMISS_STYLE}",
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
