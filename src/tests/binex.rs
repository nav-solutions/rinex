use crate::navigation::Ephemeris;
use crate::prelude::{Constellation, Rinex};

use binex::prelude::Meta;

#[test]
fn esbcdnk_ephv3_binex() {
    let mut gps_passed = 0;
    let mut gal_passed = 0;
    let mut glo_passed = 0;
    // TODO let mut bds_passed = 0;
    // TODO let mut qzss_passed = 0;
    let mut sbas_passed = 0;

    let rinex = Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    for (k, ephemeris) in rinex.nav_ephemeris_frames_iter() {
        match k.sv.constellation {
            Constellation::GPS | Constellation::Galileo => {
                if let Some(serialized) = ephemeris.to_binex(k.epoch, k.sv) {
                    // mirror
                    let (decoded_sv, decoded) = Ephemeris::from_binex(k.epoch, serialized)
                        .unwrap_or_else(|| {
                            panic!("Failed to decoded {}({}) BINEX frame", k.epoch, k.sv);
                        });

                    // testbench
                    assert_eq!(k.sv, decoded_sv, "{}({}) invalid SV", k.epoch, k.sv);

                    // TODO achieve full reciprocity
                    // assert_eq!(
                    //     *ephemeris, decoded,
                    //     "{}({}) invalid content decoded",
                    //     k.epoch, k.sv
                    // );

                    match k.sv.constellation {
                        Constellation::GPS => gps_passed += 1,
                        Constellation::Galileo => gal_passed += 1,
                        Constellation::Glonass => glo_passed += 1,
                        Constellation::BeiDou => {
                            // TODO
                        },
                        Constellation::Glonass => {
                            // TODO: issue with sv.PRN
                        },
                        Constellation::QZSS => {
                            // TODO
                        },
                        _ => {},
                    }
                }
            },
            constellation => {
                if constellation.is_sbas() {
                    if let Some(serialized) = ephemeris.to_binex(k.epoch, k.sv) {
                        // mirror
                        let (decoded_sv, decoded) = Ephemeris::from_binex(k.epoch, serialized)
                            .unwrap_or_else(|| {
                                panic!("Failed to decoded {}({}) BINEX frame", k.epoch, k.sv);
                            });

                        // testbench
                        assert!(decoded_sv.constellation.is_sbas());
                        // TODO error in the SV::new API unable to identify correctly
                        // assert_eq!(decoded_sv.prn, k.sv.prn);
                        // assert_eq!(decoded_sv.constellation, k.sv.constellation);

                        // TODO invalid PRN
                        // assert_eq!(k.sv, decoded_sv, "{}({}) invalid SV", k.epoch, k.sv);

                        // TODO achieve full reciprocity
                        // assert_eq!(
                        //     *ephemeris, decoded,
                        //     "{}({}) invalid content decoded",
                        //     k.epoch, k.sv
                        // );
                        sbas_passed += 1;
                    }
                }
            },
        }
    }

    assert!(gps_passed > 0);
    assert!(gal_passed > 0);
    assert!(sbas_passed > 0);
    // TODO assert!(glo_passed > 0);
    // TODO assert!(bds_passed > 0);
    // TODO assert!(qzss_passed > 0);

    assert_eq!(gps_passed, 253);
    assert_eq!(gal_passed, 806);
    assert_eq!(sbas_passed, 320);
}

#[test]
#[ignore]
fn nav_v3_to_binex() {
    let rinex = Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    let meta = Meta::default();

    let mut streamer = rinex.rnx2bin(meta);

    for message in streamer.iter() {}
}
