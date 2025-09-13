use crate::{
    navigation::Ephemeris,
    prelude::{Constellation, Rinex},
};

// NAV (V3) to RTCM
#[test]
#[cfg(feature = "nav")]
fn esbcdnk_ephv3_to_rtcm() {
    let mut gps1019 = 0;
    let mut glo1020 = 0;
    let mut bds1042 = 0;
    let mut qzss1044 = 0;
    let mut gal1045 = 0;

    let rinex = Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    for (k, ephemeris) in rinex.nav_ephemeris_frames_iter() {
        match k.sv.constellation {
            Constellation::GPS => {
                if let Some(msg) = ephemeris.to_rtcm_gps1019(k.epoch, k.sv) {
                    gps1019 += 1;
                }
            },
            Constellation::QZSS => {
                if let Some(msg) = ephemeris.to_rtcm_qzss1044(k.epoch, k.sv) {
                    qzss1044 += 1;
                }
            },
            Constellation::BeiDou => {
                if let Some(msg) = ephemeris.to_rtcm_bds1042(k.epoch, k.sv) {
                    bds1042 += 1;
                }
            },
            Constellation::Galileo => {
                if let Some(msg) = ephemeris.to_rtcm_gal1045(k.epoch, k.sv) {
                    gal1045 += 1;
                }
            },
            Constellation::Glonass => {
                if let Some(msg) = ephemeris.to_rtcm_glo1020(k.epoch, k.sv) {
                    glo1020 += 1;
                }
            },
            _ => {}, // not supported yet
        }
    }

    assert!(gps1019 > 0);
    // assert!(glo1020 > 0); // TODO
    assert!(bds1042 > 0);
    assert!(qzss1044 > 0);
    assert!(gal1045 > 0);

    assert_eq!(gps1019, 253);
    // assert_eq!(glo1020, 0);
    assert_eq!(gal1045, 806);
    assert_eq!(bds1042, 353);
    assert_eq!(qzss1044, 15);
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
}
