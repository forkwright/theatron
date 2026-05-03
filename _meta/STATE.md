# State  -  theatron

## Current phase

**v1.0.0 released 2026-05-02.** API frozen across all eight Greek-named
crates (themelion, mekhane, skeue, gramma, keryx, bathron, parodos,
dokimasia) per `_meta/SEMVER.md`. Consumers pin via `tag = "v1.0.0"`.

**Post-v1.0 polish wave landed 2026-05-03** while waiting on chalkeion
landing. Nine PRs against theatron main; two against aletheia. No
public API changes (additive surfaces, lint hardening, tests, docs):

| PR | Subject | Scope |
|---|---|---|
| #37 | Phase 5b distribution + INTEGRATION.md | docs |
| #38 | CHANGELOG `launch_cfg_with_props_*` fix | docs |
| #39 | `missing_docs` deny + rustdoc warnings cleanup | lint hardening (8 crates) |
| #40 | `examples/full_app/` reference consumer | example (6 surfaces) |
| #41 | Test coverage gaps closed | +91 tests on parodos / gramma / skeue |
| #42 | `bathron::settings` cross-platform tests | +13 tests |
| #43 | `skeue` a11y audit | ARIA across all 12 components, +14 SSR tests |
| #44 | `parodos` criterion benchmarks | sanitize / hyperlink / fuzzy, 3 input sizes each |
| #45 | `bathron::logging` cross-platform tests | +11 tests |

Aletheia consumer-side:
- aletheia#38  -  three theatron consumers (`koilon`, `proskenion`,
  `skene`) migrated from `rev = "..."` pins to `tag = "v1.0.0"` per
  INTEGRATION.md canonical pattern.
- aletheia#39  -  `koilon::fuzzy` (186 LOC) converted to a 9-line
  `pub use parodos::fuzzy::*` shim (matching the other five koilon
  modules already migrated in W2).

Active fleet consumers at v1.0:
- `aletheia/crates/theatron/koilon`  -  consuming `parodos` for theme,
  sanitize, clipboard, highlight, hyperlink, fuzzy
- `aletheia/crates/theatron/proskenion`  -  consuming `themelion`,
  `skeue`, `gramma`
- `aletheia/crates/theatron/skene`  -  consuming `keryx`
- `kanon/crates/chalkeion`  -  ported through Phase 4 Tier 5
  (sitting on unmerged feature branches; landing on kanon main is
  Cody's queue)

Next: chalkeion Phase 5a polish (chalkeion-side, blocked on
chalkeion landing on kanon main) + Phase 6 fleet rollout
(harmonia-desktop blocked on D-062, akroasis-desktop blocked on
akroasis surface-order rethink GH#118).

Updated: 2026-05-03.

## Locked decisions

- Dual license: Apache-2.0 OR MIT
- 8 crates: themelion, mekhane, skeue, gramma, keryx, bathron, parodos, dokimasia
  (renamed from `theatron-{core,blitz,components,markdown,net,platform,tui,lint}`
  per fleet naming convention  -  see PR #21)
- Renderer: Dioxus + Blitz native (no wry webview fallback per chalkeion plan)
- Cross-platform: Linux-first; macOS/Windows out of scope through Phase 5
- a11y: keyboard nav required (AccessKit verified hookable). Per-component
  ARIA + live-region semantics shipped on `skeue` in PR #43.
- i18n: English-only in v1
- v1.0 cut at end of Phase 1+2 (chalkeion-tested API surface frozen).
  Versioning + release process: see `_meta/SEMVER.md`, `_meta/CHANGELOG.md`,
  `_meta/RELEASE.md`.
- Lint policy: `#![deny(missing_docs, clippy::all, clippy::pedantic)]`
  workspace-wide. Themelion + parodos retain split form
  (`deny(missing_docs)` + `warn(clippy::*)`) for legacy reasons; rest
  use the combined deny per PR #39.
- Suppressions are violations. `#[expect]` / `#[allow]` on clippy
  findings are not a path to clean lint output  -  fix the underlying
  issue (destructure-to-consume, restructure, etc.) instead.

## Plan reference

See `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md` for the
full multi-month plan and Phase 0 progress capture.

## Active blockers

- **chalkeion main on kanon**  -  unmerged feature branches; Cody's lane.
  Blocks chalkeion Phase 5a (operator-dispatch view, chalkeion-side
  polish) and Phase 5b chalkeion-side (operator dispatch view).
- **D-062** (kanon PR #517)  -  themelion harmonia/theatron consumer-
  pressure decision. Blocks Phase 6 harmonia-desktop migration.
- **akroasis GH #118**  -  surface-order rethink (TUI -> web -> desktop
  vs desktop-first via theatron). Blocks Phase 6 akroasis-desktop side.
- Gate 2 (dioxus#2138 tray-icon upstream) is closed via composition
  layer in `mekhane`; not a blocker.

## Next steps

See `~/dev/kanon/projects/chalkeion/STATE.md` for current next-steps
and phase tracking. Theatron-side work is in maintenance mode pending
the three blockers above.

Likely v1.1 surface candidates if minor-bumped (none committed):
- `bathron::logging` stderr-layer + verbose flag (would let proskenion
  retire its 88-LOC `logging.rs` in favour of `bathron::logging::init`).
- Additional `keryx::ApiError` variants (richer error vocabulary
  beyond the current `Http` / `Auth` / `Server` / `InvalidToken` set).
- Cross-platform `bathron::dialogs` + `bathron::notifications`
  hardening (they ship as Linux-first in v1.0; macOS / Windows
  parity is non-blocking but on the horizon).

These are not yet roadmapped; they get added to `_meta/CHANGELOG.md`
under `## [Unreleased]` once they earn it via real consumer demand.
