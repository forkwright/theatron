# Release process  -  theatron

How a theatron version makes it from `main` to a tagged release that
fleet consumers can pin against.

## Pre-release checklist

Run before cutting any release tag:

1. **Tree state.** Working tree clean, on `main` (or
   `release/X.Y` for a backported patch).
2. **Versions match.** `Cargo.toml` workspace `version` matches the
   tag you're about to cut. All eight crate `Cargo.toml`s use
   `version.workspace = true` (verify with
   `grep -L 'version.workspace = true' crates/*/Cargo.toml`  -  empty
   output is the goal).
3. **Gate green.** `cargo fmt --check && cargo clippy --workspace
   --all-targets --all-features -- -D warnings && cargo test
   --workspace --all-features` all pass on the release commit.
4. **Lint clean.** `kanon lint .` adds zero new violations vs the
   prior release tag.
5. **Changelog updated.** `CHANGELOG.md` has a section for the
   version being cut, with the date set to today and migration
   notes for any breaking changes.
6. **STATE.md updated.** `kanon/projects/theatron/STATE.md` reflects
   the new release under "Released".
7. **Consumer-side smoke test.** At least one fleet consumer
   (chalkeion is canonical) builds + runs against the release
   commit. Document the smoke in the changelog entry.

## Cutting the tag

```bash
# From a clean main, on the commit that will be the release:
git tag -a vX.Y.Z -m "theatron vX.Y.Z

<one-paragraph summary mirroring the changelog headline>

Full notes: CHANGELOG.md
"
git push origin vX.Y.Z
```

No GitHub release artifacts; consumers pin via git URL + tag.

## Patch backports (post-1.0)

Patch fixes on a released minor line land via `release/X.Y`:

```bash
git checkout main
git checkout -b release/X.Y vX.Y.0          # only on the first patch
# … cherry-pick the fix commits from main onto release/X.Y …
git tag -a vX.Y.{Z+1} -m "theatron vX.Y.{Z+1}: <one-line>"
git push origin release/X.Y
git push origin vX.Y.{Z+1}
```

Backport rules:
- Only patch-eligible commits per the SemVer policy.
- Each backport gets a `CHANGELOG.md` entry under the
  `vX.Y.Z+1` heading.
- Re-run the pre-release checklist against the cherry-picked tip
  before tagging.

## Yanking a release

If a tagged release ships with a critical bug:

1. **Don't delete the tag.** Tags are public; deletion breaks anyone
   who already pinned. Leave the tag in place.
2. Cut a fix tag (`vX.Y.{Z+1}`) and announce in the changelog that
   `vX.Y.Z` is yanked with a one-line reason and the upgrade target.
3. If the fix is fundamental and the release is a few hours old at
   most, document the yank in `kanon/projects/theatron/STATE.md` so
   the next operator
   doesn't re-introduce the bug.

## Pre-release versions

Alpha / beta / RC tags follow SemVer pre-release ordering:

```
v1.0.0-alpha.1   # earliest of the v1.0 line
v1.0.0-beta.1
v1.0.0-rc.1
v1.0.0           # final
```

Cut from `main` for alphas + betas; cut from `release/X.Y` for RCs
once the release branch exists.

## Cross-references

- [`SEMVER.md`](./SEMVER.md)  -  versioning policy.
- [`CHANGELOG.md`](./CHANGELOG.md)  -  release notes archive.
- [`STATE.md`](./STATE.md)  -  current development state.
