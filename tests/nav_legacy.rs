//! Five real RINEX 4 broadcast records against an independent RTKLIB-style
//! numerical reference. The fixture is a verbatim subset of KMS300DNK.
#![cfg(feature = "nav")]
use rinex::{
    navigation::{
        gal::GalHealth,
        rinex::{
            legacy_kepler::KeplerStateError,
            selection::{NativeFrame, NavRejection, UnknownHealthPolicy},
        },
        NavMessageType, OrbitItem,
    },
    prelude::{Duration, Rinex, SV},
};
use serde_json::Value;
use std::str::FromStr;

const FIXTURE: &str = "tests/fixtures/nav_legacy_kms_2022159.rnx";
const EXPECTED: &str = include_str!("reference/nav_legacy_expected.json");

fn single_record(sv: SV, message: NavMessageType) -> Rinex {
    let mut nav = Rinex::from_file(FIXTURE).unwrap();
    nav.record
        .as_mut_nav()
        .unwrap()
        .retain(|k, _| k.sv == sv && k.msgtype == message);
    assert_eq!(nav.nav_ephemeris_frames_iter().count(), 1);
    nav
}

fn case(ident: &str) -> (SV, NavMessageType, NativeFrame) {
    let mut parts = ident.split_whitespace();
    let sv = SV::from_str(parts.next().unwrap()).unwrap();
    let msg = NavMessageType::from_str(parts.next().unwrap()).unwrap();
    let frame = match ident.as_bytes()[0] {
        b'G' => NativeFrame::GpsBroadcastWgs84,
        b'E' => NativeFrame::GalileoBroadcastGtrf,
        b'C' => NativeFrame::BeiDouBroadcastCgcs2000,
        _ => unreachable!(),
    };
    (sv, msg, frame)
}

fn numeric(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

#[test]
fn real_gps_galileo_bds_meo_and_igso_match_reference() {
    let expected: Value = serde_json::from_str(EXPECTED).unwrap();
    let mut max_position_km: f64 = 0.0;
    let mut max_velocity_km_s: f64 = 0.0;
    let mut max_clock_s: f64 = 0.0;
    for ident in ["G02 LNAV", "E08 INAV", "E08 FNAV", "C10 D1", "C20 D1"] {
        let (sv, msg, frame) = case(ident);
        let nav = single_record(sv, msg);
        let (key, eph) = nav.nav_ephemeris_frames_iter().next().unwrap();
        let toe = eph.toe(sv).unwrap();
        let clock_reference = key.epoch;
        for row in expected[ident].as_array().unwrap() {
            let t = toe + Duration::from_seconds(numeric(&row["offset_s"]));
            let report = nav.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
            let candidate = report
                .chosen()
                .unwrap_or_else(|| panic!("{ident}: {:?}", report.candidates));
            assert_eq!(candidate.key, key);
            assert_eq!(candidate.orbit_reference, Some(toe));
            assert_eq!(candidate.clock_reference, clock_reference);
            assert_eq!(candidate.native_frame, Some(frame));
            let state = candidate.kepler_state_at(t).unwrap();
            assert_eq!(state.epoch, t);
            assert_eq!(state.frame, frame);
            for axis in 0..3 {
                let dp = (state.position_km[axis] - numeric(&row["position_km"][axis])).abs();
                let dv = (state.velocity_km_s[axis] - numeric(&row["velocity_km_s"][axis])).abs();
                max_position_km = max_position_km.max(dp);
                max_velocity_km_s = max_velocity_km_s.max(dv);
                assert!(dp < 1e-6, "{ident} {axis}: position error {dp} km");
                assert!(dv < 1e-7, "{ident} {axis}: velocity error {dv} km/s");
            }
            let dc =
                (state.clock_correction_s - numeric(&row["clock_s_without_group_delay"])).abs();
            max_clock_s = max_clock_s.max(dc);
            assert!(dc < 1e-10, "{ident}: clock error {dc} s");
        }
    }
    println!("max errors: position={max_position_km:.3e} km velocity={max_velocity_km_s:.3e} km/s clock={max_clock_s:.3e} s");
}

#[test]
fn rejects_invalid_elements_and_out_of_window() {
    let sv = SV::from_str("G02").unwrap();
    let mut nav = single_record(sv, NavMessageType::LNAV);
    let toe = nav
        .nav_ephemeris_frames_iter()
        .next()
        .unwrap()
        .1
        .toe(sv)
        .unwrap();
    let t = toe + Duration::from_seconds(300.0);
    let candidate = nav.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
    let candidate = candidate.chosen().unwrap();
    assert_eq!(
        candidate
            .kepler_state_at(toe + Duration::from_seconds(7200.0))
            .unwrap_err(),
        KeplerStateError::OutOfValidity
    );
    let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
    frame
        .as_mut_ephemeris()
        .unwrap()
        .orbits
        .insert("e".into(), 1.0.into());
    let candidate = nav.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
    assert_eq!(
        candidate.candidates[0].rejection,
        Some(NavRejection::Unpropagatable)
    );
    assert!(candidate.chosen().is_none());
    let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
    frame
        .as_mut_ephemeris()
        .unwrap()
        .orbits
        .insert("toe".into(), (-1.0).into());
    let report = nav.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
    assert!(report.chosen().is_none());
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::MissingOrbitReference)
    );
}

