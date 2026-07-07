//! Queue table — sortable list of pending work items (PRs, CI jobs, issues).
//!
//! Per DESIGN-TOKENS.md component anatomy:
//! - Structure: header row + activity rows + optional pagination
//! - Token use: header `--text-secondary` / `--text-xs` /
//!   `--weight-semibold` / `--border-separator`
//! - Row: see [`ActivityRow`]
//!
//! References (folds in #40):
//! - Sourcehut PR queue: header + monospace rows + cursor pagination
//! - Linear inbox: header + activity feed + count badge
//! - Radicle distributed PR list: minimal header, full-width rows

use dioxus::prelude::*;

use crate::activity_row::{ActivityRow, ActivityStatus, RowDensity, RowSemantics};

/// One column header definition for [`QueueTable`].
///
/// Header labels are laid out with flexbox (`flex: 1 1 auto`), sharing
/// the row width equally; there is no per-column width control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueColumn {
    /// Column label. Empty string renders no text (icon-only column).
    pub label: String,
}

/// One row in a [`QueueTable`]. Mirrors [`ActivityRow`]'s props but with
/// owned data so callers can build a `Vec<QueueItem>` and pass it in.
// kanon:ignore TOPOLOGY/shallow-struct -- Dioxus props data-carrier; the fields are the API and there is no invariant a constructor would enforce
#[derive(Debug, Clone, PartialEq)]
pub struct QueueItem {
    /// Primary text.
    pub title: String,
    /// Timestamp string (consumer formats; component renders verbatim).
    pub timestamp: String,
    /// Optional leading icon glyph or short string.
    pub icon: Option<String>,
    /// Optional metadata between title and timestamp.
    pub metadata: Option<String>,
    /// Optional inline status pill.
    pub status: Option<ActivityStatus>,
}

const HEADER_STYLE: &str = "\
    display: flex; \
    align-items: center; \
    gap: var(--space-3); \
    padding: var(--space-1) var(--space-3); \
    border-bottom: 1px solid var(--border-separator); \
    color: var(--text-secondary); \
    font-size: var(--text-xs); \
    font-weight: var(--weight-semibold); \
    text-transform: uppercase; \
    letter-spacing: 0.04em;\
";

const HEADER_LABEL_STYLE: &str = "\
    flex: 1 1 auto; \
    min-width: 0; \
    overflow: hidden; \
    text-overflow: ellipsis; \
    white-space: nowrap;\
";

const TABLE_STYLE: &str = "\
    display: flex; \
    flex-direction: column; \
    background: var(--bg-surface); \
    border: 1px solid var(--border); \
    border-radius: var(--radius-md); \
    overflow: hidden;\
";

const EMPTY_STYLE: &str = "\
    display: flex; \
    align-items: center; \
    justify-content: center; \
    padding: var(--space-4); \
    color: var(--text-muted); \
    font-size: var(--text-sm);\
";

/// A sortable list of pending work items.
///
/// Composes [`ActivityRow`] for each item plus a single header row.
/// Sorting is consumer-driven — pass items already in display order.
///
/// # Accessibility
///
/// - **Role**: `table` — the header row carries `role="row"` with
///   `role="columnheader"`/`scope="col"` children; data rows render via
///   [`RowSemantics::TableRow`] as `role="row"` with `role="cell"` children.
/// - **Name**: Column header text provides the column names.
/// - **Consumer responsibility**: If rows are interactive (click-to-detail),
///   the consumer must add focus handling (`tabindex="0"`) per row.
#[component]
pub fn QueueTable(
    /// Column headers.
    columns: Vec<QueueColumn>,
    /// Items to render. Each becomes one [`ActivityRow`].
    items: Vec<QueueItem>,
    /// Row density — applied uniformly to every row.
    #[props(default)]
    density: RowDensity,
    /// Optional message shown when `items` is empty.
    #[props(default)]
    empty_label: Option<String>,
) -> Element {
    let empty_msg = empty_label.unwrap_or_else(|| "No items".to_string());
    rsx! {
        div {
            role: "table",
            style: TABLE_STYLE,
            div {
                role: "row",
                style: HEADER_STYLE,
                for (i , col) in columns.iter().enumerate() {
                    span {
                        key: "{i}",
                        role: "columnheader",
                        "scope": "col",
                        style: HEADER_LABEL_STYLE,
                        "{col.label}"
                    }
                }
            }
            if items.is_empty() {
                div { style: EMPTY_STYLE, {empty_msg} }
            } else {
                for (i , item) in items.into_iter().enumerate() {
                    ActivityRow {
                        key: "{i}",
                        title: item.title,
                        timestamp: item.timestamp,
                        icon: item.icon,
                        metadata: item.metadata,
                        status: item.status,
                        density,
                        semantics: RowSemantics::TableRow,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status_pill::StatusPillKind;

    #[test]
    fn queue_column_carries_label() {
        let c = QueueColumn {
            label: "Title".to_string(),
        };
        assert_eq!(c.label, "Title");
    }

    #[test]
    fn renders_role_table_and_scope_col() {
        use dioxus::prelude::*;
        use dioxus_ssr::render_element;
        let html = render_element(rsx! {
            QueueTable {
                columns: vec![QueueColumn { label: "Title".to_string() }],
                items: vec![QueueItem {
                    title: "PR #1".to_string(),
                    timestamp: "2m ago".to_string(),
                    icon: None,
                    metadata: None,
                    status: None,
                }],
            }
        });
        assert!(
            html.contains("role=\"table\""),
            "expected role=table in {html}"
        );
        assert!(
            html.contains("scope=\"col\""),
            "expected scope=col in {html}"
        );
        assert!(html.contains("Title"), "expected header text in {html}");
    }

    #[test]
    fn data_rows_render_role_row_with_cells_not_listitem() {
        use dioxus::prelude::*;
        use dioxus_ssr::render_element;
        let html = render_element(rsx! {
            QueueTable {
                columns: vec![QueueColumn { label: "Title".to_string() }],
                items: vec![QueueItem {
                    title: "PR #1".to_string(),
                    timestamp: "2m ago".to_string(),
                    icon: None,
                    metadata: None,
                    status: None,
                }],
            }
        });
        // Header row + one data row.
        assert_eq!(
            html.matches("role=\"row\"").count(),
            2,
            "expected header row and data row in {html}"
        );
        assert!(
            html.contains("role=\"cell\""),
            "expected role=cell on data-row cells in {html}"
        );
        assert!(
            !html.contains("role=\"listitem\""),
            "listitem is invalid inside role=table: {html}"
        );
    }

    #[test]
    fn renders_empty_label() {
        use dioxus::prelude::*;
        use dioxus_ssr::render_element;
        let html = render_element(rsx! {
            QueueTable {
                columns: vec![QueueColumn { label: "Title".to_string() }],
                items: vec![],
            }
        });
        assert!(html.contains("No items"), "expected empty label in {html}");
    }

    #[test]
    fn queue_item_carries_all_fields() {
        let item = QueueItem {
            title: "PR #100".to_string(),
            timestamp: "2m ago".to_string(),
            icon: Some("\u{2605}".to_string()),
            metadata: Some("forkwright/theatron".to_string()),
            status: Some(ActivityStatus {
                kind: StatusPillKind::Success,
                label: "merged".to_string(),
            }),
        };
        assert_eq!(item.title, "PR #100");
        assert!(item.icon.is_some());
        assert!(item.status.is_some());
    }
}
