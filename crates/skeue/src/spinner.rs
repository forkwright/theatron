//! Spinner — pure-CSS rotation indicator for loading states.
//!
//! Per DESIGN-TOKENS.md component anatomy:
//! - Structure: optional inline label + circular indicator
//! - Size: small (16px), medium (24px, default), large (32px)
//! - Token use: `--accent` for the active arc, `--border` for the
//!   background ring, `--text-muted` for the label
//! - Animation: 1s linear rotation, no consumer-side JS state
//!
//! References (folds in kanon discussion docket #40):
//! - Linear loading state: small circular spinner, accent stroke
//! - GitHub octicon-sync: rotating glyph
//! - Fly.io operation pending: thin-stroke ring + label
//! - Sourcehut build pending: monochrome dot triplet

use dioxus::prelude::*;

/// Visual size for a [`Spinner`]. Maps to a `--space-*` size token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SpinnerSize {
    /// Small (16px) — fits inline in a single-line input or button.
    Small,
    /// Medium (24px, default) — fits in a content area or list row.
    #[default]
    Medium,
    /// Large (32px) — fits in a dialog or empty-state placeholder.
    Large,
}

impl SpinnerSize {
    const fn px(self) -> &'static str {
        match self {
            Self::Small => "16px",
            Self::Medium => "24px",
            Self::Large => "32px",
        }
    }
}

const CONTAINER_STYLE: &str = "\
    display: inline-flex; \
    align-items: center; \
    gap: var(--space-2); \
    color: var(--text-muted); \
    font-size: var(--text-sm);\
";

const SPINNER_STYLE_FMT: &str = "\
    display: inline-block; \
    border-radius: 50%; \
    border-style: solid; \
    border-color: var(--border); \
    border-top-color: var(--accent); \
    animation: skeue-spinner-rotate 1s linear infinite;\
";

const KEYFRAMES: &str = "\
@keyframes skeue-spinner-rotate { \
    from { transform: rotate(0deg); } \
    to { transform: rotate(360deg); } \
}\
";

/// Loading indicator for asynchronous operations.
///
/// Pure CSS — no consumer-side animation state, no JavaScript hook.
/// The rotation runs as long as the component is mounted; consumers
/// unmount it (or replace with the loaded view / [`crate::EmptyState`]
/// / a populated component) when the operation completes.
///
/// Per DESIGN-TOKENS.md component anatomy. See module docs for the
/// reference inventory.
///
/// # Accessibility
///
/// - **Role**: `status` — conveys a non-error transient state to
///   assistive technology.
/// - **Live region**: `aria-live=polite` so screen readers announce
///   the loading state without interrupting the user.
/// - **Name**: The optional `label` prop provides the accessible
///   name. When `label` is absent, the spinner falls back to a
///   default `"Loading"` aria-label.
/// - **Animation**: Pure CSS rotation. Honors
///   `prefers-reduced-motion` only when the consumer's stylesheet
///   suppresses the keyframes (the component itself doesn't pause
///   on the media-query — that's a global decision the host app
///   makes via its own CSS).
#[component]
pub fn Spinner(
    /// Visual size — small / medium / large.
    #[props(default)]
    size: SpinnerSize,
    /// Optional inline label (e.g. "Loading…", "Connecting"). When
    /// absent, no visible text and `aria-label="Loading"`.
    #[props(default)]
    label: Option<String>,
) -> Element {
    let px = size.px();
    let stroke = match size {
        SpinnerSize::Small => "2px",
        SpinnerSize::Medium => "3px",
        SpinnerSize::Large => "4px",
    };
    let spinner_style =
        format!("{SPINNER_STYLE_FMT} width: {px}; height: {px}; border-width: {stroke};");

    let aria_label = label.clone().unwrap_or_else(|| "Loading".to_string());

    rsx! {
        // Inject the keyframes inline so consumers don't have to add
        // them to their global stylesheet manually. CSS rule names
        // are namespaced (`skeue-spinner-rotate`) to avoid collisions.
        style { {KEYFRAMES} }
        span {
            style: CONTAINER_STYLE,
            role: "status",
            "aria-live": "polite",
            "aria-label": aria_label,
            span {
                style: spinner_style,
                "aria-hidden": "true",
            }
            if let Some(label) = label {
                span { {label} }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_size_default_is_medium() {
        assert_eq!(SpinnerSize::default(), SpinnerSize::Medium);
    }

    #[test]
    fn spinner_size_pixel_values_are_distinct() {
        assert_ne!(SpinnerSize::Small.px(), SpinnerSize::Medium.px());
        assert_ne!(SpinnerSize::Medium.px(), SpinnerSize::Large.px());
        assert_ne!(SpinnerSize::Small.px(), SpinnerSize::Large.px());
    }

    #[test]
    fn spinner_renders_role_status_and_aria_live() {
        let mut vdom = VirtualDom::new_with_props(
            Spinner,
            SpinnerProps {
                size: SpinnerSize::Medium,
                label: None,
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(html.contains(r#"role="status""#), "role: {html}");
        assert!(html.contains(r#"aria-live="polite""#), "aria-live: {html}");
    }

    #[test]
    fn spinner_with_no_label_falls_back_to_loading_aria_label() {
        let mut vdom = VirtualDom::new_with_props(
            Spinner,
            SpinnerProps {
                size: SpinnerSize::Small,
                label: None,
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(
            html.contains(r#"aria-label="Loading""#),
            "default aria-label: {html}"
        );
    }

    #[test]
    fn spinner_renders_visible_label_when_provided() {
        let mut vdom = VirtualDom::new_with_props(
            Spinner,
            SpinnerProps {
                size: SpinnerSize::Medium,
                label: Some("Connecting…".to_string()),
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(html.contains("Connecting…"), "visible label: {html}");
        assert!(
            html.contains(r#"aria-label="Connecting…""#),
            "label propagated to aria-label: {html}"
        );
    }

    #[test]
    fn spinner_size_changes_pixel_dimension() {
        let mut small = VirtualDom::new_with_props(
            Spinner,
            SpinnerProps {
                size: SpinnerSize::Small,
                label: None,
            },
        );
        small.rebuild_in_place();
        let small_html = dioxus_ssr::render(&small);

        let mut large = VirtualDom::new_with_props(
            Spinner,
            SpinnerProps {
                size: SpinnerSize::Large,
                label: None,
            },
        );
        large.rebuild_in_place();
        let large_html = dioxus_ssr::render(&large);

        assert!(small_html.contains("width: 16px"), "small: {small_html}");
        assert!(large_html.contains("width: 32px"), "large: {large_html}");
    }

    #[test]
    fn spinner_includes_keyframes_in_output() {
        let mut vdom = VirtualDom::new_with_props(
            Spinner,
            SpinnerProps {
                size: SpinnerSize::Medium,
                label: None,
            },
        );
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);
        assert!(
            html.contains("@keyframes skeue-spinner-rotate"),
            "keyframes inlined: {html}"
        );
    }
}
