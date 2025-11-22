use crate::{prelude::Rinex, tests::init_logger};

#[test]
fn gh397_issue() {
    init_logger();

    let rnx =
        Rinex::from_gzip_file("data/CRNX/V3/CIBG00IDN_R_20240100000_01D_30S_MO.crx.gz").unwrap();

    let epochs: Vec<_> = rnx
        .signal_observations_iter()
        .map(|(key, _)| key.epoch)
        .collect();

    println!("Total epochs: {}", epochs.len());
    println!("First epoch: {}", epochs.first().unwrap());
    println!("Last epoch: {}", epochs.last().unwrap());
}
