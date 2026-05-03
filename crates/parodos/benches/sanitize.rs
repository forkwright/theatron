//! Criterion benchmarks for `parodos::sanitize::sanitize_for_display`.
#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

use criterion::{BenchmarkId, Criterion};

/// Insert ANSI escape sequences into `base` every `period` bytes.
fn dirty(base: &str, period: usize) -> String {
    let mut out = String::with_capacity(base.len() + base.len() / period * 10);
    for (idx, chunk) in base.as_bytes().chunks(period).enumerate() {
        // SAFETY: chunks are byte-aligned slices of a valid UTF-8 str.
        out.push_str(unsafe { std::str::from_utf8_unchecked(chunk) });
        if idx % 2 == 0 {
            out.push_str("\x1b[31m");
        } else {
            out.push_str("\x1b[0m");
        }
    }
    out
}

fn bench_sanitize(c: &mut Criterion) {
    let mut group = c.benchmark_group("sanitize_for_display");

    // Small: typical chat line (~64 bytes).
    let small_clean = "Hello world, this is a typical chat message line here.";
    let small_dirty = dirty(small_clean, 16);

    // Medium: multi-line tool output (~1 KiB).
    let medium_clean = "INFO  build process running step 42 of 100 with flags\n".repeat(20);
    let medium_dirty = dirty(&medium_clean, 64);

    // Large: worst-case tool dump (~16 KiB).
    let large_clean =
        "DEBUG  very long line of tool output from a build or test process.\n".repeat(320);
    let large_dirty = dirty(&large_clean, 128);

    for (label, text) in [
        ("small_clean", small_clean),
        ("small_dirty", small_dirty.as_str()),
        ("medium_clean", medium_clean.as_str()),
        ("medium_dirty", medium_dirty.as_str()),
        ("large_clean", large_clean.as_str()),
        ("large_dirty", large_dirty.as_str()),
    ] {
        group.bench_with_input(BenchmarkId::from_parameter(label), text, |b, input| {
            b.iter(|| parodos::sanitize::sanitize_for_display(input));
        });
    }

    group.finish();
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    bench_sanitize(&mut criterion);
    criterion.final_summary();
}
