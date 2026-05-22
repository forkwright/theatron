# Dashboard Lifecycle Registry

Updated: 2026-05-22.

This registry records dashboard and view lifecycle state before agents
edit view code. Theatron is shared UI infrastructure, not an application
dashboard repository. Local entries are examples or reusable view
primitives; active product dashboards live in consumer repositories.

## Reality Check

Issue #3 cites `src/dashboards/LIFECYCLE.md:5` from the Ergon
strip-mine source material. The current theatron repository has no
local `src/dashboards/` tree; dashboard lifecycle state therefore lives
in this `_meta/` registry until a real local dashboard surface or
machine-readable registry exists.

## State Vocabulary

| State | Meaning | Agent rule |
|---|---|---|
| active | Production or release-bound dashboard/view surface consumed by fleet users. | Preserve migration compatibility and verify against the owning consumer before broad layout changes. |
| development | Example, smoke, or work-in-progress dashboard/view surface used to exercise the substrate. | Use for representative patterns, but do not treat as a canonical product dashboard unless the registry entry says so. |
| component | Reusable component or view primitive that dashboard surfaces compose. | Prefer additive changes and keep props, accessibility, and design-token behavior stable for downstream dashboards. |
| legacy | Retired, transitional, or superseded dashboard/view surface retained for compatibility or migration reference. | Do not extend except for compatibility fixes; migrate callers to the listed target first. |

## Registry

| ID | State | Kind | Owner | Path or repo | Migration state | Edit status | Validation before merge |
|---|---|---|---|---|---|---|---|
| `aletheia-proskenion` | active | external consumer | aletheia | `forkwright/aletheia`: `crates/theatron/proskenion` | Active Dioxus dashboard consumer of `themelion`, `skeue`, and `gramma`, pinned to theatron v1.1.0 as of aletheia PR #40. | Coordinate with the consumer; keep theatron-side component/API changes additive unless a migration PR exists. | Check the current aletheia pin and proskenion state, then validate the affected theatron crates plus the consumer branch when available. |
| `kanon-chalkeion` | active | external consumer | kanon | `forkwright/kanon`: `crates/chalkeion` | Active fleet consumer target for operator dispatch views; mainline landing remains blocked on kanon-side branches. | Coordinate with chalkeion before changing shared component affordances used by operator dispatch. | Check kanon/chalkeion branch status before assuming the latest view shape; validate against the branch that owns the operator dispatch surface. |
| `examples-full-app` | development | local example | theatron | `examples/full_app` | Reference app demonstrating bathron, mekhane, skeue, gramma, and keryx integration. It is a substrate exercise surface, not a production dashboard. | Safe for representative example edits that stay on stable public APIs. | Check `examples/full_app/README.md` and `_meta/INTEGRATION.md`; run the relevant example or cargo check when Rust changes. |
| `examples-minimal` | development | local example | theatron | `examples/minimal` | Minimal Dioxus + Blitz launch example for smoke validation. | Safe for small launch-pattern edits; do not expand into a full product dashboard. | Check `examples/minimal/README.md`; run the example or cargo check when Rust changes. |
| `skeue-components` | component | local crate | theatron | `crates/skeue` | Reusable Dioxus components for fleet dashboard composition, including empty/error/loading/data display primitives and diff views. | Prefer additive props and stable accessibility/design-token behavior for downstream dashboards. | Check `crates/skeue/src/lib.rs` and component rustdoc; validate Rust changes with cargo check and clippy. |
| `gramma-view-state` | component | local crate | theatron | `crates/gramma` | Pure markdown, syntax, and diff state used by rendered file and diff views; Dioxus rendering lives in consumers or `skeue`. | Keep state types and parsing helpers backward-compatible unless a semver migration is documented. | Check `crates/gramma/src/lib.rs` and diff/syntax module docs; validate Rust changes with cargo check and clippy. |
| `legacy-prefixed-crates` | legacy | retired layout | theatron | none local | The pre-v1 prefixed crate layout was renamed to the eight Greek-named crates in PR #21 and is no longer a live dashboard/view surface. | Do not extend or recreate the retired crate names; migrate references to the current crate names. | Use current crate names from `README.md` and `_llm/architecture.toml`; docs-only reference cleanup is sufficient unless code still imports retired names. |

## Edit Boundary

- There is no local `src/dashboards/` tree in this repository as of
  2026-05-22.
- Active dashboard edits belong in the owning consumer repository.
- Theatron-side changes should focus on shared substrate APIs,
  examples, or reusable components listed above.
- If a new local dashboard/view directory is added, add a registry row
  in the same change before modifying that surface.
