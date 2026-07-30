//! Fitness function: no workspace member shares `naga`'s `codespan-reporting`.
//!
//! WHY: `codespan-reporting` 0.12 made the writer parameter of `term::emit`
//! depend on its own features — `&mut dyn WriteColor` under `termcolor`,
//! `&mut dyn std::io::Write` under `std` alone, `&mut dyn core::fmt::Write`
//! under neither. `naga` depends on it with `default-features = false` and
//! selects its diagnostic buffer type from *its* own `termcolor` / `stderr`
//! features. The two agree only while nothing else turns a feature on behind
//! naga's back.
//!
//! When a workspace member shares a semver-compatible `codespan-reporting`
//! copy with naga, cargo unifies that member's features into naga's copy.
//! naga then builds its buffer as `String` while `term::emit` demands
//! `WriteColor`, and the build fails inside a dependency we do not control:
//! `the trait bound std::string::String: WriteColor is not satisfied`.
//!
//! That is theatron#233: a dependabot bump moved the workspace pin to
//! `codespan-reporting = "0.12"`, which unified `dokimasia`'s default features
//! into naga's copy and broke `full-gate-build` on `main`. theatron#234 fixed
//! it by moving the workspace pin to `"0.13"`, outside naga's `^0.12`
//! requirement — but only a comment recorded that, so the next release on
//! either side that makes the requirements overlap would silently re-unify
//! them.
//!
//! INVARIANT: the `codespan-reporting` package instance naga resolves to is
//! never also resolved to by a theatron workspace member.
//!
//! WARNING: this asserts over resolve-graph *edges*, never over
//! `resolve.nodes[].features`. `cargo metadata` resolves features with v1
//! semantics and ignores this workspace's `resolver = "3"`, so its reported
//! feature sets describe no real build — under the broken 0.12 pin it reports
//! naga *with* `termcolor`, which would make a feature-comparing assertion
//! pass on the state it exists to catch. Package identity is unaffected by
//! that limitation.
//!
//! NOTE: measured both ways before being relied on. At the current `"0.13"`
//! pin naga keeps `0.12.0` while `dokimasia` takes `0.13.1`, and this test
//! passes. At `"0.12"` the resolve graph puts `dokimasia` and naga on a single
//! `codespan-reporting 0.12.0`, so `shared` is non-empty and the assertion
//! fires.
//!
//! WARNING: that second measurement is read from `cargo metadata` directly,
//! not by running this test, and it cannot be taken by running it. Since
//! theatron#234 `dokimasia` calls `term::emit_to_write_style`, which
//! `codespan-reporting` 0.12 does not provide, so at that pin the crate fails
//! to compile and the test binary never builds. Restoring the pin is therefore
//! not a way to rehearse this failure — compare the resolve-graph edges
//! instead.

use std::collections::BTreeMap;
use std::process::Command;

/// First `codespan-reporting` minor release whose `term::emit` signature is
/// feature-gated. Earlier releases take `&mut dyn WriteColor` unconditionally,
/// so sharing one cannot desynchronise it from naga.
const FIRST_GATED_MINOR: u64 = 12;

/// Parses the minor component of a `major.minor.patch` version.
fn minor_of(version: &str) -> u64 {
    version
        .split('.')
        .nth(1)
        .and_then(|component| component.parse().ok())
        .unwrap_or(0)
}

#[test]
fn naga_codespan_copy_is_not_shared_with_a_workspace_member() {
    let workspace_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1"])
        .current_dir(workspace_root)
        .output()
        .expect("run cargo metadata");

    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let meta: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse cargo metadata as JSON");

    let mut names = BTreeMap::new();
    for pkg in meta["packages"]
        .as_array()
        .expect("metadata carries a packages array")
    {
        let id = pkg["id"].as_str().expect("package id is a string");
        let name = pkg["name"].as_str().expect("package name is a string");
        let version = pkg["version"]
            .as_str()
            .expect("package version is a string");
        names.insert(id.to_owned(), (name.to_owned(), version.to_owned()));
    }

    let members: Vec<&str> = meta["workspace_members"]
        .as_array()
        .expect("metadata carries workspace_members")
        .iter()
        .map(|id| id.as_str().expect("workspace member id is a string"))
        .collect();

    // Only `codespan-reporting` edges matter, keyed by the dependent's id.
    let mut codespan_edges: Vec<(String, String)> = Vec::new();
    for node in meta["resolve"]["nodes"]
        .as_array()
        .expect("metadata carries resolve.nodes")
    {
        let id = node["id"].as_str().expect("node id is a string");
        for dep in node["deps"]
            .as_array()
            .expect("node carries a deps array")
            .iter()
        {
            let dep_id = dep["pkg"].as_str().expect("dep pkg is a string");
            let Some((dep_name, dep_version)) = names.get(dep_id) else {
                continue;
            };
            if dep_name == "codespan-reporting" && minor_of(dep_version) >= FIRST_GATED_MINOR {
                codespan_edges.push((id.to_owned(), dep_id.to_owned()));
            }
        }
    }

    let naga_copies: Vec<&String> = codespan_edges
        .iter()
        .filter(|(dependent, _)| names.get(dependent).is_some_and(|(name, _)| name == "naga"))
        .map(|(_, dep)| dep)
        .collect();

    let shared: Vec<String> = codespan_edges
        .iter()
        .filter(|(dependent, dep)| {
            members.contains(&dependent.as_str()) && naga_copies.contains(&dep)
        })
        .map(|(dependent, dep)| {
            let member = names
                .get(dependent)
                .map_or_else(|| dependent.clone(), |(name, _)| name.clone());
            let version = names
                .get(dep)
                .map_or_else(String::new, |(_, version)| version.clone());
            format!("{member} shares codespan-reporting {version} with naga")
        })
        .collect();

    assert!(
        shared.is_empty(),
        "{}\n\
         Cargo unifies a workspace member's codespan-reporting features into naga's copy, \
         which desynchronises `term::emit`'s writer type from naga's diagnostic buffer and \
         fails the build inside naga with \
         `the trait bound std::string::String: WriteColor is not satisfied`.\n\
         Keep this workspace's codespan-reporting requirement outside the requirement of \
         every naga in the graph so the two never resolve to one copy. See theatron#233.",
        shared.join("\n")
    );

    assert!(
        !naga_copies.is_empty(),
        "no naga edge to codespan-reporting {FIRST_GATED_MINOR}+ found in the resolve graph — \
         this fitness function no longer measures anything and must be updated or removed"
    );
}
