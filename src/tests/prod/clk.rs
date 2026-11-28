use crate::tests::prod::testbench;

/// Parses entire CLK/V2 repo and runs testbench
#[test]
#[ignore]
fn clocks_v2() {
    let folder = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/CLK/V2/";
    for file in std::fs::read_dir(folder).unwrap() {
        let fp = file.unwrap();
        let fp = fp.path();
        testbench(fp.to_str().unwrap());
    }
}
