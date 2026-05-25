# SemVer policy  -  theatron

Theatron's eight crates ship together as a versioned set on the
`forkwright/theatron` git repo. Consumers pin a single revision via
git URL; the workspace version is the source of truth.

## Versioning

Theatron follows SemVer 2.0 ([semver.org](https://semver.org)) with
the workspace `[workspace.package].version` driving every crate's
published version in lockstep. There is no per-crate independent
versioning  -  the eight crates compose into one consumer surface and
divergent versions would force consumers to track each crate
separately.

| Bump | Trigger |
|---|---|
| **Major** (`X.0.0`) | Any breaking change to a public API in any of the eight crates. Removing a function, narrowing a generic bound, removing a re-export, removing a cargo feature, changing a wire-DTO field shape, changing a default-feature set. |
| **Minor** (`x.Y.0`) | Additive public API: new function, new `#[non_exhaustive]` enum variant, new struct field on a `#[non_exhaustive]` struct, new cargo feature (off by default), new re-export. |
| **Patch** (`x.y.Z`) | Bug fix, doc-only change, internal refactor, perf improvement, test additions. No public-API change. |

Pre-1.0 (`0.x.y`) follows the same rules but with the major/minor
distinction shifted: minor bumps can break (`0.x` → `0.(x+1)`),
patches must remain compatible. After 1.0 the rules above hold
unconditionally.

## What is "public API"

Per crate, the public API surface is:

1. Every `pub` item reachable from the crate root (functions, types,
   traits, modules, constants, type aliases, macros).
2. Every cargo feature in `Cargo.toml` (adding a feature is minor;
   removing one or changing its default is major).
3. Every observable type-system relationship: trait bounds, generic
   parameters, lifetimes, `Send`/`Sync` auto-trait propagation.
4. The set of dependencies whose types appear in the public API.
   Bumping such a dep across its own SemVer-major boundary is itself
   a major bump for theatron.
5. Wire DTOs (in `keryx`)  -  adding a `#[serde(default)]` field is
   minor; renaming or removing a field is major.

Internal items (`pub(crate)`, private modules, private fields on a
non-`#[non_exhaustive]` struct) are NOT public API and may change at
will.

## What is *not* public API

- Implementation details explicitly marked in rustdoc.
- Items marked `#[doc(hidden)]`. These exist for macro hygiene and
  test-only access; consumers using them do so at their own risk.
- Behaviour observable only via panics on misuse (e.g. an `unwrap()`
  on operator-supplied invalid input). The panic itself isn't a
  contract; the input validation rule is.
- Specific `tracing` event payloads and span structures. Logs are
  observability, not API.
- Specific layout / styling output of skeue components. Token
  values are stable (per `kanon/crates/basanos/standards/DESIGN-TOKENS.md`),
  but the exact CSS / element structure each component renders is
  free to evolve as long as the documented design-token contract
  holds.

## Deprecation

Deprecating a public item:

1. Add `#[deprecated(since = "x.y.0", note = "use Y instead")]` on
   the deprecated item in a minor release.
2. Document the migration path in the release notes (`CHANGELOG.md`).
3. Keep the deprecated item compiling for at least one full minor
   cycle (e.g. deprecated in 1.3 stays through 1.4, removable in 1.5).
4. Removal is a major bump.

For DTO field deprecations: mark the field with a
`#[serde(default)]` + `#[deprecated]` and stop reading it
server-side; the field stays on the wire as `null` for one minor
cycle so old clients don't deserialize-fail.

## Branch / tag conventions

- `main`  -  current release line. Tagged `vX.Y.Z` on each release.
- `release/X.Y`  -  long-lived branch for in-flight patch releases on a
  minor line. Created when a minor lands; gets `vX.Y.{Z+1}` patch
  tags as fixes backport.
- Pre-release: `vX.Y.Z-alpha.N`, `vX.Y.Z-beta.N`, `vX.Y.Z-rc.N`
  follow SemVer pre-release ordering.

Consumers pin to a specific `rev = "<sha>"` until v1.0. After v1.0
they may pin to `tag = "vX.Y.Z"` instead.

## What v1.0 means for theatron

The v1.0 cut signals that the eight-crate API surface is stable
enough that:

- Consumer churn from theatron itself drops to zero between major
  cycles. Bug fixes and additive features land without forcing any
  downstream code change.
- A consumer pinned to `vX.0.0` can upgrade to any `vX.y.z` (same
  major) by bumping the pin alone  -  no code edits.
- Breaking changes batch into major cycles with documented migration
  paths, not opportunistic per-PR breakage.

Pre-1.0 status (`0.x.y`) does not promise this; consumers tracking
unreleased revs should expect drift on every theatron change.

## Cross-references

- [`CHANGELOG.md`](../CHANGELOG.md)  -  per-version release notes.
- [`_meta/STATE.md`](./STATE.md)  -  current development state.
- [`_meta/ROADMAP.md`](./ROADMAP.md)  -  forward plan.
- [`README.md`](../README.md)  -  crate inventory + consumer matrix.
