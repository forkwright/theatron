//! Criterion benchmarks for `parodos::hyperlink`.
#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

use criterion::{BenchmarkId, Criterion};

/// Build a block of text containing `count` URLs.
fn url_block(count: usize) -> String {
    let mut out = String::with_capacity(count * 80);
    for i in 0..count {
        out.push_str("See https://example.com/page/");
        out.push_str(&i.to_string());
        out.push_str(" for details. ");
    }
    out
}

fn bench_detect_urls(c: &mut Criterion) {
    let mut group = c.benchmark_group("detect_urls");

    let small = url_block(1);
    let medium = url_block(20);
    let large = url_block(320);

    for (label, text) in [("small", &small), ("medium", &medium), ("large", &large)] {
        group.bench_with_input(BenchmarkId::from_parameter(label), text, |b, input| {
            b.iter(|| parodos::hyperlink::detect_urls(input));
        });
    }

    group.finish();
}

fn bench_osc8_open(c: &mut Criterion) {
    let mut group = c.benchmark_group("osc8_open");

    let short = "https://example.com";
    let medium = "https://docs.example.com/en/guide/chapter/section/page.html?foo=bar&baz=qux";
    let long = format!("https://example.com/{}", "a/".repeat(50));

    for (label, url) in [("short", short), ("medium", medium), ("large", &long)] {
        group.bench_with_input(BenchmarkId::from_parameter(label), url, |b, input| {
            b.iter(|| parodos::hyperlink::osc8_open(input));
        });
    }

    group.finish();
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    bench_detect_urls(&mut criterion);
    bench_osc8_open(&mut criterion);
    criterion.final_summary();
}
