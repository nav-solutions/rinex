//! Public GPS LNAV selection and native WGS-84 state.
//! The G02 record is an unchanged excerpt of the 2022-06-08 Septentrio NAV
//! sample on the earlier NAV branch (full excerpt SHA-256:
//! 579b5da043a98f8027f7c191a5b2f239f033da252b14fe7dcb00622761062425).
//! Expected coordinates were frozen by that branch's independent
//! `tests/reference/nav_legacy.py` before this API was written.
#![cfg(feature = "nav")]

use rinex::{
    navigation::{
        gps::GpsQzssl1l2l5Health,
        rinex::selection::{NativeFrame, NavRejection, StateError, UnknownHealthPolicy},
        NavMessageType, OrbitItem,
    },
    prelude::{Constellation, Duration, Rinex, SV},
};
use std::str::FromStr;

fn sample() -> (Rinex, SV) {
    (
        Rinex::from_file("tests/fixtures/nav_gps_g02_lnav_2022159.rnx").unwrap(),
        SV::from_str("G02").unwrap(),
    )
}

#[test]
fn stale_toc_rejects_gps_selection_and_direct_state() {
    let (mut nav, sv) = sample();
    let (key, frame) = nav.record.as_mut_nav().unwrap().pop_first().unwrap();
    let toe = frame.as_ephemeris().unwrap().toe(sv).unwrap();
    let mut stale_clock = key;
    stale_clock.epoch = toe + Duration::from_seconds(7200.0);
    nav.record.as_mut_nav().unwrap().insert(stale_clock, frame);
    let report = nav.nav_select_gps_lnav(sv, toe, UnknownHealthPolicy::Reject);
    assert!(report.chosen().is_none());
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::OutOfValidity)
    );

    let (nav, sv) = sample();
    let (key, eph) = nav.nav_ephemeris_frames_iter().next().unwrap();
    let toe = eph.toe(sv).unwrap();
    let report = nav.nav_select_gps_lnav(sv, toe, UnknownHealthPolicy::Reject);
    let mut candidate = *report.chosen().unwrap();
    candidate.clock_reference = key.epoch - Duration::from_seconds(7200.0);
    assert!(matches!(
        candidate.native_state_at(toe),
        Err(StateError::OutOfValidity)
    ));
}

#[test]
fn real_g02_lnav_native_position_velocity_and_clock() {
    let (nav, sv) = sample();
    let (key, eph) = nav.nav_ephemeris_frames_iter().next().unwrap();
    let toe = eph.toe(sv).unwrap();
    let t = toe + Duration::from_seconds(300.0);
    let selection = nav.nav_select_gps_lnav(sv, t, UnknownHealthPolicy::Reject);
    let chosen = selection.chosen().unwrap();
    assert_eq!(chosen.key, key);
    assert_eq!(chosen.orbit_reference, Some(toe));
    let state = chosen.native_state_at(t).unwrap();
    assert_eq!(state.frame, NativeFrame::GpsBroadcastWgs84);
    assert_eq!(state.epoch, t);
    let expected_p = [-19807.364277161476, 15714.011524581605, 9032.842588495178];
    let expected_v = [-1.0481002171218279, 0.23196199235826498, -2.898636913378141];
    for axis in 0..3 {
        assert!((state.position_km[axis] - expected_p[axis]).abs() < 1e-6);
        assert!((state.velocity_km_s[axis] - expected_v[axis]).abs() < 1e-7);
    }
    assert!((state.clock_correction_s - (-0.0006528146392490496)).abs() < 1e-12);
}

#[test]
fn nearer_unsupported_message_cannot_displace_gps_lnav() {
    let (mut nav, sv) = sample();
    let (key, frame) = nav
        .record
        .as_nav()
        .unwrap()
        .iter()
        .next()
        .map(|(k, v)| (*k, v.clone()))
        .unwrap();
    let toe = frame.as_ephemeris().unwrap().toe(sv).unwrap();
    let t = toe + Duration::from_seconds(300.0);
    // Synthetic CNAV record at the target time, with the same valid orbit
    // numbers. The message identity alone makes its propagator unsupported.
    let mut nearer = key;
    nearer.epoch = t;
    nearer.msgtype = NavMessageType::CNAV;
    nav.record.as_mut_nav().unwrap().insert(nearer, frame);
    let report = nav.nav_select_gps_lnav(sv, t, UnknownHealthPolicy::Reject);
    assert_eq!(report.chosen().unwrap().key.msgtype, NavMessageType::LNAV);
    assert!(report
        .candidates
        .iter()
        .any(|c| c.key.msgtype == NavMessageType::CNAV
            && c.rejection == Some(NavRejection::UnsupportedMessage)));
}

