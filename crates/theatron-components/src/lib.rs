//! theatron-components — generic Dioxus components per kanon DESIGN-TOKENS.md
//! component anatomy.
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

pub mod conn_indicator;
pub mod status_pill;
pub mod table;
pub mod toast;
pub mod virtual_list;

pub use conn_indicator::{ConnectionIndicator, IndicatorTone};
pub use status_pill::{StatusPill, StatusPillKind, StatusPillShape};
pub use table::{MdTable, TableAlignment};
pub use toast::{Toast, ToastAction, ToastId, ToastItem, ToastSeverity};
pub use virtual_list::{DEFAULT_OVERSCAN, VirtualScrollContainer, spacer_heights, visible_range};
