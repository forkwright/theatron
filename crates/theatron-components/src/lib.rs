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
//! - [`virtual_list`] — virtual scrolling primitives (extracted verbatim
//!   from aletheia/proskenion, 100% generic)
//! - [`table`] — markdown table renderer (extracted verbatim, depends
//!   only on public `pulldown_cmark::Alignment`)
//! - [`toast`] — toast notification with API redesign: `Toast`, `ToastId`
//!   types defined here; `ToastDispatcher` trait + `EventHandler`
//!   callbacks replace aletheia's `use_toast` hook + `NavAction` parser
//!
//! Spike provenance: `/tmp/theatron-extract-spike/`. Verified compile +
//! 10 tests passing under Dioxus 0.7.

pub mod table;
pub mod toast;
pub mod virtual_list;

pub use table::MdTable;
pub use toast::{Toast, ToastAction, ToastDispatcher, ToastId, ToastItem, ToastSeverity};
pub use virtual_list::{DEFAULT_OVERSCAN, VirtualScrollContainer, spacer_heights, visible_range};
