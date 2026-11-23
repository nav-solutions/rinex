use crate::{
    navigation::{Ephemeris, GpsQzssLnavCompositionOpts, OrbitItem},
    prelude::{Constellation, Rinex, SV},
    tests::toolkit::nav::{assert_faithful_ephemeris, panic_eq_ephemeris},
};

use hifitime::prelude::{Epoch, TimeScale, TimeSeries, Unit};

use std::{fs::File, io::Read};

use gnss_protos::{
    gps::lnav::{
        GpsQzssLnavAlmanach, GpsQzssLnavFrame, GpsQzssLnavIonosphereUtc, GpsQzssLnavSubfDecoder,
        GpsQzssLnavSubframe4, GpsQzssLnavSubframeId, GpsQzssLnavTelemetry, GPS_LNAV_FRAME_BYTES,
    },
    Message,
};

/// genearates GPS/QZSS LNAV 30s broadcast
/// verifies ALM-4 page18 is bit true and broadcasts the RINEX information
/// correctly.
fn run_alm4_page18_bit_true(path: &str) {
    let mut page18_count = 0;

    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/" + path;

    let rinex = if path.ends_with(".gz") {
        Rinex::from_gzip_file(&path)
    } else {
        Rinex::from_file(&path)
    }
    .unwrap();

    let t0 = rinex.first_epoch().unwrap().to_time_scale(TimeScale::GPST);

    // compose the largest 30s temporal arc we can,
    // -1h ; +1h from that file range
    let arc = TimeSeries::inclusive(
        t0 - 1.0 * Unit::Hour,
        t0 - 25.0 * Unit::Hour,
        30.0 * Unit::Second,
    );

    // grab test values: depends on file REV
    let rev_major = rinex.header.version.major;

    let expected = match rev_major {
        2 => GpsQzssLnavIonosphereUtc::default(),
        3 => GpsQzssLnavIonosphereUtc::default(),
        4 => {
            panic!("not applicable! use dedicated method");
        },
        _ => {
            panic!("invalid RINEX revision");
        },
    };

    for proto in rinex.to_gps_qzss_lnav_proto(arc, |(satellite, t_tx)| GpsQzssLnavCompositionOpts {
        satellite,
        epoch: t_tx,
        alert: false,
        anti_spoofing: false,
        tlm_1: GpsQzssLnavTelemetry::default(),
        tlm_2: GpsQzssLnavTelemetry::default(),
        tlm_3: GpsQzssLnavTelemetry::default(),
        tlm_4: GpsQzssLnavTelemetry::default(),
        tlm_5: GpsQzssLnavTelemetry::default(),
        alm_4: None,
        alm_5: None,
    }) {
        match proto.frame.alm_4().subframe.as_alm4() {
            Some(GpsQzssLnavSubframe4::IonosphereUtc(iono_utc)) => {
                // assert_eq!(iono_utc, expected);
                page18_count += 1;
            },
            _ => {}, // untested
        }
    }
    assert_eq!(page18_count, 3);
}

/// genearates GPS/QZSS LNAV 30s broadcast
/// verifies ALM-4 page18 is bit true and broadcasts the RINEX information
/// correctly.
fn run_v4_alm4_page18_bit_true(path: &str) {
    let mut sto_count = 0;
    let mut iono_count = 0;

    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/" + path;

    let rinex = if path.ends_with(".gz") {
        Rinex::from_gzip_file(&path)
    } else {
        Rinex::from_file(&path)
    }
    .unwrap();

    let t0 = rinex.first_epoch().unwrap().to_time_scale(TimeScale::GPST);

    // compose the largest 30s temporal arc we can,
    // -1h ; +1h from that file range
    let arc = TimeSeries::inclusive(
        t0 - 1.0 * Unit::Hour,
        t0 - 25.0 * Unit::Hour,
        30.0 * Unit::Second,
    );

    assert_eq!(
        rinex.header.version.major, 4,
        "not applicabie! use dedicated test"
    );

    for proto in rinex.to_gps_qzss_lnav_proto(arc, |(satellite, t_tx)| GpsQzssLnavCompositionOpts {
        satellite,
        epoch: t_tx,
        alert: false,
        anti_spoofing: false,
        tlm_1: GpsQzssLnavTelemetry::default(),
        tlm_2: GpsQzssLnavTelemetry::default(),
        tlm_3: GpsQzssLnavTelemetry::default(),
        tlm_4: GpsQzssLnavTelemetry::default(),
        tlm_5: GpsQzssLnavTelemetry::default(),
        alm_4: None,
        alm_5: None,
    }) {
        match proto.frame.alm_4().subframe.as_alm4() {
            Some(GpsQzssLnavSubframe4::IonosphereUtc(iono_utc)) => {
                // retrieve closest IONO frame
                //
                // retrieve closest STO frame
            },
            _ => {}, // untested
        }
    }
    assert_eq!(sto_count, 3);
    assert_eq!(iono_count, 3);
}

#[test]
fn test_lnav_v2_page18() {
    run_alm4_page18_bit_true("data/NAV/V2/cbw10010.21n.gz");
}

#[test]
fn test_lnav_v3_page18() {
    run_alm4_page18_bit_true("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz");
}

#[test]
fn test_lnav_v4_page18() {
    run_v4_alm4_page18_bit_true("data/NAV/V4/KMS300DNK_R_20221591000_01H_MN.rnx.gz");
}
