# Changelog — theatron

This file is the authoritative release-note record for theatron's
eight-crate set. Entries follow [Keep a Changelog](https://keepachangelog.com)
formatting and theatron's [SemVer policy](./SEMVER.md).

The eight crates ship together at a single workspace version; one
entry per release covers all eight.

## [Unreleased]

Active development on the road to v1.0. See `_meta/STATE.md` and
`_meta/ROADMAP.md` for current progress.

---

## v1.0 candidate scope

The v1.0 cut formalizes the public API of all eight crates:

- **`themelion`** — theme provider, design-token consumer surface,
  ThemeMode enum, ThemeProvider component, ThemeToggle. Stable as of
  v1.0; future themes layer in via additive variants.
- **`mekhane`** — desktop windowing wrapper. `launch`,
  `launch_cfg`, `launch_cfg_with_props`, `launch_cfg_with_props_ext`
  (the latter accepting optional menu + hotkey configs). Hooks:
  `use_tray_icon_event_handler`, `use_tray_menu_event_handler`,
  `use_app_menu_event_handler`, `use_global_hotkey_event_handler`.
  Cargo features: `menus`, `global-hotkeys`, `default-icon`.
  `tray::default_icon` for PNG-bytes-to-Icon conversion.
- **`skeue`** — visual components per
  `basanos/standards/DESIGN-TOKENS.md` anatomy. StatusPill,
  ConnectionIndicator, MetricTile, Sparkline, ActivityRow,
  QueueTable, MdTable, VirtualScrollContainer, Toast/ToastItem,
  CodeBlock, DiffHunkView, DiffLineView. Each carries a stable
  `#[component]` Props struct.
- **`gramma`** — markdown rendering primitives. `highlight` module
  (syntect-backed code highlighting, no Dioxus dep) and `diff`
  module (unified-diff parser + state types). Public API frozen at
  v1.0.
- **`keryx`** — HTTP / SSE client primitives. `SseStream`,
  `SseEvent`, `ApiError`. Generic over consumer DTO types so each
  fleet repo (kanon, aletheia, harmonia, akroasis) layers its own
  contract on top.
- **`bathron`** — OS services: notifications, file dialogs, settings
  KV store, structured logging. Each module gated behind a cargo
  feature so consumers pay only for what they use.
- **`parodos`** — TUI infrastructure. Initial v1.0 surface: `fuzzy`
  matching. Additional modules (theming, hyperlinks, clipboard,
  sanitization, terminal highlight) extract progressively from
  aletheia/koilon.
- **`dokimasia`** — design-token enforcement linter. Stable rule set:
  every `MANIFEST/`, `STANDARDS/`, `BEHAVIOR/` rule documented.
  v1.0 freezes the rule namespace; new rules add at minor bumps.

### Stability boundaries (v1.0)

Locked at v1.0:
- The crate names + their roles.
- Public-API names + signatures (per the rules in `SEMVER.md`).
- The cargo-feature namespace (every existing feature stays
  available; new features only add).
- Wire-DTO shapes (in `keryx`).
- Design-token contracts (per `basanos/standards/DESIGN-TOKENS.md`).

Free to evolve post-v1.0:
- Component rendering output (HTML element structure, CSS class
  values) — operator-visible behaviour stays consistent through the
  design-token contract, but how each component fulfills that
  contract is implementation detail.
- `tracing` log payloads (observability, not API).
- Internal module structure (`pub(crate)` reorganization).
- Performance characteristics (we'll make things faster without
  promising the timing).

### Migration path 0.x → 1.0

Consumers pinned to a specific `0.1.0` rev:
1. Bump the workspace pin to `tag = "v1.0.0"`.
2. Run `cargo check` — any break here is documented in the v1.0
   release notes below with the specific migration.
3. Update consumer code per the migrations below.
4. Re-run the consumer's lint + test suite.

The intent is for the 0.x → 1.0 bump to be near-zero-touch for
consumers tracking a recent post-rename rev (post `9bf1e9e`
"refactor: rename crates to standalone Greek names").

---

## v0.1.0 (current pre-release)

Initial pre-release of the eight-crate set after the Phase 0
fork-vs-composition decision. The chalkeion plan tracks Phase 0
through Phase 6; theatron's v0.1.x line carries the work through
the chalkeion + harmonia-desktop + akroasis-desktop fleet rollouts.

Notable milestones reached on the 0.1.x line:
- Phase 0 Gate 2: tray support shipped as composition over unmodified
  Dioxus + Blitz upstream — no fork, no `[patch.crates-io]` block.
- W1 spike: 500 LOC removed from aletheia/proskenion in favour of
  thin theatron consumers.
- W2: complete component library landed per DESIGN-TOKENS.md anatomy.
- W3: `gramma` extraction (syntax highlight + diff parsing).
- W4: `keryx` SSE primitives extracted from aletheia/skene.

Forward path to v1.0:
- Complete the koilon → parodos extraction wave.
- Implement `bathron`'s four OS-service modules.
- Land at least one fleet consumer (chalkeion) at parity with its
  pre-extraction baseline.
- Cut the v1.0 release once consumer churn from theatron itself
  reaches zero on the chalkeion + aletheia/proskenion baseline.