#[test]
fn nonconvergent_kepler_equation_is_an_error() {
    let sv = SV::from_str("G02").unwrap();
    let mut nav = single_record(sv, NavMessageType::LNAV);
    let toe = nav
        .nav_ephemeris_frames_iter()
        .next()
        .unwrap()
        .1
        .toe(sv)
        .unwrap();
    let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
    let eph = frame.as_mut_ephemeris().unwrap();
    // Synthetic high-eccentricity stress input for the bounded Newton solver.
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
fn zero_harmonics_and_week_rollover_have_analytic_state() {
    let sv = SV::from_str("G02").unwrap();
    let mut nav = single_record(sv, NavMessageType::LNAV);
    let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
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
    // Synthetic ToC is placed at ToE so the clock also crosses the week.
    let frames = nav.record.as_mut_nav().unwrap();
    let (mut key, frame) = frames.pop_first().unwrap();
    key.epoch = toe;
    frames.insert(key, frame);
    let report = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    let chosen = report.chosen().unwrap();
    let at_toe = chosen.kepler_state_at(toe).unwrap();
    let after_week = chosen
        .kepler_state_at(toe + Duration::from_seconds(20.0))
        .unwrap();
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
    let rate = (3.986005e14 / a.powi(3)).sqrt() - 7.2921151467e-5;
    let node: f64 = -7.2921151467e-5 * 604790.0;
    assert!((at_toe.position_km[0] - a * node.cos() / 1000.0).abs() < 1e-8);
    assert!((at_toe.position_km[1] - a * node.sin() / 1000.0).abs() < 1e-8);
    assert!((at_toe.velocity_km_s[0] + a * rate * node.sin() / 1000.0).abs() < 1e-9);
    assert!((at_toe.velocity_km_s[1] - a * rate * node.cos() / 1000.0).abs() < 1e-9);
    assert!((after_week.position_km[1] - a * (node + rate * 20.0).sin() / 1000.0).abs() < 1e-8);
    let eph = chosen.ephemeris;
    let expected_clock = eph.clock_bias + eph.clock_drift * 20.0 + eph.clock_drift_rate * 400.0;
    assert!((after_week.clock_correction_s - expected_clock).abs() < 1e-12);
}

#[test]
fn galileo_message_and_health_context_are_kept() {
    let sv = SV::from_str("E08").unwrap();
    let mut nav = Rinex::from_file(FIXTURE).unwrap();
    let toe = nav
        .nav_ephemeris_frames_iter()
        .find(|(key, _)| key.sv == sv && key.msgtype == NavMessageType::INAV)
        .unwrap()
        .1
        .toe(sv)
        .unwrap();
    let selected = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    assert_eq!(selected.chosen().unwrap().key.msgtype, NavMessageType::INAV);
    assert!(selected
        .candidates
        .iter()
        .any(|c| c.key.msgtype == NavMessageType::FNAV
            && c.rejection == Some(NavRejection::LowerRank)));
    let (_, frame) = nav
        .record
        .as_mut_nav()
        .unwrap()
        .iter_mut()
        .find(|(k, _)| k.sv == sv && k.msgtype == NavMessageType::INAV)
        .unwrap();
    frame.as_mut_ephemeris().unwrap().orbits.insert(
        "health".into(),
        OrbitItem::GalHealth(GalHealth::from_bits_retain(1)),
    );
    let selected = nav.nav_select_ephemeris(sv, toe, UnknownHealthPolicy::Reject);
    assert!(selected
        .candidates
        .iter()
        .any(|c| c.key.msgtype == NavMessageType::INAV
            && c.rejection == Some(NavRejection::Unhealthy)));
    assert_eq!(selected.chosen().unwrap().key.msgtype, NavMessageType::FNAV);
}
