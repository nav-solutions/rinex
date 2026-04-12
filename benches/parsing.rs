//! Benchmarking RINEX parsing & formatting
//! using most common format and tiny file
extern crate criterion;
use criterion::{criterion_group, criterion_main, Criterion};

use rinex::prelude::Rinex;

fn rinex_parsing(path: &str) {
    let _ = Rinex::from_file(path).unwrap();
}

fn benchmark(c: &mut Criterion) {
    let mut parsing_grp = c.benchmark_group("parsing");

    // Small RINEX OBS (V2)
    parsing_grp.bench_function("OBS/V2", |b| {
        b.iter(|| {
            rinex_parsing("data/OBS/V2/rovn0010.21o");
        })
    });

    // Small RINEX OBS (V3)
    parsing_grp.bench_function("OBS/V3", |b| {
        b.iter(|| {
            rinex_parsing("data/OBS/V3/DUTH0630.22O");
        })
    });

    parsing_grp.finish();
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
