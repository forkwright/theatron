#![deny(missing_docs)]

//! σκευή (skeue, props/equipment) — generic Dioxus components per kanon
//! DESIGN-TOKENS.md component anatomy.
//!
//! Each component implementation includes `// References:` blocks citing
//! external sources where their anatomy was sourced (Linear, Sourcehut,
//! Fly.io, Grafana, Radicle) — folds in kanon discussion docket #40.
//!
//! Phase 1+2 deliverable. See `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md`
//! for the broader plan.
//!
//! ## Components seeded from extraction spike (W1)
//!
//! - [`virtual_list`] — virtual scrolling primitives + helpers
//! - [`table`] — markdown table renderer with theatron-local
//!   [`TableAlignment`](table::TableAlignment) (decoupled from
//!   `pulldown_cmark::Alignment` so consumers don't have to share a
//!   pulldown-cmark major version with us — `From` impl provided)
//! - [`toast`] — toast notification with `EventHandler<ToastId>` /
//!   `EventHandler<ToastAction>` callbacks replacing aletheia's
//!   `use_toast` hook + `NavAction` parser

pub mod activity_row;
pub mod code_block;
pub mod conn_indicator;
pub mod diff_hunk;
pub mod diff_line;
pub mod metric_tile;
pub mod queue_table;
pub mod sparkline;
pub mod status_pill;
pub mod table;
pub mod toast;
pub mod virtual_list;

pub use activity_row::{ActivityRow, ActivityStatus, RowDensity};
pub use code_block::CodeBlock;
pub use conn_indicator::{ConnectionIndicator, IndicatorTone};
pub use diff_hunk::DiffHunkView;
pub use diff_line::DiffLineView;
pub use metric_tile::{DeltaDirection, DeltaTone, MetricDelta, MetricTile};
pub use queue_table::{QueueColumn, QueueItem, QueueTable};
pub use sparkline::{Sparkline, SparklineShape, SparklineTone, bar_positions, polyline_points};
pub use status_pill::{StatusPill, StatusPillKind, StatusPillShape};
pub use table::{MdTable, TableAlignment};
pub use toast::{Toast, ToastAction, ToastId, ToastItem, ToastSeverity};
pub use virtual_list::{DEFAULT_OVERSCAN, VirtualScrollContainer, spacer_heights, visible_range};

#[cfg(test)]
mod smoke_tests {
    /// Smoke test: crate compiles and the test module runs.
    #[test]
    fn crate_smoke() {
        assert_eq!(2 + 2, 4);
    }
}
