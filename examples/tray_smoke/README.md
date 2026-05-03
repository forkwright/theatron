# tray_smoke

Compile-time smoke test for `mekhane`'s tray + menu + global-hotkey
public surface. Builds + passes clippy in CI; not meant to be `cargo
run` (would open a real desktop window and tray icon).

If this stops compiling, the breakage is in `mekhane`'s public API
and downstream consumers (proskenion, chalkeion, future
harmonia-desktop / akroasis-desktop) will break too.

```bash
# default (tray only)
cargo build -p theatron-example-tray-smoke

# all optional surfaces
cargo build -p theatron-example-tray-smoke --features "menus global-hotkeys default-icon"
```

| Feature | What it exercises |
|---|---|
| `menus` | `mekhane::use_app_menu_event_handler` + `Menu` from muda |
| `global-hotkeys` | `mekhane::use_global_hotkey_event_handler` + `GlobalHotKeyEvent` |
| `default-icon` | `mekhane::tray::default_icon` PNG-bytes-to-Icon helper |

See [`_meta/INTEGRATION.md`](../../_meta/INTEGRATION.md) for the
runtime patterns.