#[test]
fn missing_field_and_window_boundary_reject() {
    let (mut nav, sv) = sample();
    let (key, eph) = nav.nav_ephemeris_frames_iter().next().unwrap();
    let toe = eph.toe(sv).unwrap();
    let original_key = *key;
    for t in [
        toe + Duration::from_seconds(7200.0),
        toe - Duration::from_seconds(7200.0),
    ] {
        let report = nav.nav_select_gps_lnav(sv, t, UnknownHealthPolicy::Reject);
        assert_eq!(
            report.candidates[0].rejection,
            Some(NavRejection::OutOfValidity)
        );
        assert!(report.chosen().is_none());
    }
    let selected = nav.nav_select_gps_lnav(sv, toe, UnknownHealthPolicy::Reject);
    assert!(matches!(
        selected
            .chosen()
            .unwrap()
            .native_state_at(toe + Duration::from_seconds(7200.0)),
        Err(StateError::OutOfValidity)
    ));
    drop(selected);
    let frame = nav
        .record
        .as_mut_nav()
        .unwrap()
        .get_mut(&original_key)
        .unwrap();
    frame.as_mut_ephemeris().unwrap().orbits.remove("sqrta");
    let report = nav.nav_select_gps_lnav(sv, toe, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::MissingOrbitField)
    );
}

#[test]
fn health_policy_and_invalid_elements_are_explicit() {
    let (mut nav, sv) = sample();
    let (key, eph) = nav.nav_ephemeris_frames_iter().next().unwrap();
    let key = *key;
    let t = eph.toe(sv).unwrap();
    let frame = nav.record.as_mut_nav().unwrap().get_mut(&key).unwrap();
    let eph = frame.as_mut_ephemeris().unwrap();
    eph.orbits.remove("health");
    let rejected = nav.nav_select_gps_lnav(sv, t, UnknownHealthPolicy::Reject);
    assert_eq!(
        rejected.candidates[0].rejection,
        Some(NavRejection::UnknownHealth)
    );
    assert!(nav
        .nav_select_gps_lnav(sv, t, UnknownHealthPolicy::Allow)
        .chosen()
        .is_some());

    let eph = nav
        .record
        .as_mut_nav()
        .unwrap()
        .get_mut(&key)
        .unwrap()
        .as_mut_ephemeris()
        .unwrap();
    eph.orbits.insert("e".to_string(), 1.0.into());
    let invalid = nav.nav_select_gps_lnav(sv, t, UnknownHealthPolicy::Allow);
    assert_eq!(
        invalid.candidates[0].rejection,
        Some(NavRejection::MissingOrbitField)
    );
    assert!(invalid.chosen().is_none());

    let eph = nav
        .record
        .as_mut_nav()
        .unwrap()
        .get_mut(&key)
        .unwrap()
        .as_mut_ephemeris()
        .unwrap();
    eph.orbits.insert("e".to_string(), 0.02.into());
    eph.orbits.insert(
        "health".to_string(),
        OrbitItem::GpsQzssl1l2l5Health(GpsQzssl1l2l5Health::from(1)),
    );
    let unhealthy = nav.nav_select_gps_lnav(sv, t, UnknownHealthPolicy::Allow);
    assert_eq!(
        unhealthy.candidates[0].rejection,
        Some(NavRejection::Unhealthy)
    );
}

#[test]
fn gps_v2_and_v3_real_files_select_and_propagate() {
    for file in [
        "data/NAV/V2/cbw10010.21n.gz",
        "data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz",
    ] {
        let nav = Rinex::from_gzip_file(file).unwrap();
        let (key, eph) = nav
            .nav_ephemeris_frames_iter()
            .find(|(key, _)| {
                key.sv.constellation == Constellation::GPS && key.msgtype == NavMessageType::LNAV
            })
            .unwrap();
        let t = eph.toe(key.sv).unwrap();
        let selection = nav.nav_select_gps_lnav(key.sv, t, UnknownHealthPolicy::Reject);
        let state = selection.chosen().unwrap().native_state_at(t).unwrap();
        assert_eq!(state.frame, NativeFrame::GpsBroadcastWgs84);
        assert!(state
            .position_km
            .iter()
            .chain(state.velocity_km_s.iter())
            .all(|v| v.is_finite()));
    }
}
