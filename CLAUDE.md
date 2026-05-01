<!--
scope: theatron repo conventions (Dioxus desktop UI crates)
defers_to: ~/menos-ops/CLAUDE.md for machine topology; ~/.claude/CLAUDE.md for operator principles
tightens: per-crate CLAUDE.md files under crates/*/ can narrow conventions within their blast radius
-->

# CLAUDE.md

## At a glance

Repo-level conventions for AI coding agents working on Theatron. Key crates: themelion, mekhane, skeue, gramma, keryx, bathron, parodos, dokimasia. Entry points vary by crate — see crate-level docs.

## Standards

Universal: fleet standards via `~/dev/kanon/crates/basanos/standards/`

## Structure

Workspace of 8 Greek-named crates providing desktop UI infrastructure for the forkwright/menos ecosystem.

## Commands

```bash
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
```

## Key patterns

- **Errors:** `snafu` with `.context()` propagation
- **Lints:** `#[expect(lint, reason = "...")]` over `#[allow]`; every suppression justified
- **Visibility:** `pub(crate)` by default; `pub` only for cross-crate API surface
- **No barrel files**: import from the file that owns the symbol
- **Module imports flow downward**: higher layers depend on lower, never reverse

## Before submitting

1. `cargo check --workspace` passes
2. `cargo clippy --workspace --all-targets -- -D warnings`: zero warnings
3. No `unwrap()` in library code
4. All lint suppressions use `#[expect]` with reason, not `#[allow]`

## Git

Conventional commits: `<type>(<scope>): <description>`. Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `ci`, `perf`. Present tense imperative, first line ≤72 chars. Scope is the crate name.

| Branch Type | Pattern | Example |
|-------------|---------|---------|
| Feature | `feat/<description>` | `feat/virtual-list` |
| Bug fix | `fix/<description>` | `fix/theme-toggle` |
| Docs | `docs/<description>` | `docs/deployment-guide` |
| Refactor | `refactor/<description>` | `refactor/config-cascade` |
| Chore | `chore/<description>` | `chore/update-deps` |

Branch from `main`. Rebase before pushing. Always squash merge.
