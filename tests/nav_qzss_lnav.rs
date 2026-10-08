//! QZSS J02 LNAV: real RINEX 4 record, independent ICD equations, and failures.
#![cfg(feature = "nav")]
use rinex::{
    navigation::{
        gps::GpsQzssl1l2l5Health,
        rinex::{
            legacy_kepler::KeplerStateError,
            selection::{NativeFrame, NavRejection, UnknownHealthPolicy},
        },
        NavMessageType, OrbitItem,
    },
    prelude::{Duration, Rinex, TimeScale, SV},
};
use serde_json::Value;
use std::str::FromStr;

const FIXTURE: &str = "tests/fixtures/nav_qzss_j02_2023071.rnx";
const EXPECTED: &str = include_str!("reference/nav_qzss_lnav_expected.json");

fn nav() -> Rinex {
    let nav = Rinex::from_file(FIXTURE).unwrap();
    assert_eq!(nav.nav_ephemeris_frames_iter().count(), 1);
    nav
}

fn j02() -> SV {
    SV::from_str("J02").unwrap()
}

fn number(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

#[test]
fn real_j02_read_select_and_state_match_independent_reference() {
    let nav = nav();
    let sv = j02();
    let (key, eph) = nav.nav_ephemeris_frames_iter().next().unwrap();
    assert_eq!(key.msgtype, NavMessageType::LNAV);
    assert_eq!(key.epoch.time_scale, TimeScale::QZSST);
    assert_eq!(eph.get_orbit_f64("week"), Some(2253.0));
    assert_eq!(eph.get_orbit_f64("toe"), Some(0.0));
    assert_eq!(eph.get_orbit_f64("fitInt"), Some(0.0));
    assert_eq!(eph.get_orbit_f64("health"), Some(0.0));
    // The old descriptor calls this QZSS reserved/status slot `l2Codes`;
    // its value is not the GPS "codes on L2" field or a rejection bit.
    assert_eq!(eph.get_orbit_f64("l2Codes"), Some(2.0));
    let toe = eph.toe(sv).unwrap();
    assert_eq!(toe, key.epoch);

    let expected: Value = serde_json::from_str(EXPECTED).unwrap();
    let mut max_position_km: f64 = 0.0;
    let mut max_velocity_km_s: f64 = 0.0;
    let mut max_clock_s: f64 = 0.0;
    for row in expected.as_array().unwrap() {
        let t = toe + Duration::from_seconds(number(&row["offset_s"]));
        let report = nav.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
        let chosen = report
            .chosen()
            .unwrap_or_else(|| panic!("J02 selection: {:?}", report.candidates));
        assert_eq!(chosen.key, key);
        assert_eq!(chosen.orbit_reference, Some(toe));
        assert_eq!(chosen.clock_reference, key.epoch);
        assert_eq!(
            chosen.validity_half_window,
            Some(Duration::from_seconds(3600.0))
        );
        assert_eq!(chosen.native_frame, Some(NativeFrame::QzssBroadcastJgs));
        let state = chosen.kepler_state_at(t).unwrap();
        assert_eq!(state.frame, NativeFrame::QzssBroadcastJgs);
        for axis in 0..3 {
            let dp = (state.position_km[axis] - number(&row["position_km"][axis])).abs();
            let dv = (state.velocity_km_s[axis] - number(&row["velocity_km_s"][axis])).abs();
            max_position_km = max_position_km.max(dp);
            max_velocity_km_s = max_velocity_km_s.max(dv);
            assert!(dp < 1e-6, "J02 {axis}: position error {dp} km");
            assert!(dv < 1e-7, "J02 {axis}: velocity error {dv} km/s");
        }
        let dc = (state.clock_correction_s - number(&row["clock_s_without_group_delay"])).abs();
        max_clock_s = max_clock_s.max(dc);
        assert!(dc < 1e-10, "J02 clock error {dc} s");
    }
    println!("max errors: position={max_position_km:.3e} km velocity={max_velocity_km_s:.3e} km/s clock={max_clock_s:.3e} s");
}

#[test]
fn orbit_and_clock_windows_are_both_two_hours_total() {
    let mut nav = nav();
    let sv = j02();
    let toe = nav
        .nav_ephemeris_frames_iter()
        .next()
        .unwrap()
        .1
        .toe(sv)
        .unwrap();
    for offset in [-3600.0, 3600.0] {
        let t = toe + Duration::from_seconds(offset);
        let report = nav.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
        assert!(report.chosen().is_none());
        assert_eq!(
            report.candidates[0].rejection,
            Some(NavRejection::OutOfValidity)
        );
    }
    let report = nav.nav_select_ephemeris(
        sv,
        toe + Duration::from_seconds(3599.0),
        UnknownHealthPolicy::Reject,
    );
    let candidate = report.chosen().unwrap();
    assert_eq!(
        candidate
            .kepler_state_at(toe + Duration::from_seconds(3600.0))
            .unwrap_err(),
        KeplerStateError::OutOfValidity
    );

    // Synthetic clock epoch shift: orbit is valid, but clock is stale.
    let frames = nav.record.as_mut_nav().unwrap();
    let (mut key, frame) = frames.pop_first().unwrap();
    key.epoch += Duration::from_seconds(7200.0);
    frames.insert(key, frame);
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::OutOfValidity)
    );

    // A candidate selected at one time must not later return an expired clock.
    let frames = nav.record.as_mut_nav().unwrap();
    let (mut key, frame) = frames.pop_first().unwrap();
    key.epoch = toe + Duration::from_seconds(1800.0);
    frames.insert(key, frame);
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    assert_eq!(
        report
            .chosen()
            .unwrap()
            .kepler_state_at(toe - Duration::from_seconds(2000.0))
            .unwrap_err(),
        KeplerStateError::OutOfValidity
    );
}

