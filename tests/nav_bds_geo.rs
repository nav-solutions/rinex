//! BeiDou D2 GEO against a fixed, independently calculated RINEX record.
#![cfg(feature = "nav")]
use rinex::{
    navigation::{
        rinex::{
            legacy_kepler::KeplerStateError,
            selection::{NativeFrame, NavRejection, UnknownHealthPolicy},
        },
        NavMessageType,
    },
    prelude::{Duration, Rinex, SV},
};
use serde_json::Value;
use std::str::FromStr;

const FIXTURE: &str = "tests/fixtures/nav_bds_geo_c05_2022159.rnx";
const EXPECTED: &str = include_str!("reference/nav_bds_geo_expected.json");

fn record() -> (Rinex, SV) {
    let nav = Rinex::from_file(FIXTURE).unwrap();
    let sv = SV::from_str("C05").unwrap();
    assert_eq!(nav.nav_ephemeris_frames_iter().count(), 1);
    (nav, sv)
}

fn number(value: &Value) -> f64 {
    value.as_f64().unwrap()
}

#[test]
fn real_c05_d2_geo_position_and_velocity_match_reference() {
    let (nav, sv) = record();
    let (key, eph) = nav.nav_ephemeris_frames_iter().next().unwrap();
    assert_eq!(key.msgtype, NavMessageType::D2);
    let toe = eph.toe(sv).unwrap();
    let expected: Value = serde_json::from_str(EXPECTED).unwrap();
    let mut max_position_km: f64 = 0.0;
    let mut max_velocity_km_s: f64 = 0.0;
    for row in expected.as_array().unwrap() {
        let t = toe + Duration::from_seconds(number(&row["offset_s"]));
        let report = nav.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
        let chosen = report.chosen().unwrap_or_else(|| panic!("{report:?}"));
        assert_eq!(chosen.key, key);
        assert_eq!(chosen.orbit_reference, Some(toe));
        assert_eq!(
            chosen.native_frame,
            Some(NativeFrame::BeiDouBroadcastCgcs2000)
        );
        let state = chosen.kepler_state_at(t).unwrap();
        assert_eq!(state.frame, NativeFrame::BeiDouBroadcastCgcs2000);
        for axis in 0..3 {
            let dp = (state.position_km[axis] - number(&row["position_km"][axis])).abs();
            let dv = (state.velocity_km_s[axis] - number(&row["velocity_km_s"][axis])).abs();
            max_position_km = max_position_km.max(dp);
            max_velocity_km_s = max_velocity_km_s.max(dv);
            assert!(dp < 1e-6, "axis {axis}: position error {dp} km");
            assert!(dv < 1e-7, "axis {axis}: velocity error {dv} km/s");
        }
        // This difference checks the derivative using the same broadcast record.
        let h = Duration::from_seconds(0.05);
        let before = chosen.kepler_state_at(t - h).unwrap();
        let after = chosen.kepler_state_at(t + h).unwrap();
        for axis in 0..3 {
            let derivative = (after.position_km[axis] - before.position_km[axis]) / 0.1;
            assert!((derivative - state.velocity_km_s[axis]).abs() < 1e-7);
        }
    }
    println!(
        "maximum errors: position={max_position_km:.3e} km velocity={max_velocity_km_s:.3e} km/s"
    );
}

