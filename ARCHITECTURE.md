# Architecture

Theatron is the fleet desktop UI substrate for forkwright. It ships
eight Rust crates from one workspace and one version line. Consumers
pin a single theatron tag, then select only the crates they use.

## Layers

Applications such as chalkeion, proskenion, koilon, harmonia, and
akroasis own product state and domain flows. Theatron owns reusable
view infrastructure below them.

```
application surface
        |
theatron crates
        |
Dioxus, Blitz, Ratatui, winit, Vello, wgpu, AccessKit
```

The project wraps upstream Dioxus and Blitz through composition. It
does not fork either project. Desktop OS hooks live in `mekhane`.
Terminal UI helpers live in `parodos`.

## Crate Roles

| Crate | Role |
|---|---|
| `themelion` | Theme provider, window lifecycle helpers, routing scaffolding, error boundary, settings, and logging hooks. |
| `mekhane` | Window launch, tray, native menu, and global-hotkey integration over Dioxus native. |
| `skeue` | Reusable Dioxus components that follow the fleet design-token vocabulary. |
| `gramma` | Markdown, syntax highlighting, and diff data structures. |
| `keryx` | HTTP, SSE, URL, and API error helpers. |
| `bathron` | OS services for notifications, dialogs, settings, updates, and logging. |
| `parodos` | Ratatui substrate for theme, sanitize, clipboard, highlight, hyperlink, fuzzy search, layout, text, and widgets. |
| `dokimasia` | Design-token and standards linting. |

## Public Surface

The public API is every `pub` item reachable from a crate root and
every Cargo feature exposed by the workspace crates. The SemVer rules
are documented in [`kanon/projects/theatron/SEMVER.md`](http://forge.forkwright.com/forkwright/kanon/tree/main/projects/theatron/SEMVER.md).

The eight crates move in lockstep. Adding public API requires a minor
release. Removing or breaking public API requires a major release.
Docs, tests, internals, and bug fixes stay patch-compatible.

## Consumer Boundary

Theatron does not own consumer DTOs, application stores, release
cadence, or dashboard lifecycle state. Those stay in the consuming
repositories.

Consumers should start from [`_meta/INTEGRATION.md`](./_meta/INTEGRATION.md).
Platform support evidence lives in
[`kanon/projects/theatron/PLATFORM_COVERAGE.md`](http://forge.forkwright.com/forkwright/kanon/tree/main/projects/theatron/PLATFORM_COVERAGE.md).
