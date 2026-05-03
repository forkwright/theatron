# minimal

Smallest possible theatron desktop app  -  Dioxus 0.7 + Blitz native
renderer + the kanon dye-token CSS variables wired in via `<style>`.

```bash
cargo run -p theatron-example-minimal
```

Use as the seed shape when scaffolding a new fleet desktop surface.
The `app` function and CSS block are the only consumer-specific
bits; everything else (theme provider conventions, dye-token
vocabulary, `mekhane::launch` variants for tray / menus / hotkeys)
layers on top.

See [`_meta/INTEGRATION.md`](../../_meta/INTEGRATION.md) for the full
consumer guide.
