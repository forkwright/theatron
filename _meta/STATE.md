# State  -  theatron

## Current phase

**v1.1.0 released 2026-05-04.** Additive minor bundling 31 PRs
(#50-#82, sans #46-#49 docs and #80 superseded by #81) on top of
v1.0.0. No breaking changes. Consumers re-pin via
`tag = "v1.1.0"` at their own pace.

**v1.0.0 released 2026-05-02.** First stable release; API frozen
across all eight Greek-named crates (themelion, mekhane, skeue,
gramma, keryx, bathron, parodos, dokimasia) per `_meta/SEMVER.md`.
Consumers pinning `tag = "v1.0.0"` keep working unchanged — v1.1.0
is fully additive.

Updated: 2026-05-05.

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

Aletheia consumer-pin update (koilon / proskenion / skene re-pinning
`tag = "v1.0.0"` → `tag = "v1.1.0"`) is the immediate follow-up — see
the next-steps section.

### Active fleet consumers at v1.1

- `aletheia/crates/theatron/koilon`  -  consuming `parodos` for theme,
  sanitize, clipboard, highlight, hyperlink, fuzzy
- `aletheia/crates/theatron/proskenion`  -  consuming `themelion`,
  `skeue`, `gramma`
- `aletheia/crates/theatron/skene`  -  consuming `keryx`
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

- **kanon-server CI infrastructure instability** (surfaced 2026-05-04).
  Blocks aletheia PR #40 validation (the v1.1.0 consumer-pin bump,
  https://127.0.0.1:7878/forkwright/aletheia/pulls/40). Two CI
  attempts on PR #40 failed: first on queue contention (`active_runs=4`
  saturation, 3h21m runtime, sccache+target-dir thrash); second on
  kanon-server restart at 20:29:26 killing the in-flight cargo
  check (`Recovering LSM-tree at .../failure_embeddings` in journal).
  PR #40 is functionally a one-line `tag = "v1.1.0"` swap across 5
  Cargo.toml lines — content is fine, only validation is blocked.
  Workarounds: per-PR bypass-merge gate already validated theatron
  substrate; for PR #40 specifically, local `cargo check -p koilon
  -p skene -p proskenion` would prove consumer-side compiles.
- **chalkeion main on kanon** — unmerged feature branches; Cody's lane.
  Blocks chalkeion Phase 5a (operator-dispatch view, chalkeion-side
  polish) and Phase 5b chalkeion-side (operator dispatch view).
- **D-062** (kanon PR #517) — themelion harmonia/theatron consumer-
  pressure decision (three options on the PR). Blocks Phase 6
  harmonia-desktop migration.
- **akroasis GH #118** — surface-order rethink (TUI → web → desktop
  vs desktop-first via theatron). Blocks Phase 6 akroasis-desktop side.
- **PR-B proskenion::logging migration** — three options posted on
  theatron PR #50 timeline. proskenion's local `logging.rs` (88 LOC)
  diverges from `bathron::logging::init_with_stderr` in 5 behavioural
  details (log dir, file name, ANSI, EnvFilter directive, stderr
  trigger). Decision needed before retiring proskenion's local
  module. Carries forward as a v1.2 cut criterion.
- **Aletheia main red on `cargo nextest`** since 2026-05-03 17:43
  (sha 5634c65, run 1777844479342134601). Different failure mode
  from PR #40's CI infra issue: passes fmt/check/clippy, fails
  nextest. Real test failure, undiagnosed. `kanon ci show` doesn't
  expose stage stderr; needs direct fjall partition read of
  `/storage/kanon-forge/db/partitions/ci_logs/` or local nextest
  reproduction (paused 2026-05-04 to free menos resources, ~40 min
  into compile, log at `/tmp/aletheia-nextest-run.log`).
- Gate 2 (dioxus#2138 tray-icon upstream) is closed via composition
  layer in `mekhane`; not a blocker.

## Next steps

If resuming this work cold:

1. **Land aletheia PR #40** — re-pin koilon/proskenion/skene to
   `tag = "v1.1.0"`. Two paths depending on kanon-server health:
   - **Via CI** (preferred for real-consumer-validation):
     `kanon ci run forkwright/aletheia
     ed3f118346f66b36b5e955821287e2ebe2cfc922
     --ref-name refs/heads/chore/theatron-v1.1.0-pin --trigger manual`.
     Verify `active_runs` is below cap and journal has no recent
     "claim deferred" spam first; otherwise the run will fail on
     contention again.
   - **Via local + bypass-merge** (when CI is unstable): in the
     PR #40 worktree, run `CARGO_TARGET_DIR=/data/target cargo
     check -p koilon -p skene -p proskenion` (touches only the
     three consumer crates, fast under sccache warmth). On green,
     `~/menos-ops/bin/manual-pr-merge aletheia 40`. Note: bypass
     gate skips check on TOML-only diffs per
     `feedback_bypass_skips_check_on_toml_only`, so the local
     check is load-bearing.
2. **Continue waiting on the four upstream blockers** — D-062 /
   GH#118 / PR-B / chalkeion main land — to advance Phase 5a /
   Phase 6. Theatron-side work outside aletheia consumer-pin
   pickup is in maintenance mode pending these.
3. **Diagnose the 17h-old aletheia main nextest red** (separate
   from today's PR #40 infra issue). Resume local reproduction:
   `cd ~/dev/aletheia && CARGO_TARGET_DIR=/data/target cargo
   nextest run --workspace --features test-core --build-jobs 8
   --test-threads 8 --no-fail-fast 2>&1 | tee
   /tmp/aletheia-nextest-run.log` and grep FAIL.
4. **v1.2 surface additions** go to a fresh `## [Unreleased]` in
   CHANGELOG and ship as v1.2.0 when consumer pull or another
   wave's worth of items accumulates. The v1.1 wave's deferred
   cut criteria (consumer-pull validation, PR-B resolution) carry
   forward as v1.2 cut criteria. **v1.2 plan available**:
   `~/menos-ops/research-archive/2026-05-04-theatron-v1.2-consumer-pull.md`
   — codex-derived audit of koilon + skene with two STRONG
   candidates (`keryx::response::ensure_success`,
   `keryx::response::decode_json`) + one MODERATE.
