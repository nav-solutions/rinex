//! Cross-family selection boundaries using real records and labeled synthetic changes.
#![cfg(feature = "nav")]
use rinex::{
    navigation::{
        rinex::{
            legacy_kepler::KeplerStateError,
            selection::{NavRejection, UnknownHealthPolicy},
        },
        NavMessageType,
    },
    prelude::{Duration, Rinex, SV},
};
use std::str::FromStr;

#[test]
fn kepler_clock_epoch_outside_its_message_window_rejects_selection() {
    // Real orbit payloads with synthetic ToC changes. Keep ToE at the query
    // time so only the independent clock-reference gate can reject them.
    for (fixture, sv_text, half_seconds) in [
        ("tests/fixtures/nav_gps_g02_lnav_2022159.rnx", "G02", 7200.0),
        ("tests/fixtures/nav_qzss_j02_2023071.rnx", "J02", 3600.0),
        ("tests/fixtures/nav_navic_i02_2023071.rnx", "I02", 7200.0),
        ("tests/fixtures/nav_legacy_kms_2022159.rnx", "E08", 10800.0),
        (
            "tests/fixtures/nav_galileo_e08_fnav_2024128.rnx",
            "E08",
            10800.0,
        ),
        ("tests/fixtures/nav_legacy_kms_2022159.rnx", "C10", 21600.0),
        ("tests/fixtures/nav_legacy_kms_2022159.rnx", "C20", 21600.0),
        ("tests/fixtures/nav_bds_geo_c05_2022159.rnx", "C05", 21600.0),
    ] {
        let mut nav = Rinex::from_file(fixture).unwrap();
        let sv = SV::from_str(sv_text).unwrap();
        nav.record
            .as_mut_nav()
            .unwrap()
            .retain(|key, _| key.sv == sv);
        let (key, frame) = nav.record.as_mut_nav().unwrap().pop_first().unwrap();
        let toe = frame.as_ephemeris().unwrap().toe(sv).unwrap();
        let mut stale_clock = key;
        stale_clock.epoch = toe + Duration::from_seconds(half_seconds);
        nav.record.as_mut_nav().unwrap().clear();
        nav.record.as_mut_nav().unwrap().insert(stale_clock, frame);
        let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
        assert!(report.chosen().is_none(), "{sv_text}");
        assert_eq!(
            report.candidates[0].rejection,
            Some(NavRejection::OutOfValidity),
            "{sv_text}"
        );
    }
}

#[test]
fn selected_kepler_candidate_rechecks_clock_epoch_at_a_new_query_time() {
    let nav = Rinex::from_file("tests/fixtures/nav_gps_g02_lnav_2022159.rnx").unwrap();
    let sv = SV::from_str("G02").unwrap();
    let (key, eph) = nav.nav_ephemeris_frames_iter().next().unwrap();
    let toe = eph.toe(sv).unwrap();
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    let mut candidate = *report.chosen().unwrap();
    candidate.clock_reference = key.epoch - Duration::from_seconds(7200.0);
    assert!(matches!(
        candidate.kepler_state_at(toe),
        Err(KeplerStateError::OutOfValidity)
    ));
}

#[test]
fn nearer_but_invalid_kepler_record_does_not_displace_a_usable_one() {
    let mut nav = Rinex::from_file("tests/fixtures/nav_legacy_kms_2022159.rnx").unwrap();
    let sv = SV::from_str("C20").unwrap();
    nav.record
        .as_mut_nav()
        .unwrap()
        .retain(|key, _| key.sv == sv);
    let (key, frame) = nav
        .record
        .as_nav()
        .unwrap()
        .iter()
        .next()
        .map(|(key, frame)| (*key, frame.clone()))
        .unwrap();
    let toe = frame.as_ephemeris().unwrap().toe(sv).unwrap();
    let t = toe + Duration::from_seconds(300.0);
    let mut nearer = key;
    nearer.epoch = t;
    let mut bad = frame;
    bad.as_mut_ephemeris()
        .unwrap()
        .orbits
        .insert("e".into(), 1.0.into());
    nav.record.as_mut_nav().unwrap().insert(nearer, bad);
    let selection = nav.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
    assert_eq!(selection.chosen().unwrap().key, &key);
    assert!(selection
        .candidates
        .iter()
        .any(|candidate| candidate.key == &nearer
            && candidate.rejection == Some(NavRejection::Unpropagatable)));
}

#[test]
fn beidou_lnav_and_wrong_geo_message_are_explicitly_unsupported() {
    let mut nav = Rinex::from_file("tests/fixtures/nav_legacy_kms_2022159.rnx").unwrap();
    let sv = SV::from_str("C20").unwrap();
    nav.record
        .as_mut_nav()
        .unwrap()
        .retain(|key, _| key.sv == sv);
    let (mut key, frame) = nav.record.as_mut_nav().unwrap().pop_first().unwrap();
    let t = frame.as_ephemeris().unwrap().toe(sv).unwrap();
    key.msgtype = NavMessageType::LNAV;
    nav.record.as_mut_nav().unwrap().insert(key, frame);
    let report = nav.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
    assert!(report.chosen().is_none());
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::UnsupportedMessage)
    );

    let mut geo = Rinex::from_file("tests/fixtures/nav_bds_geo_c05_2022159.rnx").unwrap();
    let sv = SV::from_str("C05").unwrap();
    let (mut key, frame) = geo.record.as_mut_nav().unwrap().pop_first().unwrap();
    let t = frame.as_ephemeris().unwrap().toe(sv).unwrap();
    key.msgtype = NavMessageType::D1;
    geo.record.as_mut_nav().unwrap().insert(key, frame);
    let report = geo.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
    assert!(report.chosen().is_none());
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::UnsupportedMessage)
    );
}

#[test]
fn v4_only_message_does_not_gain_support_in_a_v3_header() {
    let mut nav = Rinex::from_file("tests/fixtures/nav_legacy_kms_2022159.rnx").unwrap();
    let sv = SV::from_str("E08").unwrap();
    nav.record
        .as_mut_nav()
        .unwrap()
        .retain(|key, _| key.sv == sv);
    let (key, eph) = nav.nav_ephemeris_frames_iter().next().unwrap();
    let t = eph.toe(sv).unwrap();
    assert!(nav
        .nav_select_ephemeris(key.sv, t, UnknownHealthPolicy::Reject)
        .chosen()
        .is_some());
    nav.header.version.major = 3; // Synthetic header change; payload remains V4.
    let report = nav.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
    assert!(report.chosen().is_none());
    assert!(report
        .candidates
        .iter()
        .all(|candidate| candidate.rejection == Some(NavRejection::UnsupportedMessage)));
}
