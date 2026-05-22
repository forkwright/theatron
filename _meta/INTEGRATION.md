# Integration  -  consuming theatron from a fleet repo

How a fleet desktop or TUI surface consumes theatron's eight crates.
The canonical consumer patterns cover chalkeion, koilon, and
proskenion; no consumer needs anything beyond what's documented here.

## Pin pattern

All eight crates ship together off `forkwright/theatron` git. Pin a
single tag; never mix-and-match versions across crates.

```toml
# Consumer Cargo.toml — workspace.dependencies (or [dependencies] in
# a single-crate project).

[workspace.dependencies]
themelion = { git = "http://forge.forkwright.com/forkwright/theatron.git", tag = "v1.0.0" }
mekhane   = { git = "http://forge.forkwright.com/forkwright/theatron.git", tag = "v1.0.0" }
bathron   = { git = "http://forge.forkwright.com/forkwright/theatron.git", tag = "v1.0.0" }
keryx     = { git = "http://forge.forkwright.com/forkwright/theatron.git", tag = "v1.0.0" }
skeue     = { git = "http://forge.forkwright.com/forkwright/theatron.git", tag = "v1.0.0" }
gramma    = { git = "http://forge.forkwright.com/forkwright/theatron.git", tag = "v1.0.0" }
parodos   = { git = "http://forge.forkwright.com/forkwright/theatron.git", tag = "v1.0.0" }
dokimasia = { git = "http://forge.forkwright.com/forkwright/theatron.git", tag = "v1.0.0" }

# Pin Dioxus to the same exact patch theatron resolves to. EventHandler<T>
# is a different concrete type at every patch release; mismatched pins
# break trait resolution at the crate boundary.
dioxus        = "=0.7.6"
dioxus-core   = "=0.7.6"
dioxus-hooks  = "=0.7.6"
dioxus-native = "=0.7.6"
```

Take only the crates the consumer actually needs. A pure-TUI
consumer (koilon) takes `parodos` and skips the Dioxus-bound crates;
a pure-desktop consumer (chalkeion) takes `themelion` + `mekhane` +
`skeue` + `gramma` + `keryx` + `bathron` and skips `parodos`.

## Desktop consumer  -  minimal app

```rust
use dioxus::prelude::*;
use dioxus_native::launch;

fn app() -> Element {
    rsx! { div { "hello from " {themelion::version()} } }
}

fn main() { launch(app); }
```

See [`examples/minimal/`](../examples/minimal) for a runnable version
with the kanon dye-token CSS variables wired in via `<style>`.

## Desktop consumer  -  with theme provider

`themelion::ThemeProvider` is a Dioxus `#[component]` that wraps the
tree in a `<div data-theme=…>` so the design-token CSS custom-
properties activate. It also installs a `Signal<ThemeMode>` as
context for descendants  -  the canonical consumer (`ThemeToggle`)
reads it via `use_context`.

```rust
use dioxus::prelude::*;
use themelion::{ThemeMode, ThemeProvider};

fn app() -> Element {
    rsx! {
        ThemeProvider {
            initial_mode: Some(ThemeMode::System),
            Body {}
        }
    }
}

// Anywhere downstream:
fn settings_pane() -> Element {
    let mut mode = use_context::<Signal<ThemeMode>>();
    rsx! { /* read or set mode() */ }
}
```

Consumers wire their own persistence on the toggle's `on_change`
callback (proskenion writes to `settings.toml`; chalkeion to its own
state dir).

## Desktop consumer  -  tray + menus + hotkeys (mekhane)

`mekhane` wraps `dioxus_native::launch_cfg_with_props`. The wrapper
installs process-global tray callbacks and provides senders as
contexts. Per-component hooks consume the broadcast and drive a
local handler closure on each event.

```rust
use mekhane::{
    launch,
    tray::{init_tray_icon, default_tray_icon, TrayIconEvent},
    use_tray_icon_event_handler,
};

fn app() -> Element {
    use_hook(|| init_tray_icon(default_tray_icon(), None));
    use_tray_icon_event_handler(|event: &TrayIconEvent| {
        // route to focus / show / hide
    });
    rsx! { div { "tray-enabled app" } }
}

fn main() { launch(app); }
```

For consumers that need a top-of-window app menu bar at launch time,
use `mekhane::launch_cfg_with_props_and_menu` (feature `menus`) and
pass an optional `muda::Menu` as the trailing argument; pair it with
`use_app_menu_event_handler` to receive `MenuEvent`s.

