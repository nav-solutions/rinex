use crate::{
    navigation::{Ephemeris, OrbitItem},
    prelude::{Constellation, Rinex, SV},
    tests::toolkit::nav::panic_eq_ephemeris,
};

use std::collections::HashMap;

#[test]
fn test_lnav_v2() {
    let mut count = 0;

    let file = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/NAV/V2/cbw10010.21n.gz";

    let rinex = Rinex::from_gzip_file(&file).unwrap();

    for (key, eph) in rinex.nav_ephemeris_frames_iter() {
        let burst = eph.to_gps_qzss_subframes(key.sv).unwrap_or_else(|e| {
            panic!(
                "Failed to format GPS subframe burst for {} @ {}",
                key.sv, key.epoch
            );
        });

        let reciprocal = Ephemeris::from_gps_qzss_subframes(key.sv, key.epoch, burst).unwrap();

        panic_eq_ephemeris(&reciprocal, eph);

        count += 1;
    }
    assert_eq!(count, 6);
}
