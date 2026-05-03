# theatron

*θέατρον  -  place for viewing*

Fleet desktop UI infrastructure for the forkwright/menos ecosystem.
Dioxus + Blitz primitives, theme provider, components (per kanon
DESIGN-TOKENS.md), HTTP/SSE, OS integration. Consumed by:

- aletheia/proskenion (Dioxus desktop chat)
- aletheia/koilon (Ratatui TUI)
- kanon/chalkeion (Dioxus desktop forge UI)
- harmonia/desktop (Dioxus desktop media platform)
- akroasis/desktop (Dioxus desktop RF intelligence)

## Status

**v1.0.0 released 2026-05-02.** Public API frozen across all eight
crates per [`_meta/SEMVER.md`](./_meta/SEMVER.md). Consumers pin a
single git tag and inherit additive minors / non-breaking patches
until v2.0.

Full plan: [`kanon/projects/chalkeion/`](http://forge.forkwright.com/forkwright/kanon/tree/main/projects/chalkeion).

## Quickstart

Add to your consumer `Cargo.toml`:

```toml
[dependencies]
themelion = { git = "http://forge.forkwright.com/forkwright/theatron.git", tag = "v1.0.0" }
mekhane  = { git = "http://forge.forkwright.com/forkwright/theatron.git", tag = "v1.0.0" }
dioxus   = "=0.7.6"
dioxus-native = "=0.7.6"
```

Smallest working app:

```rust
use dioxus::prelude::*;
use dioxus_native::launch;

fn app() -> Element {
    rsx! { div { "theatron " {themelion::version()} } }
}

fn main() { launch(app); }
```

See [`examples/minimal/`](./examples/minimal) for a runnable version
with the kanon dye-token CSS variables wired in, and
[`examples/tray_smoke/`](./examples/tray_smoke) for the tray + menu
+ global-hotkey surface.

Consumer integration patterns (Cargo features, theme provider, OS
services, mekhane launch variants): see
[`_meta/INTEGRATION.md`](./_meta/INTEGRATION.md).

## Crates

Standalone Greek names per the fleet naming convention (no `theatron-`
prefix; each crate is its own domain abstraction).

| Crate | Greek | Role |
|---|---|---|
| `themelion` | θεμέλιον  -  foundation | Theme provider, window lifecycle, routing scaffolding, error boundary, settings, logging |
| `mekhane` | μηχανή  -  stage machinery | Windowing, event loop, OS hooks (tray, hotkeys, native menus). Wraps unmodified Dioxus + Blitz. |
| `bathron` | βάθρον  -  pedestal | OS services: notifications, file dialogs, window state, settings, autoupdate, logging |
| `keryx` | κῆρυξ  -  herald/messenger | HTTP client base, SSE streaming, mDNS discovery, ApiError |
| `skeue` | σκευή  -  props/equipment | Generic Dioxus components per DESIGN-TOKENS.md component anatomy |
| `gramma` | γράμμα  -  written character | Markdown + syntax highlighting (pulldown-cmark + syntect), diff state |
| `parodos` | πάροδος  -  chorus's stage entrance | Terminal UI substrate (Ratatui + theme + sanitize + clipboard + highlight + hyperlink) |
| `dokimasia` | δοκιμασία  -  examination | Design-token + standards enforcement linter |

## Architecture

```
┌─ App (chalkeion, proskenion, harmonia-desktop, akroasis-desktop) ─┐
│  parameterized via configs                                         │
└────────────────────────────────────────────────────────────────────┘
                                    ↓ depends on
┌─ theatron crates (themelion, mekhane, skeue, gramma, …) ───────────┐
│  fleet-owned, kanon-style domain crates                             │
└────────────────────────────────────────────────────────────────────┘
                                    ↓ depends on
┌─ Dioxus (UI framework) ─┐  ┌─ Blitz (render engine) ─┐  ┌─ winit/Vello/wgpu/accesskit ─┐
│  upstream customer       │  │  upstream → dioptron     │  │  upstream foundational         │
└─────────────────────────┘  └──────────────────────────┘  └────────────────────────────────┘
```

Dioxus is a customer of theatron  -  we use it unmodified, never patch.
Blitz today is upstream; long-term `dioptron` (the fleet's sovereign
web runtime) owns the same render-band primitives directly, and
`mekhane` consumes dioptron once that lands.

## License

Apache-2.0 OR MIT.