Cargo features (off by default; opt in per consumer):

| Feature | What it enables |
|---|---|
| `menus` | App menu via `muda`. Adds `use_app_menu_event_handler`. |
| `global-hotkeys` | OS-wide keyboard shortcuts via `global-hotkey`. Adds `use_global_hotkey_event_handler`. |
| `default-icon` | `tray::default_icon` PNG-bytes-to-Icon helper. |

See [`examples/tray_smoke/`](../examples/tray_smoke) for the full
tray + menu + hotkey surface as a compile-time smoke test.

## Desktop consumer  -  OS services (bathron)

`bathron` exposes four OS-service modules behind per-feature gates so
consumers pay only for what they use.

```toml
[dependencies]
bathron = { workspace = true, features = ["notifications", "settings"] }
```

| Feature | Module | What it does |
|---|---|---|
| `notifications` | `bathron::notifications` | Native toast / OS notifications via notify-rust |
| `dialogs` | `bathron::dialogs` | Open / save / pick-folder / message-box dialogs via rfd |
| `settings` | `bathron::settings` | TOML KV store with cascade (system → user → app) |
| `logging` | `bathron::logging` | tracing-subscriber adapter with file + console writers |

Each module's docs cover the per-platform behaviour. Linux is the
validated target; macOS / Windows status is tracked in
[`PLATFORM_COVERAGE.md`](./PLATFORM_COVERAGE.md) until v1.4 parity
work proves more.

## Desktop consumer  -  HTTP + SSE (keryx)

`keryx` owns the SSE-stream framing, the `SseEvent` type, and the
`ApiError` type. It does *not* own the HTTP client or consumer DTOs
 -  consumers bring their own reqwest (or other) byte-stream and the
DTO contract for each event name.

```rust
use futures_util::StreamExt;
use keryx::{ApiError, SseEvent, SseStream};

async fn watch(client: &reqwest::Client, url: &str) -> Result<(), ApiError> {
    let resp = client.get(url).send().await?;
    let mut sse = SseStream::new(resp.bytes_stream());
    while let Some(event) = sse.next().await {
        match event? {
            SseEvent { event, data, .. } => {
                // dispatch on event name per consumer's wire contract
                let _ = (event, data);
            }
        }
    }
    Ok(())
}
```

Consumer DTO crates (e.g. `kanon-api-types`) depend on `keryx` and
layer strongly-typed event variants on top of the raw `SseEvent`.
Real consumer pattern: see `aletheia/skene/src/api/sse.rs` and
`aletheia/proskenion/src/api/sse.rs`.

## TUI consumer  -  parodos re-exports

`parodos` is the TUI substrate. The five infrastructure modules
(`theme`, `sanitize`, `clipboard`, `highlight`, `hyperlink`) extracted
from aletheia/koilon are re-exported wholesale; consumers drop them
in via `pub use`.

```rust
// In a consumer's TUI crate:
pub mod theme { pub use parodos::theme::*; }
pub mod sanitize { pub use parodos::sanitize::*; }
pub mod clipboard { pub use parodos::clipboard::*; }
pub mod highlight { pub use parodos::highlight::*; }
pub mod hyperlink { pub use parodos::hyperlink::*; }
```

`parodos::env::Env` is a small trait (`fn var(&self, name: &str) ->
Option<String>`) that the clipboard + hyperlink modules use to detect
terminal capabilities (`COLORTERM`, `TERM_PROGRAM`, `KITTY_PID`, …)
without taking a hard dependency on `aletheia/koina`. Production
code passes `parodos::RealEnv`; tests pass a stub.

The capability detector reads from `RealEnv` internally, so most
consumers don't need to touch the trait at all:

```rust
let supports_osc8 = parodos::hyperlink::supports_hyperlinks();
let result = parodos::clipboard::copy_to_clipboard("hello");
```

Only when injecting a stub for tests do consumers construct an `Env`
implementation directly.

The `parodos::fuzzy` matcher is a generic subsequence-matching
primitive used by command palettes and search inputs. It's
DTO-free; consume directly.

## Markdown rendering (gramma)

`gramma` wraps pulldown-cmark + syntect for any consumer that renders
Markdown content. Two modules:

| Module | Use |
|---|---|
| `gramma::highlight` | syntect-backed code highlighting, no Dioxus dep  -  usable from TUI or web |
| `gramma::diff` | unified-diff parser + state types  -  usable from any view layer |

