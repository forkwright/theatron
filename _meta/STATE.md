# State  -  theatron

## Current phase

**v1.0.0 released 2026-05-02.** API frozen across all eight Greek-named
crates (themelion, mekhane, skeue, gramma, keryx, bathron, parodos,
dokimasia) per `_meta/SEMVER.md`. Consumers pin via `tag = "v1.0.0"`.

Active fleet consumers at v1.0:
- `kanon/crates/chalkeion` -- ported through Phase 4 Tier 5
- `aletheia/crates/theatron/koilon` -- consuming `parodos` for theme,
  sanitize, clipboard, highlight, hyperlink (chalkeion plan W2)

Next: chalkeion Phase 5a polish + Phase 6 fleet rollout to
harmonia-desktop + akroasis-desktop.

Updated: 2026-05-02.

## Locked decisions

- Dual license: Apache-2.0 OR MIT
- 8 crates: themelion, mekhane, skeue, gramma, keryx, bathron, parodos, dokimasia
  (renamed from `theatron-{core,blitz,components,markdown,net,platform,tui,lint}`
  per fleet naming convention  -  see PR #21)
- Renderer: Dioxus + Blitz native (no wry webview fallback per chalkeion plan)
- Cross-platform: Linux-first; macOS/Windows out of scope through Phase 5
- a11y: keyboard nav required (AccessKit verified hookable)
- i18n: English-only in v1
- v1.0 cut at end of Phase 1+2 (chalkeion-tested API surface frozen).
  Versioning + release process: see `_meta/SEMVER.md`, `_meta/CHANGELOG.md`,
  `_meta/RELEASE.md`.

## Plan reference

See `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md` for the
full multi-month plan and Phase 0 progress capture.

## Active blockers

- None for Phase 1+2 work itself
- Gate 2 (dioxus#2138 tray-icon upstream PR) blocks chalkeion ship-ready
  but does not block theatron extraction work

## Next steps

See `~/dev/kanon/projects/chalkeion/STATE.md` for current next-steps
and phase tracking.