#[test]
fn geo_week_boundary_and_zero_harmonics_have_analytic_position() {
    let (mut nav, sv) = record();
    let (mut key, mut frame) = nav.record.as_mut_nav().unwrap().pop_first().unwrap();
    let eph = frame.as_mut_ephemeris().unwrap();
    let a = eph.get_orbit_f64("sqrta").unwrap().powi(2);
    for field in [
        "e", "m0", "i0", "omega", "omega0", "omegaDot", "idot", "deltaN", "cus", "cuc", "cis",
        "cic", "crs", "crc",
    ] {
        eph.orbits.insert(field.into(), 0.0.into());
    }
    eph.orbits.insert("toe".into(), 604790.0.into());
    let toe = eph.toe(sv).unwrap();
    // The synthetic orbit crosses the week boundary; move its synthetic ToC
    // with ToE so the independent clock-window rule does not mask orbit math.
    key.epoch = toe;
    nav.record.as_mut_nav().unwrap().insert(key, frame);
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    let chosen = report.chosen().unwrap();
    let state = chosen.kepler_state_at(toe).unwrap();
    let node = -7.292115e-5_f64 * 604790.0;
    let tilt = 5.0_f64.to_radians();
    let expected = [
        a * node.cos() / 1000.0,
        a * node.sin() * tilt.cos() / 1000.0,
        a * node.sin() * tilt.sin() / 1000.0,
    ];
    for axis in 0..3 {
        assert!((state.position_km[axis] - expected[axis]).abs() < 1e-8);
    }
    let after = chosen
        .kepler_state_at(toe + Duration::from_seconds(20.0))
        .unwrap();
    let u = (3.986004418e14 / a.powi(3)).sqrt() * 20.0;
    let xg = a * (u + node).cos();
    let yg = a * (u + node).sin();
    let theta: f64 = 7.292115e-5 * 20.0;
    let expected_after = [
        (xg * theta.cos() + yg * tilt.cos() * theta.sin()) / 1000.0,
        (-xg * theta.sin() + yg * tilt.cos() * theta.cos()) / 1000.0,
        yg * tilt.sin() / 1000.0,
    ];
    for axis in 0..3 {
        assert!((after.position_km[axis] - expected_after[axis]).abs() < 1e-8);
    }
    assert!(
        (chosen
            .ephemeris
            .helper_checked(sv, toe + Duration::from_seconds(20.0))
            .unwrap()
            .t_k
            - 20.0)
            .abs()
            < 1e-9
    );
}

#[test]
fn geo_invalid_and_out_of_window_fail_explicitly() {
    let (mut nav, sv) = record();
    let toe = nav
        .nav_ephemeris_frames_iter()
        .next()
        .unwrap()
        .1
        .toe(sv)
        .unwrap();
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    let chosen = report.chosen().unwrap();
    assert_eq!(
        chosen
            .kepler_state_at(toe + Duration::from_seconds(21600.0))
            .unwrap_err(),
        KeplerStateError::OutOfValidity
    );
    let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
    frame
        .as_mut_ephemeris()
        .unwrap()
        .orbits
        .insert("e".into(), 1.0.into());
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::Unpropagatable)
    );
    assert!(report.chosen().is_none());
    let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
    frame
        .as_mut_ephemeris()
        .unwrap()
        .orbits
        .insert("toe".into(), (-1.0).into());
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::MissingOrbitReference)
    );
}

#[test]
fn geo_nonconvergence_is_reported() {
    let (mut nav, sv) = record();
    let toe = nav
        .nav_ephemeris_frames_iter()
        .next()
        .unwrap()
        .1
        .toe(sv)
        .unwrap();
    let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
    let eph = frame.as_mut_ephemeris().unwrap();
    eph.orbits.insert("e".into(), 0.9999999999999442.into());
    eph.orbits.insert("m0".into(), 694.8674738744653.into());
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::Unpropagatable)
    );
    assert!(report.chosen().is_none());
}

#[test]
fn geo_identity_with_d1_message_is_not_silently_propagated() {
    let (mut nav, sv) = record();
    let frames = nav.record.as_mut_nav().unwrap();
    let (mut key, frame) = frames.pop_first().unwrap();
    key.msgtype = NavMessageType::D1; // Synthetic contradictory message/class.
    frames.insert(key, frame);
    let toe = nav
        .nav_ephemeris_frames_iter()
        .next()
        .unwrap()
        .1
        .toe(sv)
        .unwrap();
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    assert!(report.chosen().is_none());
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::UnsupportedMessage)
    );
}
