#![deny(missing_docs)]

//! σκευή (skeue, props/equipment) — generic Dioxus components per kanon
//! DESIGN-TOKENS.md component anatomy.
//!
//! Each component implementation includes `// References:` blocks citing
//! external sources where their anatomy was sourced (Linear, Sourcehut,
//! Fly.io, Grafana, Radicle) — folds in kanon discussion docket #40.
//!
//! ## Components seeded from extraction spike (W1)
//!
//! - [`virtual_list`] — virtual scrolling primitives + helpers
//! - [`table`] — markdown table renderer with theatron-local
//!   [`TableAlignment`] (decoupled from
//!   `pulldown_cmark::Alignment` so consumers don't have to share a
//!   pulldown-cmark major version with us — `From` impl provided)
//! - [`toast`] — toast notification with `EventHandler<ToastId>` /
//!   `EventHandler<ToastAction>` callbacks replacing aletheia's
//!   `use_toast` hook + `NavAction` parser

pub mod activity_row;
pub mod badge;
pub mod code_block;
pub mod conn_indicator;
pub mod diff_hunk;
pub mod diff_line;
pub mod empty_state;
pub mod error_state;
pub mod metric_tile;
pub mod queue_table;
pub mod sparkline;
pub mod spinner;
pub mod status_pill;
pub mod table;
pub mod toast;
pub mod virtual_list;

pub use activity_row::{ActivityRow, ActivityStatus, RowDensity, RowSemantics};
pub use badge::{BadgeColors, badge_style};
pub use code_block::CodeBlock;
pub use conn_indicator::{ConnectionIndicator, IndicatorTone};
pub use diff_hunk::DiffHunkView;
pub use diff_line::DiffLineView;
pub use empty_state::EmptyState;
pub use error_state::ErrorState;
pub use metric_tile::{DeltaDirection, DeltaTone, MetricDelta, MetricTile};
pub use queue_table::{QueueColumn, QueueItem, QueueTable};
pub use sparkline::{Sparkline, SparklineShape, SparklineTone};
pub use spinner::{Spinner, SpinnerSize};
pub use status_pill::{StatusPill, StatusPillKind, StatusPillShape};
pub use table::{MdTable, TableAlignment};
pub use toast::{Toast, ToastAction, ToastActionId, ToastId, ToastItem, ToastSeverity};
pub use virtual_list::{DEFAULT_OVERSCAN, VirtualScrollContainer, spacer_heights, visible_range};

#[cfg(test)]
mod public_surface_tests {
    //! Behavioral tests exercised through the crate re-exports.

    /// INVARIANT: top spacer + rendered window + bottom spacer tile the
    /// full scroll height exactly, so virtual scrolling never drifts.
    #[test]
    fn virtual_list_geometry_tiles_the_full_scroll_height() {
        let (start, end) = crate::visible_range(800.0, 600.0, 100, 80.0, crate::DEFAULT_OVERSCAN);
        assert_eq!((start, end), (0, 29));
        let (top, bottom) = crate::spacer_heights(start, end, 100, 80.0);
        assert!(top.abs() < f64::EPSILON);
        // 100 items * 80px = 8000px total; window covers 29 * 80 = 2320px.
        assert!((bottom - 5680.0).abs() < f64::EPSILON);
    }
}
