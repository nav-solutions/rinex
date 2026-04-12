//! Benchmarking of the low level CRINEX API & associated objects
use rinex::hatanaka::{NumDiff, TextDiff};

extern crate criterion;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn textdiff_decompression(textdiff: &mut TextDiff, data: &[&str]) {
    for data in data {
        let _ = textdiff.decompress(data);
    }
}

fn textdiff_compression(textdiff: &mut TextDiff, data: &[&str]) {
    for data in data {
        let _ = textdiff.compress(data);
    }
}

fn numdiff_decompression<const M: usize>(
    numdiff: &mut NumDiff<M>,
    forced_init: usize,
    pool: &[i64],
) {
    for (index, data) in pool.iter().enumerate() {
        if index % forced_init == 0 {
            numdiff.force_init(*data, 3);
        } else {
            let _ = numdiff.decompress(*data);
        }
    }
}

fn numdiff_compression<const M: usize>(numdiff: &mut NumDiff<M>, forced_init: usize, pool: &[i64]) {
    for (index, data) in pool.iter().enumerate() {
        if index % forced_init == 0 {
            numdiff.force_init(*data, 3);
        } else {
            let _ = numdiff.compress(*data);
        }
    }
}

fn benchmark(c: &mut Criterion) {
    // textdiff benchmarking
    let mut textdiff_grp = c.benchmark_group("textdiff");

    let epochs_pool = vec![
        "KD1KVK71n2Pz6AwiBcM",
        "jhyinxzgq3v6x8zMf8",
        "CjyIN8tBzFCZcjQig8J",
        "p3AhjKQKvjWpdAda",
        "ea5dVWu4UkgzIUZp9Nj",
        "vRbeL7L4StC38JOjNNk",
        "vmatRNwwO8c__aOPLp69v",
        "odzMFRVUSI4m8g2qs84",
        "pExPLrSeNhRYcOdTOr7",
        "KEcPleNxlG9vdW4waDE",
        "ohzMcNbbZm14LRCx",
        "W7Xl6fCvkaNZIXI4ImN",
        "kknmFxgbIU7rN",
        "GimZ3zrGbok7km8D17",
        "FgD6TQwbKdRuaxk",
        "dlWSE4k0xPwJlFoDiFL",
        "9QonBABq----hjyLdP0AQ9y",
        "gYhNYkFKW4nIRM3QrhS",
        "Cgajyl3ouDzRMAhAoPD",
        "SZV2vxHJzTSRjZr9Rxa",
        "qqwDdfWdePazeaze6qdzho",
        "jxeDng8sTNHVO4skvvS",
        "xjb9KfOa73HGe4U",
        "oL8k531dLdzsbSp6qfW",
        "ufHAShfefHuOf7G0yTn",
        "QLDV0vs1H9VFBsvWuoN",
        "TsHawHBBJ6yec0TUs2z",
        "m8loLwvmC@@__ @@iW6cEwy",
        "pkqFgJdNxTM2YjZbDUJ",
        "Gf5aLDwTnekdYvIQ--@azeP10",
        "zxUKBAT432t9UFJt$$@$$ZUX",
    ];

    let flags_pool = vec![
        "1", "", "3", "", "X", "0", "", "", "p", "K", "", "", "k", "G", "F", "", "", "g", "C", "S",
        "", "", "", "", "1", "3", "", "2", "3", "", "1",
    ];

    let epochs_len = epochs_pool.len();
    let mut textdiff = TextDiff::new("NyV1xYEQcXyv2zzlG1A");

    // epoch like text
    textdiff_grp.bench_function("compress/epoch", |b| {
        b.iter(|| {
            textdiff_compression(&mut textdiff, &epochs_pool[0..epochs_len]);
        })
    });

    textdiff_grp.bench_function("decompress/epoch", |b| {
        b.iter(|| {
            textdiff_decompression(&mut textdiff, &epochs_pool[0..epochs_len]);
        })
    });

    let flags_len = flags_pool.len();
    let mut textdiff = TextDiff::new("X");

    // flags like text
    textdiff_grp.bench_function("compress/flags", |b| {
        b.iter(|| {
            textdiff_compression(&mut textdiff, &flags_pool[0..flags_len]);
        })
    });

    textdiff_grp.bench_function("decompress/flags", |b| {
        b.iter(|| {
            textdiff_decompression(&mut textdiff, &flags_pool[0..flags_len]);
        })
    });

    textdiff_grp.finish();

    // numdiff benchmarking
    let mut numdiff_group = c.benchmark_group("numdiff");

    let forced_init_index = 10;
    let pool_i64 = vec![
        5918760, 92440, -240, -320, -160, -580, 360, -1380, 220, //[RE-INIT]
        -19542118, 29235, -38, 1592, -931, 645, 1001, -1038, 2198, -2679, 2804, -892,
        //[RE-INIT]
        -19542118, 29235, -38, 1592, -931, 645, 1001, -1038, 2198, -2679, 2804, -892,
        //[RE-INIT]
        -19542118, 29235, -38, 1592, -931, 645, 1001, -1038, 2198, -2679, 2804, -892,
        //[RE-INIT]
        -19542118, 29235, -38, 1592, -931, 645, 1001, -1038, 2198, -2679, 2804, -892,
        //[RE-INIT]
        -19542118, 29235, -38, 1592, -931, 645, 1001, -1038, 2198, -2679, 2804, -892,
        //[RE-INIT]
        -19542118, 29235, -38, 1592, -931, 645, 1001, -1038, 2198, -2679, 2804, -892,
        //[RE-INIT]
        -19542118, 29235, -38, 1592, -931, 645, 1001, -1038, 2198, -2679, 2804, -892,
        //[RE-INIT]
        -19542118, 29235, -38, 1592, -931, 645, 1001, -1038, 2198, -2679, 2804, -892,
        //[RE-INIT]
        -19542118, 29235, -38, 1592, -931, 645, 1001, -1038, 2198, -2679, 2804, -892,
    ];

    let mut numdiff = NumDiff::<6>::new(25065408994, 3);

    numdiff_group.bench_function("compress/small", |b| {
        b.iter(|| {
            black_box(numdiff_compression(
                &mut numdiff,
                forced_init_index,
                &pool_i64[0..20],
            ));
        })
    });

    numdiff_group.bench_function("decompress/small", |b| {
        b.iter(|| {
            black_box(numdiff_decompression(
                &mut numdiff,
                forced_init_index,
                &pool_i64[0..20],
            ));
        })
    });

    numdiff_group.bench_function("compress/big", |b| {
        b.iter(|| {
            black_box(numdiff_compression(
                &mut numdiff,
                forced_init_index,
                &pool_i64[0..pool_i64.len()],
            ));
        })
    });

    numdiff_group.bench_function("decompress/big", |b| {
        b.iter(|| {
            black_box(numdiff_decompression(
                &mut numdiff,
                forced_init_index,
                &pool_i64[0..pool_i64.len()],
            ));
        })
    });

    numdiff_group.finish();
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
