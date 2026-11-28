use crate::tests::prod::testbench;

/// parses entire NAV/V2 repo and runs the testbench
#[test]
fn nav_v2() {
    let folder = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/NAV/V2/";
    for file in std::fs::read_dir(folder).unwrap() {
        let fp = file.unwrap();
        let fp = fp.path();
        testbench(fp.to_str().unwrap());
    }
}

/// parses entire NAV/V3 repo and runs the testbench
#[test]
fn nav_v3() {
    let folder = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/NAV/V3/";
    for file in std::fs::read_dir(folder).unwrap() {
        let fp = file.unwrap();
        let fp = fp.path();
        testbench(fp.to_str().unwrap());
    }
}

/// parses entire NAV/V4 repo and runs the testbench
#[test]
fn nav_v4() {
    let folder = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/NAV/V4/";
    for file in std::fs::read_dir(folder).unwrap() {
        let fp = file.unwrap();
        let fp = fp.path();
        testbench(fp.to_str().unwrap());
    }
}
