//! NavIC I02 LNAV: normative RINEX layout, real broadcast, independent ICD reference.
#![cfg(feature = "nav")]
use rinex::{
    navigation::{
        irnss::IrnssHealth,
        rinex::{
            legacy_kepler::KeplerStateError,
            selection::{NativeFrame, NavRejection, UnknownHealthPolicy},
        },
        NavMessageType, OrbitItem,
    },
    prelude::{Duration, Epoch, Rinex, TimeScale, SV},
};
use serde_json::Value;
use std::str::FromStr;

const REAL: &str = "tests/fixtures/nav_navic_i02_2023071.rnx";
const EXPECTED: &str = include_str!("reference/nav_navic_lnav_expected.json");

fn i02() -> SV {
    SV::from_str("I02").unwrap()
}

fn real() -> Rinex {
    let nav = Rinex::from_file(REAL).unwrap();
    assert_eq!(nav.nav_ephemeris_frames_iter().count(), 1);
    nav
}

fn number(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

#[test]
fn rinex_402_normative_i02_fields_and_message_boundary() {
    let nav = Rinex::from_file("data/NAV/V4/rinex402_examples_MN.rnx").unwrap();
    let records: Vec<_> = nav
        .nav_ephemeris_frames_iter()
        .filter(|(key, _)| key.sv == i02())
        .collect();
    assert_eq!(records.len(), 2);
    let (key, eph) = records[0];
    assert_eq!(key.msgtype, NavMessageType::LNAV);
    assert_eq!(key.epoch.time_scale, TimeScale::GPST);
    assert_eq!(eph.get_orbit_f64("iodec"), Some(169.0));
    assert_eq!(eph.get_orbit_f64("toe"), Some(180336.0));
    assert_eq!(eph.get_orbit_f64("week"), Some(2123.0));
    assert_eq!(eph.get_orbit_f64("health"), Some(0.0));
    assert_eq!(
        eph.toe(i02()),
        Some(Epoch::from_str("2020-09-15T02:05:36 GPST").unwrap())
    );
    let report = nav.nav_select_ephemeris(i02(), key.epoch, UnknownHealthPolicy::Reject);
    assert_eq!(report.chosen().unwrap().key.msgtype, NavMessageType::LNAV);
    let l1nv = nav
        .nav_ephemeris_frames_iter()
        .find(|(key, _)| key.sv == SV::from_str("I10").unwrap())
        .unwrap();
    assert_eq!(l1nv.0.msgtype, NavMessageType::L1NV);
    let report = nav.nav_select_ephemeris(l1nv.0.sv, l1nv.0.epoch, UnknownHealthPolicy::Allow);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::UnsupportedMessage)
    );
}

#[test]
fn real_i02_selection_and_state_match_independent_reference() {
    let nav = real();
    let (key, eph) = nav.nav_ephemeris_frames_iter().next().unwrap();
    let toe = eph.toe(i02()).unwrap();
    assert_eq!(key.msgtype, NavMessageType::LNAV);
    assert_eq!(key.epoch.time_scale, TimeScale::GPST);
    assert_eq!(toe, key.epoch);
    assert_eq!(eph.get_orbit_f64("week"), Some(2253.0));
    assert_eq!(eph.get_orbit_f64("toe"), Some(0.0));
    assert_eq!(eph.get_orbit_f64("health"), Some(0.0));

    let rows: Value = serde_json::from_str(EXPECTED).unwrap();
    let mut max_position_km: f64 = 0.0;
    let mut max_velocity_km_s: f64 = 0.0;
    let mut max_clock_s: f64 = 0.0;
    for row in rows.as_array().unwrap() {
        let t = toe + Duration::from_seconds(number(&row["offset_s"]));
        let report = nav.nav_select_ephemeris(i02(), t, UnknownHealthPolicy::Reject);
        let chosen = report
            .chosen()
            .unwrap_or_else(|| panic!("I02 selection: {:?}", report.candidates));
        assert_eq!(chosen.key, key);
        assert_eq!(chosen.orbit_reference, Some(toe));
        assert_eq!(chosen.clock_reference, key.epoch);
        assert_eq!(chosen.native_frame, Some(NativeFrame::NavicBroadcastWgs84));
        assert_eq!(
            chosen.validity_half_window,
            Some(Duration::from_seconds(7200.0))
        );
        let state = chosen.kepler_state_at(t).unwrap();
        assert_eq!(state.frame, NativeFrame::NavicBroadcastWgs84);
        for axis in 0..3 {
            let dp = (state.position_km[axis] - number(&row["position_km"][axis])).abs();
            let dv = (state.velocity_km_s[axis] - number(&row["velocity_km_s"][axis])).abs();
            max_position_km = max_position_km.max(dp);
            max_velocity_km_s = max_velocity_km_s.max(dv);
            assert!(dp < 1e-6, "I02 {axis}: position error {dp} km");
            assert!(dv < 1e-7, "I02 {axis}: velocity error {dv} km/s");
        }
        let dc = (state.clock_correction_s - number(&row["clock_s_without_group_delay"])).abs();
        max_clock_s = max_clock_s.max(dc);
        assert!(dc < 1e-10, "I02 clock error {dc} s");
    }
    println!("max errors: position={max_position_km:.3e} km velocity={max_velocity_km_s:.3e} km/s clock={max_clock_s:.3e} s");
}

