# State  -  theatron

## Current phase

**v1.2.0 released 2026-05-08.** Additive minor bundling 5 helpers
across keryx + gramma + themelion + bathron (PRs #86 / #87 / #91)
plus the post-tag wave doc-link fix (#85). No breaking changes;
consumers re-pin via `tag = "v1.2.0"` at their own pace. Workspace
MSRV bumped 1.85 → 1.86 (transitive vello_shaders requirement);
`rust-toolchain.toml` channel `nightly`. v1.1's deferred cut
criterion #1 (consumer-pull validation) **satisfied** via aletheia
PR #40 land 2026-05-08; criterion #2 (PR-B logging migration)
substrate-side complete (bathron knobs in this release); proskenion-
side migration ships next as PR-B.2 against this tag.

**v1.1.0 released 2026-05-04.** Additive minor bundling 31 PRs
(#50-#82, sans #46-#49 docs and #80 superseded by #81) on top of
v1.0.0. No breaking changes. Consumers re-pin via
`tag = "v1.1.0"` at their own pace.

**v1.0.0 released 2026-05-02.** First stable release; API frozen
across all eight Greek-named crates (themelion, mekhane, skeue,
gramma, keryx, bathron, parodos, dokimasia) per `_meta/SEMVER.md`.
Consumers pinning `tag = "v1.0.0"` keep working unchanged — v1.1.0
+ v1.2.0 are fully additive.

Updated: 2026-05-08.

### 2026-05-03 maintenance wave (PRs #37-#49)

Polish post-v1.0.0 with no public API changes — distribution docs, lint
hardening, test coverage, examples.

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
| #46 | STATE.md refresh post-polish | docs |
| #47 | CHANGELOG `[Unreleased]` populate | docs |
| #48 | `gramma::highlight` test coverage expansion | +tests |
| #49 | `themelion::ThemeMode` test coverage | +tests |

### 2026-05-03 → 2026-05-04 v1.1 surface wave (PRs #50-#77)

28 PRs, all additive, all merged. Drove the SemVer cut from v1.0.1
(patch) to v1.1.0 (minor) the instant PR #50 added the first new
public function. Each PR is a self-contained additive helper /
accessor / predicate / variant / component, with rustdoc + tests +
CHANGELOG entry. Detail per item is in `_meta/CHANGELOG.md`
`## [Unreleased]`.

By crate:

**bathron** (7 additions across 7 PRs)
- `logging::init_with_stderr(config, also_to_stderr)` — PR #50
- `dialogs::{info, warn, error, confirm}` + `MessageKind` — PR #54
- `settings::Settings::contains` (key-presence) — PR #60
- `SettingsError::path` + `::lookup_key` accessors — PR #64
- `LoggingError::path` accessor — PR #65
- `settings::Settings::keys()` (top-level enumeration) — PR #73
- `settings::Settings::remove(key)` (CRUD symmetry with `set`) — PR #74

**keryx** (6 additions across 6 PRs)
- `ApiError` variants `Timeout` / `RateLimited` / `BadResponse` — PR #51
- `ApiError::operation()` accessor — PR #56
- `ApiError::is_retryable()` predicate — PR #59
- `ApiError::status_code()` accessor — PR #63
- `ApiError::retry_after()` accessor — PR #71
- `ApiError::is_client_error()` + `::is_server_error()` HTTP-class
  predicates — PR #77

**parodos** (4 additions across 4 PRs)
- `clipboard::supports_osc52()` capability probe — PR #58
- `theme::ColorDepth` predicates (`is_truecolor` / `is_256` / `is_basic`
  / `has_256`) — PR #69
- `theme::ThemeMode::is_dark` + `::is_light` predicates — PR #70
- `theme::ThemeMode::from_label` + `::all` helpers — PR #72

**themelion** (3 additions across 3 PRs)
- `ThemeMode::from_label` + `::all` helpers — PR #57
- `ResolvedTheme::is_dark` + `::is_light` predicates — PR #61
- `ThemeMode::is_dark` + `::is_light` + `::is_system` predicates — PR #76

**gramma** (4 additions across 4 PRs)
- `diff::DiffStats` aggregate (files_changed / additions / deletions
  + `from_files` / `total_lines_changed` / `is_empty`) — PR #62
- `diff::ChangeType` predicates (`is_add` / `is_remove` / `is_context`
  / `is_change`) — PR #67
- `diff::DiffViewMode::is_unified` + `::is_side_by_side` — PR #68
- `diff::DiffStats` shape helpers (`net_change` / `is_pure_addition`
  / `is_pure_deletion`) — PR #75

**skeue** (3 new components across 3 PRs)
- `EmptyState` component — PR #52
- `Spinner` component (pure-CSS rotation, 3 sizes) — PR #55
- `ErrorState` component (sibling to `EmptyState`) — PR #66

**CI / tooling** (1 PR)
- `cargo doc -D warnings` stage in `.kanon-ci.toml` — PR #53

### v1.1.0 tag cut — 2026-05-04

Tagged on 2026-05-04 from `main` after the 31-PR additive wave. Cut
criteria #3 (no fmt/clippy/doc backlog) was met. Criteria #1/#2
(consumer-pull validation, PR-B logging migration) were **deferred
to v1.2** rather than blocking the v1.1 cut — the rationale being
that consumers can't validate a wave that hasn't been tagged, so
shipping the tag IS the validation step. Any v1.2 surface
additions go through the fresh `## [Unreleased]` section.

### 2026-05-08 post-tag wave (PRs #85-#87) + cut-criterion #1 satisfied

Three PRs landed this session via `manual-pr-merge` bypass while
forge CI was queue-contended (3+ concurrent workspace nextest jobs
from sister codex teams on aletheia drove run latency past 70 min).
Local gates (fmt + check + clippy `-D warnings` + nextest +
`RUSTDOCFLAGS='-D warnings' cargo doc` + `kanon lint --summary`
PR-scoped) were green for every PR before bypass.

| PR | Subject | Scope |
|---|---|---|
| #85 | `bathron`/`mekhane` broken-intra-doc-links repair | docs (lib doctests on bathron `settings.rs:242` + mekhane `hotkey.rs:4`) |
| #86 | `keryx::response` — `ensure_success` + `decode_json` helpers | feat (16 unit tests, new module) |
| #87 | v1.2 surface bundle: `keryx::url` + `gramma::syntax` + `themelion::ThemeMode::{slug, from_slug}` | feat (3 STRONG candidates from 2026-05-09 rescan; 19 unit tests + 9 doctests across 3 crates) |

**v1.2 cut criterion #1 (consumer-pull validation) now SATISFIED** —
aletheia PR #40 (re-pin koilon/skene/proskenion to `tag = "v1.1.0"`)
shipped 2026-05-08 22:46 via local cargo-check on the 3 consumer
crates plus bypass-merge (kanon-server CI continued unstable). Real
consumer code now compiles against the v1.1.0 substrate.

PR #80 (themelion `ResolvedTheme::from_str + ::all`) closed-as-
superseded — verification post-#81 confirmed `parse_data_attr` is
also case-sensitive (`crates/themelion/src/theme.rs:547`), making
the two PRs functionally identical.

**v1.2 surface accumulated under `[Unreleased]`** (5 additions
across 3 crates):

- `keryx::response::ensure_success` + `decode_json` (2 STRONG, 2026-05-04 audit)
- `keryx::url::encode_path_segment` (STRONG, 2026-05-09 rescan)
- `gramma::syntax::language_from_path` + `language_from_extension` (STRONG, 2026-05-09 rescan)
- `themelion::ThemeMode::slug` + `from_slug` (STRONG, 2026-05-09 rescan)

Per-detail entries in `_meta/CHANGELOG.md` `## [Unreleased]`.

### Active fleet consumers at v1.1 (post-#40 land)

### Active fleet consumers at v1.1

- `aletheia/crates/theatron/koilon`  -  consuming `parodos` for theme,
  sanitize, clipboard, highlight, hyperlink, fuzzy. **Pinned to
  `tag = "v1.1.0"` as of 2026-05-08** (aletheia PR #40 squash 0b3cdd8c).
- `aletheia/crates/theatron/proskenion`  -  consuming `themelion`,
  `skeue`, `gramma`. **Pinned to `tag = "v1.1.0"` as of 2026-05-08.**
- `aletheia/crates/theatron/skene`  -  consuming `keryx`. **Pinned to
  `tag = "v1.1.0"` as of 2026-05-08.**
- `kanon/crates/chalkeion`  -  ported through Phase 4 Tier 5
  (sitting on unmerged feature branches; landing on kanon main is
  Cody's queue)

## Locked decisions

- Dual license: Apache-2.0 OR MIT
- 8 crates: themelion, mekhane, skeue, gramma, keryx, bathron, parodos, dokimasia
  (renamed from `theatron-{core,blitz,components,markdown,net,platform,tui,lint}`
  per fleet naming convention — see PR #21)
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
  findings are not a path to clean lint output — fix the underlying
  issue (destructure-to-consume, restructure, etc.) instead.
- v1.1 wave merge gate: `~/menos-ops/bin/manual-pr-merge` invoked with
  `RUSTC_WRAPPER=` override (sccache crashes under concurrent kanon-ci
  build pressure). Runs the full local gate (fmt + check + clippy
  -D warnings) on a rebased squash before advancing main.

## Plan reference

See `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md` for the
full multi-month plan and Phase 0 progress capture.

## Active blockers

- **kanon-server CI infrastructure instability** (still active as of
  2026-05-08). Queue contention + LSM-tree recovery on restart
  remain symptoms; mitigation is per-PR `manual-pr-merge` bypass
  with `RUSTC_WRAPPER=` override (used 4× this session). No longer
  blocks aletheia PR #40 (that landed via bypass on 2026-05-08).
  Lane: kanon CI infra (Cody / kanon-server work).
- **chalkeion main on kanon** — unmerged feature branches; Cody's lane.
  Blocks chalkeion Phase 5a (operator-dispatch view, chalkeion-side
  polish) and Phase 5b chalkeion-side (operator dispatch view).
- **akroasis GH #118 + akroasis own roadmap** — doubly blocked. GH
  #118 surface-order debate (TUI → web → desktop vs desktop-first
  via theatron) needs resolution AND akroasis itself is at Phase 02
  (kerykeion mesh networking complete); zero HTTP API surface,
  STATE.md locks order TUI → web → desktop. Phase 6 akroasis-
  desktop work doubly blocked until both clear.
- **PR-B proskenion::logging migration** — three options posted on
  theatron PR #50 timeline. proskenion's local `logging.rs` (88 LOC)
  diverges from `bathron::logging::init_with_stderr` in 5 behavioural
  details (log dir, file name, ANSI, EnvFilter directive, stderr
  trigger). Decision needed before retiring proskenion's local
  module. **Last remaining v1.2 cut criterion** (criterion #1 —
  consumer-pull validation — was satisfied 2026-05-08 via PR #40).
- Gate 2 (dioxus#2138 tray-icon upstream) is closed via composition
  layer in `mekhane`; not a blocker.

### Resolved blockers (since 2026-05-04)

- **D-062** (kanon PR #517) — themelion harmonia/theatron consumer-
  pressure decision. **Resolved 2026-05-04** via fleet-wide collision
  scan in PR #517 comment chain + NAMING rule scope captured. Phase 6
  harmonia-desktop no longer blocked on D-062 (still blocked by
  harmonia's own roadmap; out of scope here).
- **aletheia PR #40** (consumer-pin v1.0.0 → v1.1.0). **Resolved
  2026-05-08 22:46** via local `cargo check -p koilon -p skene` +
  `cargo check --manifest-path crates/theatron/proskenion/Cargo.toml`
  + bypass-merge (squash 0b3cdd8c). v1.2 cut criterion #1 satisfied.
- **Aletheia main red on `cargo nextest`** (since 2026-05-03 17:43).
  **Diagnosed 2026-05-08** via codex middle-manager Team B (filed as
  aletheia issue #215). Root cause: NOT a test regression — CI build
  infra (sccache connection reset during ndarray compile in original
  run; subsequent runs time out ~40 min before reaching test
  execution). Recommended fix: rerun with `RUSTC_WRAPPER=` or healthy
  sccache + longer timeout; lane is kanon CI infra, not theatron.

## Next steps

If resuming this work cold:

1. **Resolve PR-B logging migration design** (last v1.2 cut
   criterion). Three options posted on theatron PR #50 timeline.
   Operator decision needed; once decided, implementation is a
   small additive change in `bathron::logging` (if the chosen
   option requires bathron tweaks) plus a proskenion-side switch
   from local `logging.rs` to `bathron::logging::init_with_stderr`.
   Once PR-B lands, **v1.2 cut is feasible**: both deferred cut
   criteria from v1.1 will be satisfied.
2. **Continue waiting on chalkeion main + akroasis** for Phase 5a /
   Phase 6 advancement — theatron-side work outside ongoing v1.2
   surface accumulation is in maintenance mode pending these.
3. **v1.2 surface accumulation continues** under `## [Unreleased]`.
   5 additive helpers landed 2026-05-08 (see post-tag wave section
   above). 7 MODERATE candidates from 2026-05-09 rescan are
   documented in `~/menos-ops/research-archive/2026-05-09-theatron-v1.2-consumer-pull-rescan.md`
   and tracked as theatron forge issues for incremental shipment
   when consumer pull strengthens. Next consumer-pull rescan can
   re-walk koilon / proskenion / skene once they've absorbed the
   v1.1.0 pin into normal feature work.
4. **v1.2 cut.** Once PR-B is resolved + landed, cut v1.2.0 from
   `main` per `_meta/RELEASE.md`. The `[Unreleased]` accumulator
   becomes the v1.2.0 entry; tag pushed at the cut sha.
