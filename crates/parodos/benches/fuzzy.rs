//! Criterion benchmarks for `parodos::fuzzy::fuzzy_match`.
#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

use criterion::{BenchmarkId, Criterion};

/// Build a candidate string by repeating `fragment` until it reaches at least `target_len`.
fn candidate(fragment: &str, target_len: usize) -> String {
    let repeats = target_len.div_ceil(fragment.len());
    let mut out = fragment.repeat(repeats);
    out.truncate(target_len);
    out
}

fn bench_fuzzy_match(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuzzy_match");

    // Small: typical command-palette entry (~64 bytes, ~6-char pattern).
    let small_candidate = candidate("open file dialog ", 64);
    let small_pattern = "ofdlg";

    // Medium: multi-line command list (~1 KiB, ~32-char pattern).
    let medium_candidate = candidate(
        "git status\ngit log --oneline\ngit diff --cached\ngit checkout -b ",
        1024,
    );
    let medium_pattern = "git log --oneline --graph --all";

    // Large: worst-case document dump (~16 KiB, ~64-char pattern).
    let large_candidate = candidate(
        "performance budget baseline measurement for fuzzy matcher subsequence search\n",
        16384,
    );
    let large_pattern = "performance budget baseline measurement for fuzzy matcher";

    for (label, candidate, pattern) in [
        ("small", small_candidate.as_str(), small_pattern),
        ("medium", medium_candidate.as_str(), medium_pattern),
        ("large", large_candidate.as_str(), large_pattern),
    ] {
        group.bench_with_input(
            BenchmarkId::from_parameter(label),
            &(candidate, pattern),
            |b, (cand, pat)| {
                b.iter(|| parodos::fuzzy::fuzzy_match(cand, pat));
            },
        );
    }

    group.finish();
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    bench_fuzzy_match(&mut criterion);
    criterion.final_summary();
}
