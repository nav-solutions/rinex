use crate::{
    prelude::Rinex,
    tests::toolkit::{generic_rinex_comparison, random_name},
};

use std::{
    str::FromStr,
    path::Path,
};

mod clk;
mod meteo;
mod nav;
mod obs;

#[cfg(feature = "log")]
use log::{debug, info};

/// Parses input RINEX file (must be valid and succeed),
// formats it without any modification,
// parses it back & runs the generic comparison method (will fail on data mismatch)
fn testbench(path: &str) {

    let path = Path::new(path);

    #[cfg(feature = "log")] {
        let stem = path.file_stem().unwrap(); 
        debug!("(prod bench): parsing file \"{}\"", path);
    }

    let model = if path.ends_with(".gz") {
        Rinex::from_gzip_file(path)
    } else {
        Rinex::from_file(path)
    };

    let model = model.unwrap();

    let tmp_path = format!("test-{}.rnx", random_name(5));
    model.to_file(&tmp_path).unwrap(); // test writer

    let dut = Rinex::from_file(&tmp_path).unwrap();

    // testbench
    generic_rinex_comparison(&dut, &model);

    #[cfg(feature = "log")]
    info!("Formatting test passed for \"{}\"", path);

    // clean up
    let _ = std::fs::remove_file(tmp_path);
}