The output of both is renderer-agnostic data: `gramma::diff::DiffHunk`
holds the parsed structure, and the consumer's component (skeue's
`DiffHunkView` for desktop; a parodos-bound widget for TUI) renders
it.

## Visual components (skeue)

`skeue` is the Dioxus component library per kanon's
`basanos/standards/DESIGN-TOKENS.md` anatomy. Each component carries
a stable `#[component]` Props struct and renders against the design-
token CSS custom-property vocabulary  -  it doesn't own colour or
spacing values, only structure.

The full v1.0 inventory:

| Component | Re-export path |
|---|---|
| `StatusPill` (+ `StatusPillKind`, `StatusPillShape`) | `skeue::status_pill` |
| `ConnectionIndicator` (+ `IndicatorTone`) | `skeue::conn_indicator` |
| `MetricTile` (+ `MetricDelta`, `DeltaDirection`, `DeltaTone`) | `skeue::metric_tile` |
| `Sparkline` (+ `SparklineShape`, `SparklineTone`) | `skeue::sparkline` |
| `ActivityRow` (+ `ActivityStatus`, `RowDensity`) | `skeue::activity_row` |
| `QueueTable` (+ `QueueColumn`, `QueueItem`) | `skeue::queue_table` |
| `MdTable` (+ `TableAlignment`) | `skeue::table` |
| `VirtualScrollContainer` | `skeue::virtual_list` |
| `ToastItem` (+ `Toast`, `ToastAction`, `ToastId`, `ToastSeverity`) | `skeue::toast` |
| `CodeBlock` | `skeue::code_block` |
| `DiffHunkView`, `DiffLineView` | `skeue::diff_hunk`, `skeue::diff_line` |

Each component's rustdoc documents the props (slot vocabulary, dye
tokens it consumes, allowed kind / tone / shape variants). Pull
the rendered docs with `cargo doc -p skeue --open` against a
v1.0.0 checkout.

## Linting (dokimasia)

`dokimasia` is the design-token + standards enforcer. Consumers
bring it in via the `dokimasia` binary (recommended) or as a library
inside a custom CI step.

Recommended setup: a `.kanon-ci.toml` stage that runs `dokimasia`
against the consumer's source tree. The dokimasia rule namespace was
frozen at v1.0; new rules add at minor bumps with a
DESIGN-TOKENS.md / STANDARDS.md reference for each.

```toml
# .kanon-ci.toml in the consumer repo
[[stages]]
name = "dokimasia"
cmd = "dokimasia ."
```

Suppression lives in a per-repo `.kanon.yml` `suppress` block, file-
scoped, with a reason. Inline `// kanon:ignore RULE/NAME` is
discouraged because rustfmt may relocate the comment off its target
line.

## Common pitfalls

- **Dioxus version drift.** Every crate that uses
  `EventHandler<T>` or any other Dioxus type parameterized over
  consumer types must pin the *exact same* Dioxus patch version as
  theatron. Resolve them all to `=0.7.6` at the consumer's workspace
  level. Mixed pins compile in isolation and fail with cryptic trait
  errors at the crate boundary.
- **ratatui 0.30 + crossterm 0.29.** parodos pins the workspace at
  these majors. Consumers using parodos must pin matching versions
  in their own ratatui-bound crates  -  a 0.29 / 0.30 mix triggers
  unicode-width resolution conflicts.
- **`#[non_exhaustive]` enums.** Public enums in
  `parodos::clipboard::ClipboardContent` and
  `mekhane::tray::TrayIconEvent` are `#[non_exhaustive]`. Add a
  wildcard arm to any consumer-side `match` or risk breakage on a
  minor bump that adds a variant.
- **Single tag pin.** Don't mix tags across the eight crates. The
  workspace ships at one version; staggered consumer pins produce a
  duplicate-version graph and trait-resolution failures.

## Cross-references

- [`SEMVER.md`](./SEMVER.md)  -  versioning rules (what counts as
  breaking, deprecation pattern, branch / tag conventions)
- [`RELEASE.md`](./RELEASE.md)  -  release process for theatron
  itself (relevant if you're patching theatron, not consuming it)
- [`CHANGELOG.md`](./CHANGELOG.md)  -  per-version release notes
- [`STATE.md`](./STATE.md)  -  current development state
- [`ROADMAP.md`](./ROADMAP.md)  -  forward plan
- [`PLATFORM_COVERAGE.md`](./PLATFORM_COVERAGE.md)  -  OS-hook coverage matrix
- [`README.md`](../README.md)  -  crate inventory + consumer matrix
