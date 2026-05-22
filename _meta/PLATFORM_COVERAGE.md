# Platform coverage - theatron

Theatron is Linux-first. This matrix records what the repo compiles
and documents, plus the operator validation needed before v1.4 can
claim broader platform parity.

Coverage terms:

- **Validated** - repo tests or examples have been exercised on that
  OS by the release process.
- **Compiled** - the code is conditionally available for that target
  and expected to compile, but no release smoke has verified runtime
  behavior on that OS.
- **Dependency-backed** - the upstream crate advertises or provides
  platform support; theatron has not added its own parity test.
- **Out of scope** - intentionally not claimed in the current release
  line.

## Current Matrix

| Surface | Crate / feature | Linux | macOS | Windows | Notes |
|---|---|---:|---:|---:|---|
| Dioxus + Blitz launch wrappers | `mekhane` default | Validated | Out of scope | Out of scope | Fleet desktop consumers are Linux-first; composition wraps unmodified `dioxus_native`. |
| System tray icon + tray menu events | `mekhane` default | Validated | Compiled | Compiled | `tray-icon` is enabled for `linux`, `macos`, and `windows`; non-Linux runtime smoke remains v1.4 work. |
| App menu bar | `mekhane/menus` | Validated | Dependency-backed | Dependency-backed | Exposes `muda::Menu` directly. Linux smoke is via examples; macOS/Windows need manual runtime checks. |
| Global hotkeys | `mekhane/global-hotkeys` | Validated | Dependency-backed | Dependency-backed | Event delivery is through `tokio::sync::broadcast`; registration behavior is owned by `global-hotkey`. |
| Default icon PNG decode | `mekhane/default-icon` | Validated | Validated | Validated | Pure PNG decoding plus `tray_icon::Icon::from_rgba`; tests cover valid and invalid bytes independent of OS. |
| Notifications | `bathron/notifications` | Dependency-backed | Dependency-backed | Dependency-backed | Thin `notify-rust` wrapper; release validation has not claimed cross-OS behavior. |
| Dialogs | `bathron/dialogs` | Dependency-backed | Dependency-backed | Dependency-backed | Thin `rfd` wrapper for file and message dialogs. |
| Settings | `bathron/settings` | Validated | Compiled | Compiled | Cross-platform path tests cover behavior that does not require native UI integration. |
| Logging | `bathron/logging` | Validated | Compiled | Compiled | File/stderr logging is platform-neutral; release validation has been Linux-local. |
| TUI utilities | `parodos` | Validated | Compiled | Compiled | Ratatui/crossterm substrate is not OS-hooked. Windows terminal details remain consumer responsibility. |
| Markdown/diff state | `gramma` | Validated | Validated | Validated | Pure parsing/highlighting state; no OS integration. |
| Network/SSE | `keryx` | Validated | Validated | Validated | HTTP/SSE substrate is OS-neutral. |
| Components/theme/lint | `skeue`, `themelion`, `dokimasia` | Validated | Validated | Validated | Pure Rust/Dioxus/design-token surfaces; native renderer parity still depends on consumer runtime. |

## v1.4 Parity Work

Before moving any macOS or Windows cell from dependency-backed or
compiled to validated:

1. Run `examples/minimal` on the target OS.
2. Run `examples/tray_smoke` with `menus global-hotkeys default-icon`.
3. Exercise bathron `notifications` and `dialogs` manually on the
   target OS.
4. Record the target OS version, desktop/session environment, and any
   required permissions in this file.
5. Keep any fix additive unless the SemVer policy explicitly permits a
   v2 break.

The release process should continue to say "Linux-first" until those
checks are recorded.
