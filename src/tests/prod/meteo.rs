use crate::tests::prod::testbench;

/// Parses entire METEO/V2 repo and runs testbench
#[test]
fn meteo_v2() {
    let folder = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/MET/V2/";
    for file in std::fs::read_dir(folder).unwrap() {
        let fp = file.unwrap();
        let fp = fp.path();
        testbench(fp.to_str().unwrap());
    }
}

/// Parses entire METEO/V3 repo and runs testbench
#[test]
fn meteo_v3() {
    let folder = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/MET/V3/";
    for file in std::fs::read_dir(folder).unwrap() {
        let fp = file.unwrap();
        let fp = fp.path();
        testbench(fp.to_str().unwrap());
    }
}

/// Parses entire METEO/V4 repo and runs testbench
#[test]
fn meteo_v4() {
    let folder = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/MET/V4/";
    for file in std::fs::read_dir(folder).unwrap() {
        let fp = file.unwrap();
        let fp = fp.path();
        testbench(fp.to_str().unwrap());
    }
}
