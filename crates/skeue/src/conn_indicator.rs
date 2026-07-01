//! Connection indicator — a colored dot + label showing live-stream
//! health.
//!
//! Per DESIGN-TOKENS.md component anatomy ("conn status"):
//! - Structure: one colored dot + one short label
//! - Token use: `--status-success` / `--status-warning` / `--status-error`
//! - Optional tooltip describing the state
//!
//! References (folds in #40):
//! - GitHub repo "deployment status" indicator
//! - Linear sync indicator
//! - Vercel deployment dot

use dioxus::prelude::*;

/// Semantic health register for the indicator.
///
/// Names describe the *meaning* of the state, not the rendered color —
/// per the gnomon naming principle (`--accent` not `--brass-gold`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IndicatorTone {
    /// Connected, receiving events normally.
    Healthy,
    /// Reconnecting, intermittent, or otherwise degraded.
    Degraded,
    /// Disconnected, errored, or unable to recover.
    Failed,
}

impl IndicatorTone {
    /// Status color token for this tone.
    #[must_use]
    pub const fn color_token(self) -> &'static str {
        match self {
            Self::Healthy => "var(--status-success)",
            Self::Degraded => "var(--status-warning)",
            Self::Failed => "var(--status-error)",
        }
    }
}

const INDICATOR_STYLE: &str = "\
    display: flex; \
    align-items: center; \
    gap: var(--space-1); \
    padding: var(--space-1) var(--space-2); \
    font-size: var(--text-xs); \
    opacity: 0.85;\
";

/// A colored dot + short label indicating connection / live-stream
/// health. Pairs with [`IndicatorTone`] for semantic color.
///
/// Generic over the source of the state — consumers map their own
/// connection-state types to a `(tone, label, tooltip)` tuple and pass
/// the props.
///
/// # Accessibility
///
/// - **Role**: `status` — conveys connection health.
/// - **Name**: The `label` prop provides the accessible name.
/// - **Consumer responsibility**: Provide a descriptive `label` that
///   conveys the current state (e.g. "Connected" or "Reconnecting (3)").
#[component]
pub fn ConnectionIndicator(
    /// Semantic health register.
    tone: IndicatorTone,
    /// Short label rendered next to the dot, e.g. "Connected" or
    /// "Reconnecting (3)".
    label: String,
    /// Optional tooltip with extended description.
    #[props(default)]
    tooltip: Option<String>,
) -> Element {
    let color = tone.color_token();
    // kanon:ignore RUST/no-result-unwrap-or-default -- `tooltip` is Option<String>; None -> empty title attribute is intentional.
    let title = tooltip.unwrap_or_default();
    rsx! {
        div {
            role: "status",
            style: INDICATOR_STYLE,
            title: title,
            span {
                aria_hidden: "true",
                style: "color: {color}; font-size: var(--text-xs);",
                "\u{25CF}"
            }
            span { {label} }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tone_color_tokens_use_status_namespace() {
        assert_eq!(
            IndicatorTone::Healthy.color_token(),
            "var(--status-success)"
        );
        assert_eq!(
            IndicatorTone::Degraded.color_token(),
            "var(--status-warning)"
        );
        assert_eq!(IndicatorTone::Failed.color_token(), "var(--status-error)");
    }

    #[test]
    fn renders_role_status() {
        use dioxus::prelude::*;
        use dioxus_ssr::render_element;
        let html = render_element(rsx! {
            ConnectionIndicator {
                tone: IndicatorTone::Healthy,
                label: "Connected".to_string(),
            }
        });
        assert!(
            html.contains("role=\"status\""),
            "expected role=status in {html}"
        );
        assert!(html.contains("Connected"), "expected label text in {html}");
    }

    #[test]
    fn renders_aria_hidden_on_dot() {
        use dioxus::prelude::*;
        use dioxus_ssr::render_element;
        let html = render_element(rsx! {
            ConnectionIndicator {
                tone: IndicatorTone::Healthy,
                label: "Connected".to_string(),
            }
        });
        assert!(
            html.contains("aria-hidden=\"true\""),
            "expected aria-hidden on dot in {html}"
        );
    }

    #[test]
    fn tone_is_copy_and_eq() {
        let a = IndicatorTone::Healthy;
        let b = a;
        assert_eq!(a, b);
    }
}
