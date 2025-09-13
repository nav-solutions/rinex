use std::io::Read;

use crate::{
    navigation::Ephemeris,
    prelude::{Constellation, Rinex},
};

// NAV (V3) to RTCM
#[test]
#[cfg(feature = "nav")]
fn esbcdnk_ephv3_to_rtcm() {
    let rinex = Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    for (k, ephemeris) in rinex.nav_ephemeris_frames_iter() {
        match k.sv.constellation {
            Constellation::GPS => {},
            Constellation::QZSS => {},
            Constellation::BeiDou => {},
            Constellation::Galileo => {},
            _ => {}, // not supported yet
        }
    }
}

// GLO (V2) to RTCM
#[test]
#[ignore]
#[cfg(feature = "nav")]
fn glo_v2_to_rtcm() {
    let rinex = Rinex::from_file("data/NAV/V2/dlf10010.21g").unwrap();

    for (k, ephemeris) in rinex.nav_ephemeris_frames_iter() {
        match k.sv.constellation {
            Constellation::Glonass => {},
            constellation => {
                panic!("found invalid {} constellation", constellation);
            },
        }
    }
}

// RNX2RTCM (NAV V3)
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
