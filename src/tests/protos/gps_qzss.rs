use crate::{
    navigation::{Ephemeris, GpsQzssEphemerisTelemetries, OrbitItem},
    prelude::{Constellation, Rinex, SV},
    tests::toolkit::nav::panic_eq_ephemeris,
};

use log::debug;

use std::collections::HashMap;

use gnss_protos::GpsQzssTelemetry;

#[test]
fn test_lnav_v2() {
    let mut count = 0;

    let file = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/NAV/V2/cbw10010.21n.gz";

    let rinex = Rinex::from_gzip_file(&file).unwrap();

    for (key, eph) in rinex.nav_ephemeris_frames_iter() {
        let tlm = GpsQzssEphemerisTelemetries {
            tlm_1: GpsQzssTelemetry {
                message: 0,
                integrity: false,
                reserved_bit: false,
            },
            tlm_2: GpsQzssTelemetry {
                message: 1,
                integrity: false,
                reserved_bit: false,
            },
            tlm_3: GpsQzssTelemetry {
                message: 2,
                integrity: false,
                reserved_bit: false,
            },
        };

        let burst = eph
            .to_gps_qzss_eph_message(key.sv, key.epoch, false, false, tlm)
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to format GPS subframe burst for {} @ {}",
                    key.sv, key.epoch
                );
            });

        println!("EPH_1 {:?}", burst.eph_1);
        println!("EPH_2 {:?}", burst.eph_2);
        println!("EPH_3 {:?}", burst.eph_3);

        let (toc, reciprocal) =
            Ephemeris::from_gps_qzss_eph_message(key.epoch, key.sv, burst).unwrap();

        assert_eq!(toc, key.epoch, "ToC non reciprocity");

        panic_eq_ephemeris(&reciprocal, eph);

        count += 1;
    }
    assert_eq!(count, 6);
}
