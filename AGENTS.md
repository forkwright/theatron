# AGENTS.md

<!--
scope: theatron repo — agent onboarding and dispatch conventions
defers_to: CLAUDE.md for full coding conventions; README.md for architecture overview
-->

## Purpose

Theatron is a Rust workspace of 8 Greek-named crates providing desktop UI infrastructure
for the forkwright fleet. Agents working here add or fix components, fix CI/lint/gate
failures, update dependencies, and maintain the doc set. The primary consumers are
aletheia, kanon/chalkeion, and harmonia.

## Crates

| Crate | Role |
|-------|------|
| `themelion` | Foundation — theme provider, routing scaffolding, error boundary, settings, logging |
| `mekhane` | Stage machinery — windowing, event loop, OS hooks (tray, hotkeys, native menus) |
| `skeue` | Props/equipment — generic Dioxus components: status pill, metric tile, queue table, sparkline, diff hunk, virtual list |
| `gramma` | Written character — markdown + syntax highlighting + diff state (pulldown-cmark + syntect) |
| `keryx` | Herald/messenger — HTTP client, SSE/streaming, retry, `ApiError` |
| `bathron` | Pedestal/base — OS-service integration: notifications, file dialogs, settings persistence, logging |
| `parodos` | Stage entrance — TUI substrate: Ratatui primitives + Elm state/update/view dispatcher |
| `dokimasia` | Examination — design-token + standards enforcement linter; fails CI on undeclared token refs |

## Build notes

`mekhane`, `examples/tray_smoke`, and `examples/full_app` require GTK3 system libraries
(`libgtk-3-dev libglib2.0-dev libgdk-pixbuf2.0-dev libatk1.0-dev` on Debian/Ubuntu).
On headless boxes without GTK3, check only the headless-compatible crates:

```
export PATH="$HOME/.cargo/bin:$PATH"
cargo check -p gramma -p bathron -p keryx -p skeue -p themelion -p parodos -p dokimasia
cargo clippy -p gramma -p bathron -p keryx -p skeue -p themelion -p parodos -p dokimasia -- -D warnings
cargo nextest run -p gramma -p bathron -p keryx -p skeue -p themelion -p parodos -p dokimasia
```

Full workspace check (requires GTK3 dev headers):
```
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
```

## Gate

Before opening a PR, the gate must pass locally. Branch protection requires a
`Gate-Passed:` trailer on at least one commit in the PR:

```
Gate-Passed: kanon <version>
```

Run `kanon gate .` locally; the command prints the trailer to use. Docs-only or
workflow-only diffs may use a descriptive inline attestation (e.g.
`Gate-Passed: docs-only; no Rust changes`). Never fabricate the trailer.

## Commit convention

```
<type>(<crate-name>): <description in present tense imperative>
```

Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `ci`, `perf`.
Scope is the crate name or `repo` for root-level changes. First line ≤ 72 chars.
Branch from `main`; one PR per focused change; squash-merge only.

## Key invariants

- `mekhane` uses `kanon:ignore RUST/expect` on OS-level tray/hotkey init panics — these
  are documented, unrecoverable OS errors. Do not remove those annotations or replace
  with `unwrap()`.
- The dioxus pin (`=0.7.6`) is intentional — see the WHY comment in the root Cargo.toml.
  Never auto-bump dioxus; coordinate with fleet consumers.
- `dokimasia` lints run as part of `kanon lint` in CI; do not break its token-registry
  logic when touching CSS or design token files.
- Workspace version (`[workspace.package] version`) and all intra-workspace path dep
  versions in `[workspace.dependencies]` must stay in sync after every release.