#[test]
fn health_codes_and_missing_health_fail_closed() {
    let mut nav = real();
    let toe = nav
        .nav_ephemeris_frames_iter()
        .next()
        .unwrap()
        .1
        .toe(i02())
        .unwrap();
    for code in 1..=3 {
        let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
        frame.as_mut_ephemeris().unwrap().orbits.insert(
            "health".into(),
            OrbitItem::IrnssHealth(IrnssHealth::from_bits_retain(code)),
        );
        let report = nav.nav_select_ephemeris(i02(), toe, UnknownHealthPolicy::Allow);
        assert_eq!(
            report.candidates[0].rejection,
            Some(NavRejection::Unhealthy)
        );
    }
    let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
    frame.as_mut_ephemeris().unwrap().orbits.remove("health");
    let report = nav.nav_select_ephemeris(i02(), toe, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::UnknownHealth)
    );
    assert!(nav
        .nav_select_ephemeris(i02(), toe, UnknownHealthPolicy::Allow)
        .chosen()
        .is_some());
}

#[test]
fn orbit_and_clock_windows_are_independent() {
    let mut nav = real();
    let toe = nav
        .nav_ephemeris_frames_iter()
        .next()
        .unwrap()
        .1
        .toe(i02())
        .unwrap();
    for offset in [-7200.0, 7200.0] {
        let t = toe + Duration::from_seconds(offset);
        let report = nav.nav_select_ephemeris(i02(), t, UnknownHealthPolicy::Reject);
        assert_eq!(
            report.candidates[0].rejection,
            Some(NavRejection::OutOfValidity)
        );
    }
    let chosen = nav.nav_select_ephemeris(i02(), toe, UnknownHealthPolicy::Reject);
    assert_eq!(
        chosen
            .chosen()
            .unwrap()
            .kepler_state_at(toe + Duration::from_seconds(7200.0))
            .unwrap_err(),
        KeplerStateError::OutOfValidity
    );

    // Synthetic stale clock epoch while the orbit epoch remains valid.
    let frames = nav.record.as_mut_nav().unwrap();
    let (mut key, frame) = frames.pop_first().unwrap();
    key.epoch += Duration::from_seconds(7200.0);
    frames.insert(key, frame);
    let report = nav.nav_select_ephemeris(i02(), toe, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::OutOfValidity)
    );
}

#[test]
fn missing_or_invalid_toe_is_rejected() {
    let mut nav = real();
    let toe = nav
        .nav_ephemeris_frames_iter()
        .next()
        .unwrap()
        .1
        .toe(i02())
        .unwrap();
    let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
    frame.as_mut_ephemeris().unwrap().orbits.remove("toe");
    let report = nav.nav_select_ephemeris(i02(), toe, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::MissingOrbitReference)
    );
}

#[test]
fn real_i02_same_version_write_read_preserves_state() {
    let nav = real();
    let path = std::env::temp_dir().join(format!("rinex-navic-lnav-{}.rnx", std::process::id()));
    nav.to_file(&path).unwrap();
    let reread = Rinex::from_file(&path);
    std::fs::remove_file(&path).unwrap();
    let reread = reread.unwrap();
    let (key, eph) = reread.nav_ephemeris_frames_iter().next().unwrap();
    let toe = eph.toe(i02()).unwrap();
    assert_eq!(key.msgtype, NavMessageType::LNAV);
    assert_eq!(eph.get_orbit_f64("health"), Some(0.0));
    assert_eq!(eph.get_orbit_f64("week"), Some(2253.0));
    assert_eq!(toe, key.epoch);
    let report = reread.nav_select_ephemeris(i02(), toe, UnknownHealthPolicy::Reject);
    let state = report.chosen().unwrap().kepler_state_at(toe).unwrap();
    let original = nav.nav_select_ephemeris(i02(), toe, UnknownHealthPolicy::Reject);
    let expected = original.chosen().unwrap().kepler_state_at(toe).unwrap();
    for axis in 0..3 {
        assert!((state.position_km[axis] - expected.position_km[axis]).abs() < 1e-6);
        assert!((state.velocity_km_s[axis] - expected.velocity_km_s[axis]).abs() < 1e-7);
    }
    assert!((state.clock_correction_s - expected.clock_correction_s).abs() < 1e-10);
}
