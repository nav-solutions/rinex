//! Navigation zero, blank and invalid orbit fields.
use super::{navigation_rinex, parse, GLO_BLOCK, GPS_BLOCK};
use crate::{
    navigation::{NavMessageType, OrbitItem},
    prelude::*,
};
use std::str::FromStr;

#[test]
fn real_nav_keeps_zero_and_blank_distinct() {
    let rnx = parse(&navigation_rinex("4.00", GLO_BLOCK)).unwrap();
    let nav = rnx.record.as_nav().unwrap();
    assert_eq!(nav.len(), 1);
    let (key, frame) = nav.first_key_value().unwrap();
    assert_eq!(
        key.epoch,
        Epoch::from_str("2024-05-10T02:45:00 UTC").unwrap()
    );
    let eph = frame.as_ephemeris().unwrap();
    assert_eq!(eph.clock_drift_rate, 441_000.0);
    assert_eq!(eph.orbits.get("accelY"), Some(&OrbitItem::F64(0.0)));
    assert_eq!(eph.orbits.get("ageOp"), Some(&OrbitItem::F64(0.0)));
    assert_eq!(eph.orbits.get("channel"), Some(&OrbitItem::I8(-1)));
    let rnx = parse(&navigation_rinex("4.02", GPS_BLOCK)).unwrap();
    let nav = rnx.record.as_nav().unwrap();
    let eph = nav.values().find_map(|f| f.as_ephemeris()).unwrap();
    assert_eq!(eph.orbits.get("fitInt"), Some(&OrbitItem::F64(4.0)));
    // Explicitly modified source: an omitted optional fit interval remains absent.
    let blank = GPS_BLOCK.replacen(" 4.000000000000E+00", "                   ", 1);
    let rnx = parse(&navigation_rinex("4.02", &blank)).unwrap();
    let eph = rnx
        .record
        .as_nav()
        .unwrap()
        .values()
        .find_map(|f| f.as_ephemeris())
        .unwrap();
    assert!(eph.orbits.get("fitInt").is_none());
}

#[test]
fn synthetic_nav_native_integer_zero_is_a_value() {
    for (kind, expected) in [
        ("u8", OrbitItem::U8(0)),
        ("i8", OrbitItem::I8(0)),
        ("u32", OrbitItem::U32(0)),
        ("f64", OrbitItem::F64(0.0)),
    ] {
        assert_eq!(
            OrbitItem::new(
                "test",
                kind,
                "0.0",
                &NavMessageType::LNAV,
                Constellation::GPS
            )
            .unwrap(),
            expected
        );
        assert!(
            OrbitItem::new("test", kind, "", &NavMessageType::LNAV, Constellation::GPS).is_err()
        );
    }
}

#[test]
fn synthetic_zero_fit_interval_is_present() {
    let modified = GPS_BLOCK.replacen(" 4.000000000000E+00", " 0.000000000000E+00", 1);
    let rnx = parse(&navigation_rinex("4.02", &modified)).unwrap();
    let eph = rnx
        .record
        .as_nav()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .as_ephemeris()
        .unwrap();
    assert_eq!(eph.orbits.get("fitInt"), Some(&OrbitItem::F64(0.0)));
}

#[test]
fn synthetic_nav_invalid_orbit_field_keeps_field_and_cause() {
    let header = Header::basic_nav().with_version(Version::new(4, 2));
    for token in ["BAD", "NaN", "inf"] {
        let modified = GPS_BLOCK.replacen(" 1.100000000000E+02", &format!("{token:>19}"), 1);
        match crate::navigation::parse_epoch(&header, &modified) {
            Err(ParsingError::NavOrbitParsing { field, source }) => {
                assert_eq!(field, "iode");
                assert!(matches!(*source, ParsingError::NavNullOrbit));
            },
            other => panic!("expected invalid orbit field, got {other:?}"),
        }
    }
}
