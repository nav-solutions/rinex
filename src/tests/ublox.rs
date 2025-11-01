use std::io::Read;

use crate::{
    navigation::Ephemeris,
    prelude::{Constellation, Rinex},
};

use ublox::{MgaBdsEphRef, MgaGloEphRef, MgaGpsEphRef, PacketRef, Parser};

// MGA-EPH-XXX
#[test]
#[cfg(feature = "nav")]
fn esbcdnk_ephv3_to_ubx_mga() {
    let mut gps = 0;
    let mut gal = 0;
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

                            assert_eq!(decoded_sv, k.sv);

                            // TODO: testbench
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

                            assert_eq!(decoded_sv, k.sv);

                            // TODO: testbench
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

                            assert_eq!(decoded_sv, k.sv);

                            // TODO: testbench
                            // assert_eq!(decoded_eph, ephemeris.clone());

                            bds += 1;
                        },
                        _ => panic!("{}({}) did not encode a UBX-MGA-BDS frame", k.epoch, k.sv),
                    }
                }
            },
            Constellation::Galileo => {
                if let Some(mga_bytes) = ephemeris.to_ubx_mga_gal(k.epoch, k.sv) {
                    let mut it = ubx_parser.consume_ubx(&mga_bytes);

                    let ubx_frame = it.next().unwrap_or_else(|| {
                        panic!("{}({}) did not encode a valid UBX frame", k.epoch, k.sv);
                    });

                    match ubx_frame {
                        Ok(PacketRef::MgaGalEph(encoded)) => {
                            // run mirror OP
                            let (decoded_sv, decoded_eph) =
                                Ephemeris::from_ubx_mga_gal(k.epoch, encoded);

                            assert_eq!(decoded_sv, k.sv);

                            // TODO: testbench
                            // assert_eq!(decoded_eph, ephemeris.clone());

                            gal += 1;
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
    assert!(gal > 0);
    assert!(qzss > 0);

    assert_eq!(gps, 253);
    println!("UBX-MGA-EPH: {:4} GPS frames", gps);

    assert_eq!(qzss, 15);
    println!("UBX-MGA-EPH: {:4} QZSS frames", qzss);

    assert_eq!(bds, 353);
    println!("UBX-MGA-BDS: {:4} BDS frames", bds);

    assert_eq!(gal, 806);
    println!("UBX-MGA-GAL: {:4} GAL frames", gal);
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

// UBX2RINEX (NAV V3)
#[test]
#[ignore]
fn esbcdnk_nav3_to_ubx() {
    let mut total_msg = 0;
    let mut total_size = 0;
    let mut total_mga_gps_eph = 0;
    let mut total_mga_bds_eph = 0;
    let mut total_mga_gal_eph = 0;
    let mut total_mga_glo_eph = 0;

    let mut buffer = [0; 2048];

    let rinex = Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    let mut streamer = rinex.rnx2ubx();

    let mut parser = Parser::default(); // tester

    loop {
        match streamer.read(&mut buffer) {
            Ok(0) => {
                break;
            },
            Ok(size) => {
                total_size += size;
                let mut iter = parser.consume_ubx(&buffer);

                loop {
                    match iter.next() {
                        Some(message) => match message {
                            Ok(packet) => match packet {
                                PacketRef::MgaGpsEph(_) => {
                                    total_mga_gps_eph += 1;
                                },
                                PacketRef::MgaBdsEph(_) => {
                                    total_mga_bds_eph += 1;
                                },
                                PacketRef::MgaGalEph(_) => {
                                    total_mga_gal_eph += 1;
                                },
                                PacketRef::MgaGloEph(_) => {
                                    total_mga_glo_eph += 1;
                                },
                                msg => {
                                    panic!("unexpected UBX message found: {:?}", msg);
                                },
                            },
                            Err(e) => {
                                panic!("invalid UBX content identified: {}", e);
                            },
                        },
                        None => break,
                    }
                    total_msg += 1;
                }
            },
            Err(_) => {},
        }
    }

    assert!(total_size > 0);
    assert!(total_msg > 0);

    // assert_eq!(total_mga_gps_eph, 253 + 15); // TODO: this fails, should be GPS+QZSS from test #1
    assert_eq!(total_mga_bds_eph, 360);
    // assert_eq!(total_mga_gal_eph, 806);

    println!("ESCDNK-NAV (V3): {:8} bytes", total_size);
    println!("ESCDNK-NAV (V3): {:8} messages", total_msg);
    println!(
        "ESCDNK-NAV (V3): {:8} MGA-GPS-EPH frames",
        total_mga_gps_eph
    );
    println!(
        "ESCDNK-NAV (V3): {:8} MGA-GAL-EPH frames",
        total_mga_glo_eph
    );
    println!(
        "ESCDNK-NAV (V3): {:8} MGA-BDS-EPH frames",
        total_mga_bds_eph
    );
    println!(
        "ESCDNK-NAV (V3): {:8} MGA-GLO-EPH frames",
        total_mga_glo_eph
    );
}

// MGA-TIM-XXX
#[test]
fn esbcdnk_timv4_to_ubx_mga() {
    let mut total_msg = 0;
    let mut total_size = 0;

    let mut buffer = [0; 2048];

    let rinex = Rinex::from_gzip_file("data/NAV/V4/BRD400DLR_S_20230710000_01D_MN.rnx.gz").unwrap();

    for (k, time_offset) in rinex.nav_system_time_frames_iter() {
        match k.sv.constellation {
            Constellation::GPS => {},
            Constellation::Galileo => {},
            _ => {},
        }
    }
}
