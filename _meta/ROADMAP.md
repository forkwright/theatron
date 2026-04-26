# Roadmap — theatron

This is the implementation roadmap for the theatron repo. The broader
fleet plan lives in `~/dev/kanon/projects/chalkeion/ROADMAP.md`.

## Phase 1+2 (current, 5-7 weeks)

Build theatron crates concurrently with porting aletheia/proskenion +
koilon to consume them.

1. **Workspace skeleton** ✅ (this commit)
2. **theatron-components seed** — port virtual_list, table, toast from
   spike at `/tmp/theatron-extract-spike/` (1 day)
3. **theatron-core theme provider** — extract from
   proskenion/src/theme.rs; ThemeMode enum, signal binding, OS pref
   detection (~1 week)
4. **theatron-platform** — window state persistence + tray icon
   (blocked on Gate 2) + global hotkeys + native notifications + file
   dialogs (1-2 weeks)
5. **theatron-net** — HTTP client base + SSE pattern (per Phase 0
   Gate 3 reference at `/tmp/sse-spike/`) + mDNS discovery (~1 week)
6. **theatron-markdown** — pulldown-cmark + syntect wrapper, derived
   from kanon/crates/stoa/src/{markdown,highlight,escape}.rs (~3 days)
7. **theatron-tui** — extract Elm dispatcher + Markdown renderer +
   editor + utilities from aletheia/koilon (~1 week)
8. **theatron-blitz** — Dioxus + Blitz integration helpers (small,
   mostly version pins + glue)
9. **theatron-lint** — CSS parser + Rust string-literal scanner +
   DESIGN-TOKENS.md crossref. Fails CI on undocumented tokens. (~1 week
   parallel)
10. **proskenion refactor** — port to consume theatron, validate API
    against real consumer pressure
11. **koilon refactor** — port to consume theatron-tui
12. **theatron v1.0 cut** — API frozen; breaking changes require minor
    bump + migration guide

## Phase 6 (post-v1.0, ongoing)

- harmonia/desktop builds on theatron when their Phase 3.5 W4 starts
- akroasis/desktop builds on theatron when desktop tertiary surface begins
- Future fleet surfaces consume theatron by default

## SemVer policy

- v0.x.y during Phase 1+2 — breaking changes free
- v1.0.0 at Phase 1+2 exit — frozen API
- Additive changes → minor version bump (v1.x.y)
- Breaking changes → major version bump + migration guide
- Theatron-lint enforces token vocabulary; new tokens require
  DESIGN-TOKENS.md PR + theatron-components release

## Blockers

- **Gate 2** (dioxus#2138 tray-icon) — blocks theatron-platform tray
  capability, but theatron-platform can ship without tray initially
  and add it once Gate 2 resolves
