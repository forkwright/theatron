# State — theatron

## Current phase

Phase 1+2 (kickoff). Repo created 2026-04-26, workspace + 8 crate
skeletons + minimal example + dual licensing.

Inaugural commit lands the structure; iterative extraction from
aletheia/proskenion follows in subsequent commits per the chalkeion
plan.

Updated: 2026-04-26.

## Locked decisions

- Dual license: Apache-2.0 OR MIT
- 8 crates: theatron-{core,platform,net,components,markdown,blitz,tui,lint}
- Renderer: Dioxus + Blitz native (no wry webview fallback per chalkeion plan)
- Cross-platform: Linux-first; macOS/Windows out of scope through Phase 5
- a11y: keyboard nav required (AccessKit verified hookable)
- i18n: English-only in v1
- v1.0 cut at end of Phase 1+2 (proskenion-tested API surface frozen)

## Plan reference

See `~/dev/kanon/projects/chalkeion/{vision,STATE,ROADMAP}.md` for the
full multi-month plan and Phase 0 progress capture.

## Active blockers

- None for Phase 1+2 work itself
- Gate 2 (dioxus#2138 tray-icon upstream PR) blocks chalkeion ship-ready
  but does not block theatron extraction work

## Next steps

1. Land inaugural commit (workspace + skeletons)
2. Port virtual_list.rs + table.rs + toast.rs from spike at
   /tmp/theatron-extract-spike/ as theatron-components first content
3. Begin proskenion refactor: theme.rs → theatron-core; toast/table/
   virtual_list → theatron-components
4. theatron-lint design — what tokens are valid, parser, fail mode
