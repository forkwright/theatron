# Dashboard Lifecycle Registry

Updated: 2026-05-22.

This registry records dashboard and view lifecycle state before agents
edit view code. Theatron is shared UI infrastructure, not an application
dashboard repository. Local entries are examples or reusable view
primitives; active product dashboards live in consumer repositories.

## State Vocabulary

| State | Meaning | Agent rule |
|---|---|---|
| active | Production or release-bound dashboard/view surface consumed by fleet users. | Preserve migration compatibility and verify against the owning consumer before broad layout changes. |
| development | Example, smoke, or work-in-progress dashboard/view surface used to exercise the substrate. | Use for representative patterns, but do not treat as a canonical product dashboard unless the registry entry says so. |
| component | Reusable component or view primitive that dashboard surfaces compose. | Prefer additive changes and keep props, accessibility, and design-token behavior stable for downstream dashboards. |
| legacy | Retired, transitional, or superseded dashboard/view surface retained for compatibility or migration reference. | Do not extend except for compatibility fixes; migrate callers to the listed target first. |

## Registry

| ID | State | Kind | Path or repo | Migration state | Before editing |
|---|---|---|---|---|---|
| `aletheia-proskenion` | active | external consumer | `forkwright/aletheia`: `crates/theatron/proskenion` | Active Dioxus dashboard consumer of `themelion`, `skeue`, and `gramma`, pinned to theatron v1.1.0 as of aletheia PR #40. | Check current aletheia pin and proskenion state. Keep theatron-side component/API changes additive unless a coordinated migration exists. |
| `kanon-chalkeion` | active | external consumer | `forkwright/kanon`: `crates/chalkeion` | Active fleet consumer target for operator dispatch views; mainline landing remains blocked on kanon-side branches. | Check kanon/chalkeion branch status before assuming the latest view shape. Avoid removing shared component affordances used by operator dispatch. |
| `examples-full-app` | development | local example | `examples/full_app` | Reference app demonstrating bathron, mekhane, skeue, gramma, and keryx integration. It is a substrate exercise surface, not a production dashboard. | Check `examples/full_app/README.md` and `_meta/INTEGRATION.md`. Keep usage representative of stable public APIs. |
| `examples-minimal` | development | local example | `examples/minimal` | Minimal Dioxus + Blitz launch example for smoke validation. | Check `examples/minimal/README.md`. Keep the surface intentionally small. |
| `skeue-components` | component | local crate | `crates/skeue` | Reusable Dioxus components for fleet dashboard composition, including empty/error/loading/data display primitives and diff views. | Check `crates/skeue/src/lib.rs` and component rustdoc. Validate Rust changes with cargo check and clippy. |
| `gramma-view-state` | component | local crate | `crates/gramma` | Pure markdown, syntax, and diff state used by rendered file and diff views; Dioxus rendering lives in consumers or `skeue`. | Check `crates/gramma/src/lib.rs` and diff/syntax module docs. Validate Rust changes with cargo check and clippy. |
| `legacy-prefixed-crates` | legacy | retired layout | none local | The pre-v1 prefixed crate layout was renamed to the eight Greek-named crates in PR #21 and is no longer a live dashboard/view surface. | Do not recreate `theatron-{core,blitz,components,markdown,net,platform,tui,lint}`. Use current crate names from `README.md` and `_llm/architecture.toml`. |

## Edit Boundary

- There is no local `src/dashboards/` tree in this repository as of
  2026-05-22.
- Active dashboard edits belong in the owning consumer repository.
- Theatron-side changes should focus on shared substrate APIs,
  examples, or reusable components listed above.
- If a new local dashboard/view directory is added, add a registry row
  in the same change before modifying that surface.
