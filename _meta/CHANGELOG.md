# Changelog — theatron

This file is the authoritative release-note record for theatron's
eight-crate set. Entries follow [Keep a Changelog](https://keepachangelog.com)
formatting and theatron's [SemVer policy](./SEMVER.md).

The eight crates ship together at a single workspace version; one
entry per release covers all eight.

## [Unreleased]

Post-v1.0 polish wave landed 2026-05-03 while waiting on chalkeion
landing. Mostly additive surfaces, lint hardening, tests, docs.
The `bathron::logging::init_with_stderr` addition is the first
genuinely new public API since v1.0.0  -  flips the upcoming tag from
patch (`v1.0.1`) to minor (`v1.1.0`).

### Added

- **`bathron::logging::init_with_stderr(config, also_to_stderr)`**
  (PR-A of the proskenion logging-migration sequence) -- new public
  function next to existing `init`. When `also_to_stderr = true`,
  adds an stderr layer alongside the daily-rotated file appender.
  Both layers share the same env-filter (`RUST_LOG` if set,
  otherwise the configured `LogConfig::level`). Lifts the stderr-on-
  verbose pattern out of proskenion's local `logging.rs` so any
  fleet desktop surface can opt into it via a single function call.
  `init(config)` now delegates to `init_with_stderr(config, false)`
  (no behaviour change for existing callers).
- **`keryx::ApiError` variants: `Timeout`, `RateLimited`, `BadResponse`**
  (PR #51). Three additive variants on the `#[non_exhaustive]`
  `ApiError` enum, splitting failure modes that previously had to
  fold into the existing `Http` / `Server` variants:
    - `Timeout { operation, timeout_secs }` -- caller-detected via
      `reqwest::Error::is_timeout`. Useful for retry layers that
      distinguish timeouts (worth retrying) from connection refusals.
    - `RateLimited { operation, retry_after_secs }` -- 429 responses.
      Carries the `Retry-After` header in seconds when supplied.
    - `BadResponse { operation, source: serde_json::Error }` -- 2xx
      response with an unparseable body. Common when server schema
      drifts.

  Existing consumers using a wildcard `match` arm continue to work;
  consumers can opt into the new variants for richer error routing.
- **`keryx::ApiError::operation() -> Option<&'static str>`** (PR
  #56). Accessor returning the operation name embedded in the
  error variant, or `None` for context-free variants
  (`Auth` / `InvalidToken`). Useful for consumer logging /
  routing without manual destructuring per variant. +2 tests
  covering both branches; keryx tests: 31 → 33.
- **`keryx::ApiError::is_retryable() -> bool`** (PR #59).
  Predicate for retry layers. Returns `true` for transient
  failures (`Timeout`, `RateLimited`, 5xx `Server`, connect /
  timeout `Http`), `false` for terminal failures (4xx `Server`,
  `BadResponse`, `Auth`, `InvalidToken`, non-connect `Http`).
  Conservative default — consumers wanting more aggressive
  retries (e.g. on 4xx for idempotent reads) make their own
  judgment. +5 tests covering each branch; keryx tests:
  33 → 38.
- **`bathron::settings::Settings::contains(key) -> Result<bool, …>`**
  (PR #60). Cheaper presence check than `get::<T>(key)` when the
  consumer only needs to know whether a key is set (e.g. "has the
  user configured a theme yet?"). Skips the `DeserializeOwned`
  cost; reports presence regardless of value type. Cannot return
  `DeserializeValue` since no deserialization happens. +5 tests
  covering existing key / missing key / missing file / type-
  coercion-free presence / idempotent re-set. bathron tests:
  53 → 58.
- **`themelion::ResolvedTheme::is_dark`** + **`::is_light`**
  (PR #61). Two convenience predicates on `ResolvedTheme`.
  `theme.is_dark()` reads better than `theme == ResolvedTheme::Dark`
  at consumer call sites. Both are `const fn`, so usable in
  const contexts. +3 tests covering each branch + the
  mutually-exclusive partition. themelion tests: 19 → 22.
- **`bathron::LoggingError::path`** (PR #65). Symmetric to
  `SettingsError::path` (PR #64). Returns `Some(&Path)` for
  `CreateDir` (the only filesystem-touching variant); `None` for
  `NoStateDir` and `SetGlobalDefault`. +2 tests. bathron tests:
  61 → 63.
- **`bathron::SettingsError::path` + `::lookup_key`** (PR #64).
  Two accessors symmetric to keryx's `operation` / `status_code`
  pattern. `path()` returns `Some(&Path)` for the filesystem-
  touching variants (`CreateDir`, `ReadFile`, `WriteFile`,
  `PersistFile`) and `None` for the rest. `lookup_key()` returns
  `Some(&str)` only for `DeserializeValue` (the only variant
  that knows which key was being read). Useful for consumer code
  that wants "couldn't read setting 'theme' from /…/file" log
  lines without per-variant destructuring. +3 tests covering
  fs-variant passthrough, non-fs-variant None, deserialize-value
  key. bathron tests: 58 → 61.
- **`keryx::ApiError::status_code() -> Option<u16>`** (PR #63).
  Accessor for the HTTP status code carried by the variant.
  Returns `Some(status)` for `Server` (the wire value) and
  `RateLimited` (always `429`); `None` for variants without a
  wire response (`Http`, `Timeout`, `BadResponse`, `Auth`,
  `InvalidToken`). Symmetric to `operation()` from PR #56 and
  `is_retryable()` from PR #59 — together the three accessors
  let consumer code log / route / retry on errors without manual
  destructuring per variant. +3 tests covering server-status
  passthrough, `RateLimited` always-429, response-less variants
  None. keryx tests: 38 → 41.
- **`gramma::diff::DiffStats`** (PR #62). Aggregate stats summed
  across one or more `DiffFile`s -- `files_changed`, `additions`,
  `deletions`. Constructed via `DiffStats::from_files(&[DiffFile])`
  with saturating arithmetic on overflow. Convenience methods
  `total_lines_changed()` and `is_empty()`. Common use: a PR list
  view rendering "N files changed, +X / -Y" without iterating
  the file list at every render. +6 tests covering empty slice,
  multi-file aggregation, total-lines sum, saturating-overflow,
  default-is-empty, files-present-not-empty. gramma tests:
  45 → 51.
- **`parodos::clipboard::supports_osc52`** (PR #58). Capability
  probe symmetric to `parodos::hyperlink::supports_hyperlinks` for
  OSC 8 — checks the running terminal's env-var signals to decide
  if OSC 52 clipboard escapes will work. Returns `true` for
  iTerm2, Kitty, `WezTerm`, Alacritty, Ghostty, foot, Windows
  Terminal, tmux (with passthrough), and the `xterm` / `screen` /
  `tmux` `TERM` families. Useful when consumer code wants to know
  upfront whether `copy_to_clipboard` will fall through to OSC 52.
  Result is cached for the process lifetime. +10 tests covering
  each detection branch via the `Env` trait stub. parodos tests:
  162 → 172.
- **`themelion::ThemeMode::from_label`** + **`::all`** (PR #57).
  Two small additions on `ThemeMode`. `from_label(s) -> Option<Self>`
  parses back from the `label()` string for round-trip with settings
  storage (the inverse of `label()`); case-sensitive, returns
  `None` for unknown input. `all() -> [ThemeMode; 3]` returns the
  three modes in canonical order (Dark, Light, System) for
  rendering complete settings selectors without hard-coding the
  variant list. Replaces the inline `match label.as_str() {
  "Dark" => ThemeMode::Dark, … }` boilerplate present in the
  `examples/full_app` (PR #40) — consumers can now write
  `ThemeMode::from_label(label).unwrap_or(ThemeMode::System)`.
  +5 tests; themelion tests: 14 → 19.
- **`skeue::EmptyState` component** (PR #52). Common pattern for
  views with no content (zero-result search, fresh app launch,
  disconnected state). Slots: `title` (required, accessible name),
  optional `message` / `icon` / `action`. Marked `role=status`,
  decorative icon `aria-hidden`, action slot the consumer's
  responsibility for keyboard focus + label. Joins the existing
  12-component skeue inventory; brings the count to 13.
- **`bathron::dialogs` message helpers** (PR #54). Four new
  blocking-thread message-dialog functions next to the existing
  file-pick surface: `info(title, message)`, `warn(title, message)`,
  `error(title, message)` (each shows a one-button OK dialog), and
  `confirm(title, message) -> bool` (Yes/No, returns Yes-ness).
  Plus a `MessageKind` enum (`Info` / `Warning` / `Error`, default
  `Info`, `#[non_exhaustive]`). Lifts the wrapper boilerplate
  consumers were writing on top of `rfd::MessageDialog` directly
  into the bathron surface. +3 tests on the pure-logic side.
- **`skeue::Spinner` component** (PR #55). Pure-CSS rotation
  indicator for loading states. Three sizes (`SpinnerSize::Small`
  / `Medium` (default) / `Large`, mapping to 16/24/32px). Optional
  inline `label` prop; falls back to `aria-label="Loading"` when
  absent. `role=status`, `aria-live=polite`. Inlines its own
  keyframes (`@keyframes skeue-spinner-rotate`) so consumers don't
  have to add CSS to their global stylesheet. Complements
  `EmptyState` from PR #52 — together they cover the
  pre-data and no-data states every fleet desktop view needs.
  +7 SSR tests; skeue inventory: 13 → 14.
- **`examples/full_app/`** (PR #40) -- runnable Dioxus reference
  consumer exercising all six desktop-bound crates (themelion,
  mekhane, bathron, skeue, gramma, keryx) in one place. Operators
  scaffolding harmonia-desktop / akroasis-desktop / their own
  surfaces copy from this example.
- **`_meta/INTEGRATION.md`** (PR #37) -- consumer guide covering
  Cargo pin pattern, Dioxus version-pin pitfall, ThemeProvider
  usage, mekhane launch + tray + menus + global-hotkey surface,
  bathron OS-service feature flags, keryx SseStream usage,
  parodos re-export pattern, gramma module split, full skeue v1.0
  component inventory, dokimasia adoption pattern, common pitfalls.
- **`crates/parodos/benches/`** (PR #44) -- criterion benchmarks
  for `sanitize`, `hyperlink`, `fuzzy` parameterized over three
  input sizes each. `criterion = "0.5"` added to
  `[workspace.dependencies]`.
- **Per-component a11y on `skeue`** (PR #43) -- ARIA roles + names
  + live-region semantics + `# Accessibility` rustdoc section on
  all 12 components. `dioxus-ssr = "=0.7.6"` added as a `skeue`
  dev-dependency for SSR-rendered a11y tests.

### Changed

- **`#![deny(missing_docs, clippy::all, clippy::pedantic)]` workspace-wide**
  (PR #39) -- promoted from `warn` on `keryx` / `bathron` /
  `dokimasia` / `gramma` / `mekhane`. `themelion` / `parodos` /
  `skeue` retain split form (`deny(missing_docs)` +
  `warn(clippy::*)`) for legacy reasons. Workspace-wide review
  fixed 7 broken intra-doc links in `mekhane`, 5 redundant /
  ambiguous links in `parodos`, 2 redundant in `skeue`, 1 in
  `keryx`. Two `bathron` public functions destructure their
  by-value parameter at function top to consume ownership rather
  than carry an `#[expect(needless_pass_by_value)]` suppression.
- **Test coverage closed on parodos / gramma / skeue** (PR #41) --
  +91 tests across hyperlink URL regex edge cases, sanitize
  malformed-UTF-8 boundaries, clipboard `rgba_to_png`, gramma
  `parse_unified_diff` + `align_side_by_side` edge cases, skeue
  layout helpers (`bar_positions`, `polyline_points`,
  `spacer_heights`, `visible_range`).
- **`bathron::settings` cross-platform tests** (PR #42) -- +13
  tests covering path resolution, cascade, persistence, TOML
  round-trip, error paths.
- **`bathron::logging` cross-platform tests** (PR #45) -- +11
  tests covering `LogConfig` constructors / Clone / Debug,
  `resolve_log_dir` default + override + abs-path behaviour,
  `LoggingError` Display + Send/Sync/Error trait impls.

### Fixed

- **CHANGELOG**: corrected `mekhane` v2 launch fn name from the
  documentation-only-typo `launch_cfg_with_props_ext` to the
  shipped `launch_cfg_with_props_and_menu` (PR #38).
- **`examples/full_app`**: changed theme-load chain from
  `.and_then(|label| Some(...))` to `.map(|label| ...)` to satisfy
  `clippy::bind_instead_of_map` exposed by PR #39's pedantic-deny
  promotion (caught at bypass-merge gate, fixed in same PR cycle).
- **`crates/bathron/src/{logging,notifications}.rs`**: destructure
  `LogConfig` / `NotificationRequest` at function top so by-value
  pass actually consumes ownership, removing two
  `#[expect(needless_pass_by_value)]` suppressions added by the
  PR #39 first-pass and rejected per the suppressions-are-violations
  fleet rule.

### Aletheia consumer-side (not theatron, but in lockstep)

- aletheia#38 -- migrated `koilon` (`parodos` rev `4846ef4` ->
  tag `v1.0.0`), `proskenion` (`themelion + skeue + gramma` rev
  `9bf1e9e` -> tag `v1.0.0`), `skene` (`keryx` rev `9bf1e9e` ->
  tag `v1.0.0`).
- aletheia#39 -- `koilon::fuzzy` (186 LOC verbatim duplicate)
  converted to `pub use parodos::fuzzy::*` shim, matching the
  five other koilon modules already migrated in the W2
  extraction.

---

## v1.0.0 -- 2026-05-02

First stable release. Public API frozen per the `SEMVER.md` policy
across all eight crates. Consumers can now pin `tag = "v1.0.0"` and
expect additive minors / non-breaking patches until v2.0.

### Added (since v0.1.0)

- **`parodos`** -- complete TUI substrate landed (chalkeion plan W2).
  Modules: `theme` (semantic palette + ColorDepth/ThemeMode + per-depth
  per-mode constructors), `sanitize` (CSI/OSC/DCS/APC/SOS/PM escape
  stripping; C0/C1 control replacement), `hyperlink` (OSC 8 emit +
  terminal-capability detection + URL/file-path regex), `clipboard`
  (arboard + OSC52 fallback + PNG-encoded image support), `highlight`
  (syntect bridge to ratatui::Lines), `fuzzy` (subsequence matcher
  for command palettes), `env` (minimal `Env` trait + `RealEnv` impl
  so parodos doesn't depend on aletheia's koina). Lifted from
  `aletheia/koilon` after the audit confirmed extractability.
- **`mekhane`** -- v2 tray + menu + global-hotkey support landed.
  `launch_cfg_with_props_and_menu` accepts an optional menu config;
  new hooks `use_app_menu_event_handler` and
  `use_global_hotkey_event_handler`. Cargo features `menus`,
  `global-hotkeys`, `default-icon`. `tray::default_icon` for
  PNG-bytes-to-Icon conversion.
- **`bathron`** -- four OS-service modules implemented behind
  per-feature gates: `notifications` (notify-rust), `dialogs` (rfd),
  `settings` (TOML KV store with cascade), `logging`
  (tracing-subscriber adapter).
- **`dokimasia`** -- design-token enforcement linter shipped with
  `MANIFEST/cargo-patch-block` rule. Rule namespace frozen at v1.0.

### Changed

- Workspace ratatui pin bumped 0.29 → 0.30 (and crossterm 0.28 → 0.29)
  to align with downstream consumers (aletheia/koilon). parodos is
  the only theatron crate currently using either.
- All crates renamed to standalone Greek names (no `theatron-` prefix):
  `theatron-core` → `themelion`, `theatron-blitz` → `mekhane`,
  `theatron-components` → `skeue`, `theatron-markdown` → `gramma`,
  `theatron-net` → `keryx`, `theatron-platform` → `bathron`,
  `theatron-tui` → `parodos`, `theatron-lint` → `dokimasia`. (Landed
  pre-v1.0 in `9bf1e9e`; documented here for the v1.0 baseline.)

### Migration -- v0.1.x → v1.0

For consumers pinned to a recent `0.1.0` rev (post-rename):

1. Bump the workspace pin to `tag = "v1.0.0"`.
2. `cargo check` -- the only break vs. recent 0.1.x revs is the ratatui
   0.29 → 0.30 dep bump, which transitively requires `unicode-width
   >= 0.2.1`.
3. If consuming `parodos::clipboard::ClipboardContent`, note the enum
   is `#[non_exhaustive]`; add a wildcard arm to any `match`.
4. Re-run consumer lint + tests.

The aletheia/koilon consumer landed at `e9e7b537b1` already runs
against the v1.0.0 candidate without further changes.

### Smoke test

Consumer-side smoke validated:
- `forkwright/aletheia#37` (`koilon` consuming `parodos` via
  `pub use` re-exports for theme / sanitize / clipboard / highlight
  / hyperlink) builds + tests green: cargo fmt, cargo check
  --workspace --features test-core, cargo clippy -D warnings,
  cargo nextest -p koilon (906/906).

---

## v1.0 candidate scope

The v1.0 cut formalizes the public API of all eight crates:

- **`themelion`** — theme provider, design-token consumer surface,
  ThemeMode enum, ThemeProvider component, ThemeToggle. Stable as of
  v1.0; future themes layer in via additive variants.
- **`mekhane`** — desktop windowing wrapper. `launch`,
  `launch_cfg`, `launch_cfg_with_props`, `launch_cfg_with_props_and_menu`
  (the latter accepting an optional menu config). Hooks:
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
