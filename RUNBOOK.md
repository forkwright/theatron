# Runbook

This runbook covers local maintenance for the theatron workspace. Use
the repo `CLAUDE.md` for coding conventions and `_meta/RELEASE.md`
for release cuts.

## Local Gates

Run the standard gate before opening a pull request:

```bash
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test
kanon lint --summary <touched-path>
```

For CI parity, also run the workspace forms when a change touches
shared dependencies, features, or examples:

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

## Release Hygiene

1. Keep all eight crates on one workspace version.
2. Update `CHANGELOG.md` for user-visible changes.
3. Keep `kanon/projects/theatron/STATE.md`, `kanon/projects/theatron/ROADMAP.md`,
   and `_llm/current_state.toml` aligned after a release or planning-state
   change.
4. Cut tags only from `main` after the release checklist in
   `_meta/RELEASE.md` passes.

## Consumer-Pull Work

Theatron should not add speculative public helpers. Use a
research-archive note or issue body that names the consuming repo,
the duplicated local pattern, and the expected shared API. Keep the
PR narrow enough that the SemVer impact is obvious.

Cross-repo pin bumps belong to the owning repo manager. Theatron may
record evidence, but it should not edit aletheia, kanon, harmonia, or
akroasis from this repo.

## Operational Checks

Use MCP or GitHub to verify open work:

```bash
gh issue list -R forkwright/theatron --state open
gh pr list -R forkwright/theatron --state open
```

Audit-fleet writes to the forge audit store. If stoa already holds
the writer lock, use the MCP audit report list and retry the CLI
audit later.
