use crate::{
    navigation::Ephemeris,
    prelude::{Constellation, Rinex},
};

use ublox::{MgaBdsEphRef, MgaGloEphRef, MgaGpsEphRef, PacketRef, Parser};

// MGA-EPH-XXX
#[test]
#[cfg(feature = "nav")]
fn nav_v3_to_ubx_mga() {
    let mut gps = 0;
    let mut bds = 0;
    let mut qzss = 0;

    let mut ubx_parser: Parser<Vec<u8>> = Parser::default();

    let rinex = Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    for (k, ephemeris) in rinex.nav_ephemeris_frames_iter() {
        match k.sv.constellation {
            Constellation::GPS => {
                if let Some(mga_bytes) = ephemeris.to_ubx_mga_gps_qzss(k.epoch, k.sv) {
                    // parse back
                    let mut it = ubx_parser.consume_ubx(&mga_bytes);

                    let ubx_frame = it.next().unwrap_or_else(|| {
                        panic!("{}({}) did not encode a valid UBX frame", k.epoch, k.sv);
                    });

                    match ubx_frame {
                        Ok(PacketRef::MgaGpsEph(encoded)) => {
                            // run mirror OP
                            let (decoded_sv, decoded_eph) =
                                Ephemeris::from_ubx_mga_gps(k.epoch, encoded);

                            // TODO: testbench
                            // assert_eq!(decoded_sv, k.sv);
                            // assert_eq!(decoded_eph, ephemeris.clone());
                            gps += 1;
                        },
                        _ => panic!("{}({}) did not encode a UBX-MGA-GPS frame", k.epoch, k.sv),
                    }
                }
            },
            Constellation::QZSS => {
                if let Some(mga_bytes) = ephemeris.to_ubx_mga_gps_qzss(k.epoch, k.sv) {
                    let mut it = ubx_parser.consume_ubx(&mga_bytes);

                    let ubx_frame = it.next().unwrap_or_else(|| {
                        panic!("{}({}) did not encode a valid UBX frame", k.epoch, k.sv);
                    });

                    match ubx_frame {
                        Ok(PacketRef::MgaGpsEph(encoded)) => {
                            // run mirror OP
                            let (decoded_sv, decoded_eph) =
                                Ephemeris::from_ubx_mga_qzss(k.epoch, encoded);

                            // TODO: testbench
                            // assert_eq!(decoded_sv, k.sv);
                            // assert_eq!(decoded_eph, ephemeris.clone());
                            qzss += 1;
                        },
                        _ => panic!("{}({}) did not encode a UBX-MGA-QZSS frame", k.epoch, k.sv),
                    }
                }
            },
            Constellation::BeiDou => {
                if let Some(mga_bytes) = ephemeris.to_ubx_mga_bds(k.epoch, k.sv) {
                    let mut it = ubx_parser.consume_ubx(&mga_bytes);

                    let ubx_frame = it.next().unwrap_or_else(|| {
                        panic!("{}({}) did not encode a valid UBX frame", k.epoch, k.sv);
                    });

                    match ubx_frame {
                        Ok(PacketRef::MgaBdsEph(encoded)) => {
                            // run mirror OP
                            let (decoded_sv, decoded_eph) =
                                Ephemeris::from_ubx_mga_bds(k.epoch, encoded);

                            // TODO: testbench
                            // assert_eq!(decoded_sv, k.sv);
                            // assert_eq!(decoded_eph, ephemeris.clone());
                            bds += 1;
                        },
                        _ => panic!("{}({}) did not encode a UBX-MGA-BDS frame", k.epoch, k.sv),
                    }
                }
            },
            _ => {}, // not supported yet
        }
    }

    assert!(gps > 0);
    assert!(bds > 0);
    assert!(qzss > 0);

    assert_eq!(gps, 253);
    println!("UBX-MGA-EPH: {:4} GPS frames", gps);

    assert_eq!(qzss, 15);
    println!("UBX-MGA-EPH: {:4} QZSS frames", qzss);

    assert_eq!(bds, 353);
    println!("UBX-MGA-BDS: {:4} BDS frames", bds);
}

// MGA-EPH-GLO
#[test]
#[ignore]
#[cfg(feature = "nav")]
fn glo_v2_to_ubx_mga() {
    let mut glo = 0;

    let mut ubx_parser: Parser<Vec<u8>> = Parser::default();

    let rinex = Rinex::from_file("data/NAV/V2/dlf10010.21g").unwrap();

    for (k, ephemeris) in rinex.nav_ephemeris_frames_iter() {
        match k.sv.constellation {
            Constellation::Glonass => {
                if let Some(mga_bytes) = ephemeris.to_ubx_mga_bds(k.epoch, k.sv) {
                    let mut it = ubx_parser.consume_ubx(&mga_bytes);

                    let ubx_frame = it.next().unwrap_or_else(|| {
                        panic!("{}({}) did not encode a valid UBX frame", k.epoch, k.sv);
                    });

                    match ubx_frame {
                        Ok(PacketRef::MgaGloEph(encoded)) => {
                            // could go even further matching all data fields

                            glo += 1;
                        },
                        _ => panic!("{}({}) did not encode a UBX-MGA-BDS frame", k.epoch, k.sv),
                    }
                }
            },
            constellation => {
                panic!("found invalid {} constellation", constellation);
            },
        }
    }

    assert!(glo > 0);
    assert_eq!(glo, 253);
    println!("UBX-MGA-EPH: {:4} GLO frames", glo);
}
