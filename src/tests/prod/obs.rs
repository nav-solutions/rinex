use crate::tests::{
    prod::testbench,
    toolkit::{generic_rinex_comparison, random_name},
};

use std::path::Path;

#[test]
fn obs_v2() {
    let prefix = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("OBS")
        .join("V2");

    // does not work well on very old rinex like V2/KOSG..
    for file in [
        "AJAC3550.21O",
        "aopr0010.17o",
        "barq071q.19o",
        "delf0010.21o",
        "npaz3550.21o",
        "rovn0010.21o",
        "wsra0010.21o",
        "zegv0010.21o",
    ] {
        let path = prefix.to_path_buf().join(file);

        let fullpath = path.to_string_lossy();
        testbench(fullpath.as_ref());
    }
}

#[test]
fn obs_v3() {
    let folder = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/OBS/V3/";
    for file in std::fs::read_dir(folder).unwrap() {
        let fp = file.unwrap();
        let fp = fp.path();
        let fp_str = fp.to_string_lossy().to_string();

        // skipping a few files: although formatting looks very nice
        // all of those were encoded by receivers running some sort of software

        // for this one: test does work, but our verification method is incorrect
        // OBS RINEX garantees epoch up to 1e-3s, while we seem to test strict Eq,
        // which is 1e-9 in hifitime
        if fp_str.ends_with("240506_glacier_station.obs.gz") {
            continue;
        }

        // Same thing, receiver encoded, with weirdly rounded epochs/timestamps
        // and we are too strict at verification
        if fp_str.ends_with("gps_10MSps.23O.gz") {
            continue;
        }
        if fp_str.ends_with("GEOP092I.24o.gz") {
            continue;
        }
        if fp_str.ends_with("gps.23O.gz") {
            continue;
        }

        testbench(fp.to_str().unwrap());
    }
}
