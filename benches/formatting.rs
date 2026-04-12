//! Benchmarking RINEX parsing & formatting
//! using most common format and tiny file
extern crate criterion;
use criterion::{criterion_group, criterion_main, Criterion};

use std::io::{BufWriter, Write};

use rinex::prelude::Rinex;

#[derive(Debug)]
pub struct Utf8Buffer {
    pub inner: Vec<u8>,
}

impl Write for Utf8Buffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for b in buf {
            self.inner.push(*b);
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.clear();
        Ok(())
    }
}

impl Utf8Buffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }
}

fn rinex_formatting<W: Write>(rinex: &Rinex, w: &mut BufWriter<W>) {
    let _ = rinex.format(w).unwrap();
}

fn benchmark(c: &mut Criterion) {
    let mut formatting_grp = c.benchmark_group("formatting");

    let mut buffer = BufWriter::new(Utf8Buffer::new(4096));

    // Small RINEX OBS (V2)
    let rinex = Rinex::from_file("data/OBS/V2/rovn0010.21o").unwrap();

    formatting_grp.bench_function("OBS/V2", |b| {
        b.iter(|| {
            rinex_formatting(&rinex, &mut buffer);
        })
    });

    // Small RINEX OBS (V3)
    let rinex = Rinex::from_file("data/OBS/V3/DUTH0630.22O").unwrap();

    formatting_grp.bench_function("OBS/V3", |b| {
        b.iter(|| {
            rinex_formatting(&rinex, &mut buffer);
        })
    });

    // Small CRINEX (V3)
    let rinex = rinex.rnx2crnx();

    formatting_grp.bench_function("CRNX/V3", |b| {
        b.iter(|| {
            rinex_formatting(&rinex, &mut buffer);
        })
    });

    formatting_grp.finish();
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
