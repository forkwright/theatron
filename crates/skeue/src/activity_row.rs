//! Activity row — a single event in a feed or queue table.
//!
//! Per DESIGN-TOKENS.md component anatomy:
//! - Structure: icon/avatar + title + timestamp + optional status pill +
//!   optional metadata line
//! - Height: `--row-h-standard` (36px) for compact tables,
//!   `--row-h-roomy` (48px) for feeds
//! - Token use: title `--text-primary`, metadata `--text-secondary`,
//!   timestamp `--text-muted`
//! - Hover: background `--bg-hover`
//!
//! References (folds in #40):
//! - GitHub notifications: icon + title + timestamp + status badge
//! - Linear activity feed: avatar + actor + action + timestamp
//! - Sourcehut log: monospace icon + summary + relative time

use dioxus::prelude::*;

use crate::status_pill::{StatusPill, StatusPillKind, StatusPillShape};

/// Density variant for the row height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum RowDensity {
    /// 36px — for tables. Default.
    #[default]
    Standard,
    /// 48px — for feeds, more breathing room.
    Roomy,
}

impl RowDensity {
    const fn height_token(self) -> &'static str {
        match self {
            Self::Standard => "var(--row-h-standard, 36px)",
            Self::Roomy => "var(--row-h-roomy, 48px)",
        }
    }
}

/// Optional status pill annotation alongside the row title.
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityStatus {
    /// Pill kind.
    pub kind: StatusPillKind,
    /// Pill label.
    pub label: String,
}

impl ActivityStatus {
    /// Create an activity-row status annotation.
    #[must_use]
    pub fn new(kind: StatusPillKind, label: impl Into<String>) -> Self {
        Self {
            kind,
            label: label.into(),
        }
    }
}

const ROW_STYLE_FMT: &str = "\
    display: flex; \
    align-items: center; \
    gap: var(--space-3); \
    padding: var(--space-1) var(--space-3); \
    border-bottom: 1px solid var(--border-separator); \
    transition: background-color var(--transition-quick);\
";

const ICON_STYLE: &str = "\
    flex: 0 0 auto; \
    color: var(--text-secondary); \
    font-size: var(--text-sm);\
";

const TITLE_STYLE: &str = "\
    flex: 1 1 auto; \
    min-width: 0; \
    color: var(--text-primary); \
    font-size: var(--text-sm); \
    font-weight: var(--weight-medium); \
    overflow: hidden; \
    text-overflow: ellipsis; \
    white-space: nowrap;\
";

const META_STYLE: &str = "\
    flex: 0 0 auto; \
    color: var(--text-secondary); \
    font-size: var(--text-xs);\
";

const TIMESTAMP_STYLE: &str = "\
    flex: 0 0 auto; \
    color: var(--text-muted); \
    font-size: var(--text-xs); \
    font-variant-numeric: tabular-nums;\
";

/// A single activity-feed / queue-table row.
///
/// Per DESIGN-TOKENS.md component anatomy. See module docs for the
/// reference inventory.
///
/// # Accessibility
///
/// - **Role**: `listitem` — intended for use inside a list.
/// - **Name**: The `title` text provides the primary accessible name;
///   the optional status pill contributes additional state.
/// - **Consumer responsibility**: Wrap rows in a parent with `role="list"`
///   when rendering an activity feed.
#[component]
pub fn ActivityRow(
    /// Primary text — actor + action, or just title.
    title: String,
    /// Timestamp display string (consumer formats; component renders verbatim).
    timestamp: String,
    /// Optional leading icon glyph or short string.
    #[props(default)]
    icon: Option<String>,
    /// Optional secondary metadata between title and timestamp.
    #[props(default)]
    metadata: Option<String>,
    /// Optional status pill rendered to the right of the title.
    #[props(default)]
    status: Option<ActivityStatus>,
    /// Standard or roomy density. Defaults to standard.
    #[props(default)]
    density: RowDensity,
) -> Element {
    let height = density.height_token();
    rsx! {
        div {
            role: "listitem",
            style: "{ROW_STYLE_FMT} min-height: {height};",
            if let Some(ref glyph) = icon {
                span {
                    style: "{ICON_STYLE}",
                    aria_hidden: "true",
                    "{glyph}"
                }
            }
            div {
                style: "{TITLE_STYLE}",
                title: "{title}",
                "{title}"
            }
            if let Some(ref s) = status {
                StatusPill {
                    kind: s.kind,
                    label: s.label.clone(),
                    shape: StatusPillShape::Pill,
                }
            }
            if let Some(ref meta) = metadata {
                span { style: "{META_STYLE}", "{meta}" }
            }
            span { style: "{TIMESTAMP_STYLE}", "{timestamp}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_height_tokens_use_row_h_namespace() {
        assert_eq!(
            RowDensity::Standard.height_token(),
            "var(--row-h-standard, 36px)"
        );
        assert_eq!(RowDensity::Roomy.height_token(), "var(--row-h-roomy, 48px)");
    }

    #[test]
    fn density_default_is_standard() {
        assert_eq!(RowDensity::default(), RowDensity::Standard);
    }

    #[test]
    fn renders_role_listitem() {
        use dioxus::prelude::*;
        use dioxus_ssr::render_element;
        let html = render_element(rsx! {
            ActivityRow {
                title: "Event".to_string(),
                timestamp: "2m ago".to_string(),
            }
        });
        assert!(
            html.contains("role=\"listitem\""),
            "expected role=listitem in {html}"
        );
        assert!(html.contains("Event"), "expected title text in {html}");
    }

    #[test]
    fn renders_aria_hidden_on_icon() {
        use dioxus::prelude::*;
        use dioxus_ssr::render_element;
        let html = render_element(rsx! {
            ActivityRow {
                title: "Event".to_string(),
                timestamp: "2m ago".to_string(),
                icon: Some("★".to_string()),
            }
        });
        assert!(
            html.contains("aria-hidden=\"true\""),
            "expected aria-hidden on icon in {html}"
        );
    }

    #[test]
    fn activity_status_carries_kind_and_label() {
        let s = ActivityStatus::new(StatusPillKind::Success, "merged");
        assert_eq!(s.kind, StatusPillKind::Success);
        assert_eq!(s.label, "merged");
    }
}
