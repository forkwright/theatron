# full_app

Runnable reference consumer for all six theatron desktop crates in one
Dioxus app. Theme toggle with persistence, tray icon, menu bar,
global hotkeys, skeue components, gramma syntax highlighting, and a
stubbed keryx SSE consumer.

```bash
# default (core surfaces)
cargo run -p theatron-example-full-app

# all optional surfaces
cargo run -p theatron-example-full-app --features "menus global-hotkeys default-icon"
```

| Feature | What it exercises |
|---|---|
| `menus` | `mekhane::use_app_menu_event_handler` + `muda::Menu` |
| `global-hotkeys` | `mekhane::use_global_hotkey_event_handler` + bathron notification on trigger |
| `default-icon` | `mekhane::tray::default_icon` PNG-bytes-to-Icon helper |

See [`_meta/INTEGRATION.md`](../../_meta/INTEGRATION.md) for the
consumer patterns this example operationalizes.
