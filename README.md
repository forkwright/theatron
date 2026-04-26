# theatron

*θέατρον — place for viewing*

Fleet desktop UI infrastructure for the forkwright/menos ecosystem.
Dioxus + Blitz primitives, theme provider, components (per kanon
DESIGN-TOKENS.md), HTTP/SSE, OS integration. Consumed by:

- aletheia/proskenion (Dioxus desktop chat)
- aletheia/koilon (Ratatui TUI)
- kanon/chalkeion (Dioxus desktop forge UI)
- harmonia/desktop (Dioxus desktop media platform)
- akroasis/desktop (Dioxus desktop RF intelligence)

## Status

Phase 0 complete (Blitz upstream gates: 3/4 resolved, 1 awaiting
operator authorization for upstream PR).

Phase 1+2 in flight: theatron extracted concurrently with
aletheia/proskenion refactor; theatron v1.0 cut at exit.

Plan: [`kanon/projects/chalkeion/`](http://forge.forkwright.com/forkwright/kanon/tree/main/projects/chalkeion).

## Crates

| Crate | Role |
|---|---|
| `theatron-core` | Window lifecycle, theme provider, routing scaffolding, error boundary, settings, logging |
| `theatron-platform` | OS integration: tray icon, hotkeys, menus, notifications, file dialogs, window state |
| `theatron-net` | HTTP client base, SSE/streaming via reqwest+tokio+eventsource-stream, mDNS discovery |
| `theatron-components` | Generic Dioxus components per DESIGN-TOKENS.md component anatomy |
| `theatron-markdown` | pulldown-cmark + syntect, sandbox-safe HTML output |
| `theatron-blitz` | Dioxus + Blitz integration helpers |
| `theatron-tui` | Ratatui shared primitives + Elm dispatcher |
| `theatron-lint` | Design-token enforcement linter |

## License

Apache-2.0 OR MIT.