#[test]
fn health_fit_and_message_fail_closed() {
    let sv = j02();
    let mut nav = nav();
    let toe = nav
        .nav_ephemeris_frames_iter()
        .next()
        .unwrap()
        .1
        .toe(sv)
        .unwrap();
    let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
    let eph = frame.as_mut_ephemeris().unwrap();
    eph.orbits.insert(
        "health".into(),
        OrbitItem::GpsQzssl1l2l5Health(GpsQzssl1l2l5Health::from(1)),
    );
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::Unhealthy)
    );

    let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
    let eph = frame.as_mut_ephemeris().unwrap();
    eph.orbits.insert(
        "health".into(),
        OrbitItem::GpsQzssl1l2l5Health(GpsQzssl1l2l5Health::from(0)),
    );
    eph.orbits.insert("fitInt".into(), 1.0.into());
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::InvalidData)
    );

    let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
    let eph = frame.as_mut_ephemeris().unwrap();
    eph.orbits.remove("fitInt");
    eph.orbits.remove("health");
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::UnknownHealth)
    );
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Allow);
    assert!(report.chosen().is_some());

    let frames = nav.record.as_mut_nav().unwrap();
    let (mut key, frame) = frames.pop_first().unwrap();
    key.msgtype = NavMessageType::CNAV;
    frames.insert(key, frame);
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Allow);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::UnsupportedMessage)
    );
}

#[test]
fn synthetic_circular_equatorial_state_has_analytic_velocity() {
    let sv = j02();
    let mut nav = nav();
    let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
    let eph = frame.as_mut_ephemeris().unwrap();
    let a = eph.get_orbit_f64("sqrta").unwrap().powi(2);
    for field in [
        "e", "m0", "i0", "omega", "omega0", "omegaDot", "idot", "deltaN", "cus", "cuc", "cis",
        "cic", "crs", "crc",
    ] {
        eph.orbits.insert(field.into(), 0.0.into());
    }
    let toe = eph.toe(sv).unwrap();
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    let state = report.chosen().unwrap().kepler_state_at(toe).unwrap();
    let expected_y_km_s = a * ((3.986005e14_f64 / a.powi(3)).sqrt() - 7.2921151467e-5) / 1000.0;
    assert!((state.position_km[0] - a / 1000.0).abs() < 1e-7);
    assert!(state.position_km[1].abs() < 1e-7);
    assert!(state.position_km[2].abs() < 1e-7);
    assert!(state.velocity_km_s[0].abs() < 1e-10);
    assert!((state.velocity_km_s[1] - expected_y_km_s).abs() < 1e-10);
    assert!(state.velocity_km_s[2].abs() < 1e-10);
    assert!(
        (state.clock_correction_s - report.chosen().unwrap().ephemeris.clock_bias).abs() < 1e-14
    );
}
