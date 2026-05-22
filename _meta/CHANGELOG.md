# Changelog — theatron

This file is the authoritative release-note record for theatron's
eight-crate set. Entries follow [Keep a Changelog](https://keepachangelog.com)
formatting and theatron's [SemVer policy](./SEMVER.md).

The eight crates ship together at a single workspace version; one
entry per release covers all eight.

## [Unreleased]

The next entry here flows into the next minor (v1.4) when demand
pulls one.

### Added

- **`gramma::diff::parse_git_diff(raw) -> Vec<DiffFile>`** —
  multi-file git diff parser that splits file sections, derives paths
  from git/unified headers, and reuses the existing `DiffFile` model and
  `parse_unified_diff` hunk logic. Surfaced as issue #7 candidate 5
  from the 2026-05-09 consumer-pull rescan. 5 tests cover multi-file
  diffs, deleted-file `/dev/null` paths, single-file unified input,
  binary/metadata-only sections, and malformed input.
- **`parodos::text::{truncate_chars_ellipsis, truncate_cols_ellipsis, truncate_spans_cols}`**
  — Unicode-safe terminal truncation helpers with explicit character
  and display-column contracts. Ellipses are included inside the
  caller's budget, zero-width budgets return empty output, and styled
  span truncation preserves retained styles. Surfaced as issue #7
  candidate 1 from the 2026-05-09 consumer-pull rescan. 12 new tests
  cover no-op boundaries, ellipsis budgeting, multibyte char
  boundaries, wide display columns, styled spans, and zero/one-column
  budgets.
- **`parodos::widgets::meter_string(pct, width, filled, empty) -> String`**
  — fixed-width terminal meter helper for repeated filled/empty glyph
  gauges. Percent values above 100 clamp to a full meter, partial
  cells use integer flooring, and zero width returns an empty string.
  Surfaced as issue #7 candidate 3 from the 2026-05-09 consumer-pull
  rescan. 6 tests cover zero width, zero/full percentages, over-100
  clamping, partial-cell flooring, and custom glyphs.
- **`skeue::badge::{BadgeColors, badge_style}`** — shared CSS shell
  for compact text badges. The helper owns only the common spacing,
  radius, type, foreground, and background style string; domain labels
  and status mappings remain in consumers. Surfaced as issue #7
  candidate 4 from the 2026-05-09 consumer-pull rescan. 5 tests cover
  constructor/copy/equality semantics, shared shell tokens, supplied
  colors, style delegation, and color variation.
- **`parodos::layout::{centered_rect_pct, centered_rect_size}`** —
  ratatui `Rect` centering helpers for percentage-sized and fixed-size
  overlays. Percent values above 100 clamp to the full source area,
  requested fixed sizes clamp to the source area, and odd leftover
  space stays on the trailing side. Surfaced as issue #7 candidate 2
  from the 2026-05-09 consumer-pull rescan. 8 new tests cover normal
  percentage sizing, over-100 clamping, zero-sized axes, exact-size
  preservation, oversized fixed-size clamping, odd leftover centering,
  nonzero origins, and large-rectangle overflow resistance.
- **`keryx::url::join_base_path(base_url, path) -> String`** —
  slash-normalizing string join for endpoint construction. Strips
  trailing `/` from `base_url` and leading `/` from `path`, then
  joins with exactly one `/` between them. Either side may be
  empty. Replaces 5+ hand-rolled `format!("{}/{}", base.trim_end_matches('/'), path)`
  patterns at `aletheia/crates/theatron/skene/src/{api/client,api/sse,api/streaming,discovery}.rs`.
  Surfaced as MODERATE candidate #1 in the 2026-05-09 consumer-pull
  rescan (theatron forge issue #1). 8 new tests covering canonical
  collapse, missing-separator insertion, both-correct boundaries,
  multiple trailing slashes, empty base, empty path, both empty,
  and internal-slash preservation.
- **`keryx::sse::SseStream::next_with_timeout(deadline) -> Result<Option<SseEvent>, Elapsed>`**
  — async deadline wrapper around `StreamExt::next` for keep-alive /
  liveness detection on stalled SSE feeds. Returns `Ok(Some)` on
  event, `Ok(None)` on clean stream termination, `Err(Elapsed)` on
  deadline-fire. Stream stays usable after Elapsed (consumer can
  retry with a longer deadline or fall back to plain `next()`).
  Replaces the duplicate `tokio::time::timeout(d, es.next())` loop
  at `aletheia/crates/theatron/skene/src/api/sse.rs:89,92` and
  `streaming.rs:92,95`. Surfaced as MODERATE candidate #2 in the
  2026-05-09 rescan (theatron forge issue #1; closes that issue's
  second half). 4 new tests covering: event-in-time, clean
  termination, stalled-stream Elapsed, post-Elapsed re-pollability.

---

## v1.2.0 — 2026-05-08

Additive minor release bundling 5 helpers across keryx + gramma +
themelion + bathron, the first wave of consumer-pull surface from the
2026-05-04 + 2026-05-09 audits + the PR-B (Option 3) substrate that
makes the pending proskenion logging migration behavior-preserving.
**Fully additive, no breaking changes.** Consumers re-pin
`tag = "v1.1.0"` → `tag = "v1.2.0"` at their own pace; no migration
required.

Cut criteria status (vs v1.1's deferred list):
- Criterion #1 — consumer-pull validation — **satisfied** by aletheia
  PR #40 land (squash 0b3cdd8c) re-pinning koilon / proskenion /
  skene to v1.1.0 + this v1.2.0 substrate compiling cleanly against
  current consumers via local `cargo check`.
- Criterion #2 — PR-B logging-migration completion — **partially
  satisfied** at the substrate level (the bathron knobs are landed
  here); the proskenion-side migration ships next as PR-B.2 against
  this tag.

Workspace MSRV bumped from `1.85` → `1.86` (PR #91 chore commit) to
reflect the actual transitive minimum imposed by `vello_shaders 0.6`
in the dioxus-native / blitz / vello chain. `rust-toolchain.toml`
channel set to `nightly` (the pre-PR-#89 implicit default).

### Added (since v1.1.0)

- **`keryx::response` module — `ensure_success` + `decode_json` helpers**
  (consumer-pull, ranks #1 + #2 STRONG in
  `~/menos-ops/research-archive/2026-05-04-theatron-v1.2-consumer-pull.md`).
  Two async helpers that make the v1.1 `ApiError` variants reachable
  from `reqwest::Response` without per-consumer status-table
  boilerplate:
    - `ensure_success(response, operation) -> Result<Response>` — 2xx
      passthrough; 401/403 → `Auth`; 429 → `RateLimited` (with
      `Retry-After` parsed when delta-seconds); other non-2xx →
      `Server` with `message` extracted from JSON `message`/`error`
      fields, falling back to `"<status> <reason>"`.
    - `decode_json::<T>(response, operation) -> Result<T>` — body
      read → `Http` on transport failure; `serde_json::from_str` →
      `BadResponse` on parse failure. Use after `ensure_success` for
      typed DTO extraction from validated 2xx bodies.
  Replaces the hand-rolled `check_status` / `check_auth` /
  `resp.json().await.context(HttpSnafu)` patterns at ≥ 19 sites in
  `aletheia/crates/theatron/skene/src/api/{client,sse,streaming}.rs`
  per the audit. 16 new unit tests covering 2xx / 401 / 403 / 429
  with + without `Retry-After` / 5xx with `message` field / 5xx
  with `error` field / 5xx with non-JSON body / valid + malformed
  JSON decode.
- **`keryx::url::encode_path_segment(segment: &str) -> String`** —
  RFC 3986 percent-encoding for URL path segments. Unreserved
  characters (`A`-`Z`, `a`-`z`, `0`-`9`, `-`, `.`, `_`, `~`) pass
  through unchanged; everything else becomes `%XX` uppercase hex.
  Replaces `skene`'s local `encode_path` (~24 endpoint-builder call
  sites at `aletheia/crates/theatron/skene/src/api/client.rs`) and
  consolidates the implementation as keryx substrate. Surfaced as
  STRONG candidate #1 in the 2026-05-09 consumer-pull rescan
  (`~/menos-ops/research-archive/2026-05-09-theatron-v1.2-consumer-pull-rescan.md`).
- **`gramma::syntax::language_from_path(path: &str) -> &'static str`**
  and **`gramma::syntax::language_from_extension(ext: &str) -> &'static str`** —
  file-path-to-syntect-language resolution covering 30+ extensions
  (rust, python, ts/tsx/js/jsx as distinct tokens, c/cpp groupings,
  bash/sh/zsh/fish, yaml/yml, html/htm, markdown, etc.). Returns
  `"text"` for unknown extensions (the syntect plain-text fallback).
  Companion to the existing `highlight::detect_language` (which
  parses Markdown fenced-code-block info strings). Replaces
  `proskenion`'s two divergent extension maps at
  `aletheia/crates/theatron/proskenion/src/state/files.rs:140` and
  `views/files/diff.rs:190`. STRONG candidate #2 in the rescan.
- **`themelion::ThemeMode::slug(self) -> &'static str`** and
  **`themelion::ThemeMode::from_slug(s: &str) -> Option<Self>`** —
  lowercase storage-slug round-trip pair (`"dark"`, `"light"`,
  `"system"`). Distinct surface from existing `from_label`/`label`
  (which produces `"Dark"`/`"Light"`/`"System"` for UI display);
  `from_slug` is intentionally case-sensitive for config-file
  parsing. Replaces hand-rolled lowercase parsing at 4 proskenion
  call sites (`app.rs`, `views/settings/wizard.rs`,
  `views/settings/appearance.rs`, `components/theme_toggle.rs`).
  STRONG candidate #3 in the rescan.
- **`bathron::logging::LogConfig::with_ansi_on_file(bool)`** and
  **`bathron::logging::LogConfig::with_filter_directive(impl Into<String>)`** —
  PR-B Option 3 substrate, the two builder knobs that make the
  pending proskenion logging migration behavior-preserving.
  `with_ansi_on_file` toggles ANSI escape sequences on the rotated
  file appender (defaults to `true`; set `false` for journal /
  tail-grep pipelines that mis-render SGR). `with_filter_directive`
  sets an `EnvFilter`-compatible directive string used as the
  fallback filter when `RUST_LOG` is unset, supporting per-namespace
  filters (e.g. `"proskenion=info"`) instead of a single global
  level. `LogConfig` gains two new public fields (`ansi_on_file`,
  `filter_directive`); `init`/`init_with_stderr` honor both. 6 new
  tests covering defaults, builder chains, str+String acceptance,
  and Debug-rendering of the new fields.

### Changed

- **Workspace MSRV** bumped from `1.85` → `1.86` to reflect the
  actual transitive minimum imposed by `vello_shaders 0.6` in the
  dioxus-native / blitz / vello chain. **`rust-toolchain.toml`**
  channel set to `nightly` (theatron's pre-#89 implicit default —
  threads both the 1.86 floor and lint regressions on stable).

---

## v1.1.0 — 2026-05-04

Additive minor release accumulated 2026-05-03 → 2026-05-04 while
waiting on chalkeion landing. **31 fully additive PRs, no breaking
changes.** Consumers re-pin `tag = "v1.0.0"` → `tag = "v1.1.0"` at
their own pace; no migration required.

The `bathron::logging::init_with_stderr` addition (PR #50) is the
first genuinely new public API since v1.0.0 and flipped the planned
patch (`v1.0.1`) to a minor (`v1.1.0`). The wave kept that minor
slot open through 28 more additive items before the cut.

### Added (since v1.0.0)

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
- **`keryx::ApiError::is_client_error()` + `::is_server_error()`**
  (PR #77). HTTP-class predicates that complete the trio with
  `is_retryable`. `is_client_error()` returns `true` for
  `Server` with a 4xx status (and `RateLimited`, always 429);
  `is_server_error()` returns `true` for `Server` with a 5xx
  status. Both return `false` for transport / payload /
  pre-flight variants (those didn't receive a response in the
  named class by definition). `Auth` returns `false` from both
  because the variant erases the specific 401-vs-403 status.
  +5 tests covering 4xx range / 5xx range / `RateLimited` is
  client-not-server / response-less-variants are neither /
  boundary partition (399, 400, 499, 500, 599, 600).
- **`keryx::ApiError::is_retryable() -> bool`** (PR #59).
  Predicate for retry layers. Returns `true` for transient
  failures (`Timeout`, `RateLimited`, 5xx `Server`, connect /
  timeout `Http`), `false` for terminal failures (4xx `Server`,
  `BadResponse`, `Auth`, `InvalidToken`, non-connect `Http`).
  Conservative default — consumers wanting more aggressive
  retries (e.g. on 4xx for idempotent reads) make their own
  judgment. +5 tests covering each branch; keryx tests:
  33 → 38.
- **`keryx::ApiError::retry_after() -> Option<u64>`** (PR #71).
  Accessor for the `Retry-After` delta-seconds value carried by
  `RateLimited`. Returns `Some(secs)` when the variant is
  `RateLimited` and the server supplied a `Retry-After` header
  in delta-seconds form (per RFC 9110 § 10.2.3); `None`
  everywhere else (other variants, or `RateLimited` without a
  parseable header). Lets retry layers honour server backoff
  hints without manually destructuring. +3 tests covering the
  `Some` branch, `None`-on-`RateLimited` branch, and every other
  variant; keryx tests: 19 → 22 in error.rs.
- **`bathron::settings::Settings::remove(key) -> Result<bool, …>`**
  (PR #74). Symmetric with `set`; rounds out the CRUD surface
  (`get` / `contains` / `keys` / `set` / `remove`). Returns
  `Ok(true)` if the key was present and removed, `Ok(false)`
  if already absent — idempotent, removing an absent key is not
  an error. Atomic via tempfile + rename like `set`. Skips the
  write entirely when the key is absent (no I/O cost, no mtime
  bump, no settings file created if it didn't already exist).
  +4 tests covering existing-key removal / missing-key returns
  false / idempotent re-remove / scoped-to-named-key /
  no-write-when-absent. bathron settings tests: 34 → 38.
- **`bathron::settings::Settings::keys() -> Result<Vec<String>, …>`**
  (PR #73). Enumerates every top-level key currently present in
  the on-disk settings file, in TOML document order. Useful for
  migration code (drop or rename keys whose schema changed),
  debug UIs, and consumer-side validation that warns about
  unrecognised keys. Returns an empty vector when the file is
  missing or empty (symmetric with `get` returning `None`).
  Only enumerates **top-level** keys; nested `[ui]`-style tables
  appear as a single key whose value is the table. Cannot
  return `DeserializeValue` since no value deserialization
  happens. +4 tests covering missing-file / multi-key
  enumeration / `keys`↔`contains` round-trip / values-don't-leak.
  bathron settings tests: 30 → 34.
- **`bathron::settings::Settings::contains(key) -> Result<bool, …>`**
  (PR #60). Cheaper presence check than `get::<T>(key)` when the
  consumer only needs to know whether a key is set (e.g. "has the
  user configured a theme yet?"). Skips the `DeserializeOwned`
  cost; reports presence regardless of value type. Cannot return
  `DeserializeValue` since no deserialization happens. +5 tests
  covering existing key / missing key / missing file / type-
  coercion-free presence / idempotent re-set. bathron tests:
  53 → 58.
- **`themelion::ThemeMode::is_dark` + `::is_light` + `::is_system`**
  (PR #76). Three `const fn` predicates on the desktop-side
  `ThemeMode` (the 3-variant enum). These are `user preference`
  predicates — to ask whether the *rendered* theme is dark, call
  `mode.resolve().is_dark()` (which goes through the existing
  `ResolvedTheme` predicates from PR #61). `is_system()` is the
  new piece — tells consumers when `resolve()` would consult the
  desktop-environment preference vs. return a forced value.
  Symmetric with `parodos::ThemeMode::is_dark` + `::is_light`
  (PR #70; parodos has no `System` because terminals don't have
  an OS preference to resolve). +4 tests covering each branch
  + the exhaustive-partition compile-time check. themelion
  tests grow accordingly.
- **`themelion::ResolvedTheme::parse_data_attr` + `::all`**
  (PR #81). Round-trip with `as_str` (the canonical
  `[data-theme="…"]` attribute value applied by `ThemeProvider`).
  `parse_data_attr("dark" | "light")` is case-sensitive (matches
  the lowercase attribute the DOM actually carries) and returns
  `None` for any other input. Name is deliberately specific —
  `from_str` would clash with `std::str::FromStr` conventions,
  and the function only accepts the canonical attribute value,
  not arbitrary strings. `all()` returns the fixed-size
  `[Dark, Light]` array (no `System` since `ResolvedTheme` is
  the post-resolve enum). Useful for tests reading the attribute
  back off the DOM and for any consumer iterating every
  resolved value. Symmetric with `ThemeMode::from_label` +
  `::all` (PR #57). +5 tests: canonical, case-sensitive,
  unrecognized input, `parse_data_attr↔as_str` round-trip,
  and every-variant exhaustive.
- **`themelion::ResolvedTheme::is_dark`** + **`::is_light`**
  (PR #61). Two convenience predicates on `ResolvedTheme`.
  `theme.is_dark()` reads better than `theme == ResolvedTheme::Dark`
  at consumer call sites. Both are `const fn`, so usable in
  const contexts. +3 tests covering each branch + the
  mutually-exclusive partition. themelion tests: 19 → 22.
- **`parodos::theme::ThemeMode::from_label` + `::all`** (PR #72).
  Two helpers symmetric with `themelion::ThemeMode::from_label` /
  `::all` (PR #57). `from_label("dark" | "light")` is
  case-insensitive and returns `None` for any other input,
  including `"system"` — parodos runs in a terminal where there's
  no OS-level light/dark preference to resolve, so the parodos
  enum has no `System` variant. `all()` returns the
  fixed-size `[Dark, Light]` array (vs. themelion's three-element
  array). +5 tests covering canonical / case-insensitive /
  unrecognized input + every-variant + round-trip; parodos
  theme tests: 29 → 34.
- **`parodos::theme::ThemeMode::is_dark` + `::is_light`** (PR #70).
  Two `const fn` predicates on parodos's local `ThemeMode`
  (distinct from `themelion::ThemeMode` which has 3 variants).
  Mirrors `themelion::ResolvedTheme::is_dark` (PR #61) and the
  `ColorDepth` predicate pattern (PR #69). +3 tests covering
  each branch + mutual-exclusivity. parodos tests: 177 → 180.
- **`parodos::theme::ColorDepth` predicates** (PR #69). Four
  convenience `const fn` predicates: `is_truecolor`, `is_256`,
  `is_basic`, plus `has_256` (true for `TrueColor` or
  `Color256`). Matches the predicate pattern from `ChangeType`,
  `DiffViewMode`, `ResolvedTheme`. Useful for "use a richer
  palette if available" branches in TUI render code. +5 tests
  covering each predicate + the exhaustive-partition compile-
  time check. parodos tests: 172 → 177.
- **`gramma::diff::DiffViewMode::is_unified` + `::is_side_by_side`**
  (PR #68). Two convenience `const fn` predicates matching the
  `ChangeType` (PR #67) / `ResolvedTheme` (PR #61) pattern.
  Pre-existing `toggle()` already shipped at v1.0; the predicates
  complete the API. +3 tests covering each branch + mutual-
  exclusivity. gramma tests: 56 → 59.
- **`gramma::diff::ChangeType::glyph() -> char`** (PR #79).
  Returns the canonical unified-diff prefix character for the
  variant: `'+'` for `Add`, `'-'` for `Remove`, `' '` (space)
  for `Context`. Matches the prefix bytes used by every
  unified-diff renderer (git, patch(1), GNU diff -u). `const
  fn`. Useful for consumer code rendering a line gutter without
  per-variant matching. +3 tests: canonical mapping, predicate
  round-trip, glyph uniqueness.
- **`gramma::diff::ChangeType` predicates** (PR #67). Four
  convenience `const fn` predicates: `is_add`, `is_remove`,
  `is_context`, `is_change` (the negation of `is_context`).
  Matching pattern from `themelion::ResolvedTheme::is_dark`
  (PR #61). Useful for filtering / counting / styling diff lines
  without per-variant matching. +5 tests covering each predicate
  + the exhaustive-partition compile-time check. gramma tests:
  51 → 56.
- **`skeue::ErrorState` component** (PR #66). Sibling to
  `EmptyState` (PR #52) and `Spinner` (PR #55) -- together the
  three cover the asynchronous-view triad: loading / no-data /
  error. Slots: `title` (required, accessible name), optional
  `message` / `icon` / `action`. Marked `role=alert` with
  `aria-live=assertive` (distinct from `EmptyState`'s `role=status`
  + `aria-live=polite`). Defaults to a warning glyph when no icon
  is provided. +6 SSR tests covering title-only, default icon
  fallback, custom icon override, message rendering, message-
  omitted-when-None, action slot wiring. skeue inventory:
  14 -> 15 components.
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
- **`gramma::diff::DiffFile::stats() -> DiffStats`** (PR #82).
  Single-file aggregate convenience: returns a `DiffStats` with
  `files_changed = 1` and the file's `additions` /
  `deletions`. Flattens the `DiffStats::from_files(&[diff_file])`
  call site for consumers that already have a `DiffFile` in
  hand (e.g. per-file rows in a tree view). `const fn`. +3
  tests: returns the right values for a populated file, matches
  `DiffStats::from_files(slice::from_ref(&file))`, and a
  zero-change file still counts as 1 file changed.
- **`gramma::diff::DiffStats` shape helpers** (PR #75). Three new
  `const fn` accessors on the v1.1 `DiffStats` aggregate:
    - `net_change() -> i64` — signed `additions - deletions`;
      returns `i64` so the difference of two `u32`s fits even at
      the saturated bounds. Useful for at-a-glance PR sizing
      (a refactor netting near zero looks very different from a
      feature add netting hundreds of lines).
    - `is_pure_addition() -> bool` — true when `deletions == 0`
      (every line change is an addition). Vacuously true for
      empty stats.
    - `is_pure_deletion() -> bool` — true when `additions == 0`.
      Vacuously true for empty stats.

  All three are `const fn`. +6 tests covering signed-grew /
  signed-shrank / signed-balanced / saturated-bounds-without-
  overflow / pure-addition / pure-deletion / empty-is-vacuously-
  both. gramma tests grow accordingly.
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
