# Roadmap  -  theatron

This is the implementation roadmap for the theatron repo. The broader
fleet plan lives in `~/dev/kanon/projects/chalkeion/ROADMAP.md`.

## Phase 1+2  -  shipped

Built theatron's eight crates concurrently with porting
aletheia/proskenion + koilon to consume them. All deliverables landed
on or before the v1.0.0 cut.

1. **Workspace skeleton** ✅
2. **`skeue` seed** ✅  -  virtual_list, table, toast ported from
   proskenion
3. **`themelion` theme provider** ✅  -  ThemeMode + signal binding +
   OS pref detection
4. **`bathron` OS services** ✅  -  notifications, dialogs, settings,
   logging behind per-feature gates
5. **`mekhane` tray + menus + hotkeys** ✅  -  Gate 2 closed via
   composition over unmodified Dioxus + Blitz (no fork). v2 added
   `launch_cfg_with_props_ext`, app menus, global hotkeys,
   `default_icon`
6. **`keryx` net** ✅  -  HTTP client base + SSE pattern + ApiError
7. **`gramma` markdown** ✅  -  pulldown-cmark + syntect wrapper,
   diff-state types
8. **`parodos` TUI** ✅  -  theme, sanitize, clipboard, highlight,
   hyperlink, fuzzy, env (extracted from aletheia/koilon)
9. **`dokimasia` linter** ✅  -  design-token + standards rules,
   namespace frozen at v1.0
10. **proskenion refactor** ✅  -  consuming theatron in production
11. **koilon refactor** ✅  -  consuming `parodos` re-exports
12. **theatron v1.0 cut** ✅  -  released 2026-05-02, API frozen per
    `_meta/SEMVER.md`

## Phase 5  -  ship-ready polish + distribution

Post-v1.0 polish before broad fleet rollout. Two parallel tracks:

### 5a  -  chalkeion-side polish

Owned by the chalkeion crate (kanon repo). Real iconography, perf
budgets, a11y audit. Tracked on the chalkeion ROADMAP, not here.

### 5b  -  theatron-side distribution

Theatron-side deliverables:

- ✅ Quickstart in README pointing at the eight-crate Cargo pin
- ✅ `_meta/INTEGRATION.md`  -  consumer guide covering theme provider
  setup, mekhane launch variants, bathron feature flags, parodos
  reuse, keryx HTTP/SSE patterns, dokimasia adoption
- ✅ Per-example READMEs (`examples/minimal`, `examples/tray_smoke`)
- ✅ ratatui 0.30 + crossterm 0.29 alignment with downstream
- ⏳ Operator dispatch view (chalkeion-side; tracked there)

## Phase 6  -  fleet rollout

Post-v1.0, ongoing. Each fleet desktop surface migrates onto theatron
on its own cadence; theatron itself owes nothing here beyond keeping
the v1.0 API contract.

- harmonia/desktop builds on theatron when their Phase 3.5 W4 starts
- akroasis/desktop builds on theatron when desktop tertiary surface begins
- chalkeion already at parity through Phase 4 Tier 5
- Future fleet surfaces consume theatron by default

## SemVer policy

See [`_meta/SEMVER.md`](./SEMVER.md) for the full rules. Summary:

- v1.0.0+  -  additive minor bumps, breaking change requires major
- Workspace `version` drives all eight crates in lockstep
- `dokimasia` enforces design-token vocabulary; new tokens require a
  DESIGN-TOKENS.md PR + theatron minor bump

## Blockers

None at v1.0. Gate 2 (tray support) closed via composition layer in
`mekhane` over unmodified upstream Dioxus + Blitz.
