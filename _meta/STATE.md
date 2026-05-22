<!--
scope: current operational state for the theatron repository
defers_to: CLAUDE.md for repo conventions; _meta/ROADMAP.md for forward plan
tightens: release ledger, active blockers, and next-step handoff for the current theatron release line
-->

# State - theatron

## Current Phase

**v1.3.0 released 2026-05-22.** Additive minor release on top of
v1.2.0, tagged at `2f73ff1`, with release notes published on
GitHub. The release reconciled the `forge-archive/3c02dc4` mekhane
desktop surface onto `main` and included the already-shipped
dashboard lifecycle registry plus keryx URL/SSE helpers.

**Post-v1.3.0 maintenance wave active.** The next release accumulator
is v1.4.0 under `_meta/CHANGELOG.md` `## [Unreleased]`. Current
post-tag additions are consumer-pull helpers from issue #7 and remain
fully additive:

- `parodos::layout::{centered_rect_pct, centered_rect_size}`.
- `parodos::text::{truncate_chars_ellipsis, truncate_cols_ellipsis, truncate_spans_cols}`.
- `parodos::widgets::meter_string`.
- `skeue::badge::{BadgeColors, badge_style}`.
- `gramma::diff::parse_git_diff`.

No open GitHub issues or pull requests remain as of 2026-05-22 after
issue #7 was closed by PRs #8-#12 and SECURITY.md landed in PR #13.

Updated: 2026-05-22.

## Release Ledger

| Tag | Date | Scope |
|---|---|---|
| v1.0.0 | 2026-05-02 | First stable release; API frozen across all eight Greek-named crates per `_meta/SEMVER.md`. |
| v1.1.0 | 2026-05-04 | Additive minor bundling 31 PRs (#50-#82, sans #46-#49 docs and #80 superseded by #81): bathron logging/dialog/settings helpers, keryx ApiError accessors/predicates, parodos/theme helpers, gramma diff helpers, skeue components, and cargo-doc CI. |
| v1.2.0 | 2026-05-08 | Additive minor bundling keryx response/url helpers, gramma syntax helpers, themelion ThemeMode slug helpers, bathron logging knobs, and MSRV 1.86 alignment. |
| v1.3.0 | 2026-05-22 | mekhane muda app menus, global hotkeys, `default_icon`, examples/tests; dashboard lifecycle registry; keryx `SseStream::next_with_timeout` and `url::join_base_path`. |

## Recently Shipped

### v1.3.0 Release Cut

- **mekhane** - reconciled the forge-archive desktop surface for
  muda-backed app menus, global-hotkey event plumbing, and
  `tray::default_icon` PNG-bytes-to-Icon helper.
- **examples** - added `examples/tray_smoke` as a compile-time smoke
  for tray + menu + global-hotkey integration.
- **dashboards** - added and refined `_meta/DASHBOARD_LIFECYCLE.md`
  to keep dashboard ownership in consumer repositories.
- **keryx** - added `SseStream::next_with_timeout` and
  `url::join_base_path` from the v1.2 consumer-pull rescan.

### Post-v1.3.0 Maintenance

- **PR #8** - `parodos::layout` centered rect helpers.
- **PR #9** - `parodos::text` Unicode-safe char/column truncation
  helpers.
- **PR #10** - `parodos::widgets::meter_string`.
- **PR #11** - `skeue::badge` style shell helper.
- **PR #12** - `gramma::diff::parse_git_diff`.
- **PR #13** - root `SECURITY.md` plus GitHub security-policy pointer.

## Active Consumers

- **aletheia** - `koilon`, `proskenion`, and `skene` consume parodos,
  themelion, skeue, gramma, and keryx. Historical v1.1/v1.2
  validations remain the consumer-pull evidence for those surfaces;
  future v1.4 scope should be driven by a fresh rescan after the
  v1.3 pin is absorbed.
- **kanon/chalkeion** - canonical fleet desktop consumer for
  themelion, mekhane, skeue, gramma, keryx, and bathron. Chalkeion
  branch timing is owned by kanon; theatron keeps substrate additions
  additive and tag-pinnable.
- **harmonia / akroasis** - planned consumers, not yet active on
  theatron. Their adoption is gated by their own roadmaps, not by
  the current theatron release line.

## Locked Decisions

- **No upstream forks.** Dioxus + Blitz stay unmodified. OS hooks are
  layered through composition in mekhane.
- **Eight Greek-named crates, single workspace version.** The crate
  set is themelion, mekhane, skeue, gramma, keryx, bathron, parodos,
  and dokimasia.
- **API freeze at v1.0.** Public surface follows `_meta/SEMVER.md`;
  v1.x minors are additive and breaking changes wait for v2.
- **Consumer-pull discipline.** New public helpers land when real
  consumer pressure is documented.
- **Dioxus pinned `=0.7.6` workspace-wide.** Consumers must match the
  exact patch to avoid cross-crate `EventHandler<T>` type drift.
- **MSRV 1.86.** Workspace `rust-version` is 1.86 and
  `rust-toolchain.toml` uses nightly.
- **No browser surface.** Fleet desktop apps use Blitz native
  rendering, not wry/webview fallback.
- **Linux-first.** Linux remains the validated target. macOS and
  Windows are acknowledged by dependencies but are v1.4+ parity
  candidates, not present commitments. See `_meta/PLATFORM_COVERAGE.md`.

## Active Blockers

- **kanon-server / stoa infrastructure.** Local forge/audit tooling
  can contend with the running stoa writer lock. Use MCP audit
  surfaces where available, or rerun CLI audit commands when the lock
  is free.
- **Kimi MCP dispatch path.** As of the 2026-05-22 middle-manager
  wave, `mcp__kanon__dispatch_kimi` remained impaired for reviewer
  dispatches. Manual `kimi-ops` reviewer fallback was used for PRs
  #8-#13 and recorded in the middle-manager wrapper log.
- **Cross-repo consumer timing.** Chalkeion and aletheia pin bumps and
  rollout validation belong to their owning repositories; theatron
  should not perform cross-repo work directly.

## Next Steps

1. **Run the post-v1.3 consumer-pull rescan** against current
   aletheia and chalkeion code once those consumers have absorbed the
   v1.3 tag into normal feature work.
2. **Scope v1.4 from documented pull.** Likely candidates are skeue
   component expansion, mekhane platform parity, bathron platform
   parity, and keryx helpers surfaced by the next rescan.
3. **Keep standards metadata current.** `_llm/` corpus files and
   `_meta/*` planning files should move with release tags so audit
   tools do not see stale project state.

## Reference

- `_meta/CHANGELOG.md` - release notes archive and v1.4 accumulator.
- `_meta/ROADMAP.md` - forward plan.
- `_meta/SEMVER.md` - versioning policy.
- `_meta/RELEASE.md` - tag-cut process.
- `_meta/INTEGRATION.md` - consumer guide.
- `_meta/PLATFORM_COVERAGE.md` - OS-hook coverage matrix.
- `_meta/DASHBOARD_LIFECYCLE.md` - dashboard ownership registry.
