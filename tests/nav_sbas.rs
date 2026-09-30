//! SBAS GEO MT9 state, using one DLR broadcast record and a separate field reader.
#![cfg(feature = "nav")]
use rinex::{
    navigation::{
        rinex::{
            sbas::SbasError,
            selection::{NativeFrame, NavRejection, UnknownHealthPolicy},
        },
        OrbitItem,
    },
    prelude::{Duration, Epoch, Rinex, TimeScale, SV},
};
use serde_json::Value;
use std::str::FromStr;

const FIXTURE: &str = "tests/fixtures/nav_sbas_s27_2023071.rnx";
const EXPECTED: &str = include_str!("reference/nav_sbas_s27_expected.json");

fn s27() -> SV {
    SV::from_str("S27").unwrap()
}
fn real() -> Rinex {
    let nav = Rinex::from_file(FIXTURE).unwrap();
    assert_eq!(nav.nav_ephemeris_frames_iter().count(), 1);
    nav
}
fn reference_epoch(nav: &Rinex) -> Epoch {
    nav.nav_ephemeris_frames_iter().next().unwrap().0.epoch
}
fn number(v: &Value) -> f64 {
    v.as_f64().unwrap()
}
fn edit(nav: &mut Rinex, f: impl FnOnce(&mut rinex::navigation::Ephemeris)) {
    let (_, frame) = nav.record.as_mut_nav().unwrap().iter_mut().next().unwrap();
    f(frame.as_mut_ephemeris().unwrap());
}

#[test]
fn real_s27_fields_and_independent_reference() {
    let nav = real();
    let (key, eph) = nav.nav_ephemeris_frames_iter().next().unwrap();
    assert_eq!(key.epoch.time_scale, TimeScale::GPST);
    assert_eq!(eph.get_orbit_f64("t_tm"), Some(4519.0));
    assert_eq!(eph.get_orbit_f64("week"), None);
    assert_eq!(eph.clock_drift_rate, 0.0);
    assert_eq!(eph.get_orbit_f64("satPosX"), Some(24160.61976));
    assert_eq!(eph.get_orbit_f64("velY"), Some(0.00012625));
    assert_eq!(eph.get_orbit_f64("accelZ"), Some(-1.875e-7));
    assert_eq!(eph.get_orbit_f64("health"), Some(0.0));
    let expected: Value = serde_json::from_str(EXPECTED).unwrap();
    assert_eq!(number(&expected["t_tm_s"]), 4519.0);
    let mut max_position: f64 = 0.0;
    let mut max_velocity: f64 = 0.0;
    let mut max_clock: f64 = 0.0;
    for row in expected["rows"].as_array().unwrap() {
        let dt = number(&row["dt_s"]);
        let t = key.epoch + Duration::from_seconds(dt);
        let selection = nav.nav_select_ephemeris(s27(), t, UnknownHealthPolicy::Reject);
        let chosen = selection.chosen().unwrap();
        assert_eq!(chosen.orbit_reference, Some(key.epoch));
        assert_eq!(chosen.clock_reference, key.epoch);
        assert_eq!(
            chosen.validity_half_window,
            Some(Duration::from_seconds(360.0))
        );
        let state = chosen.sbas_state_at(t).unwrap();
        assert_eq!(state.epoch, t);
        assert_eq!(state.frame, NativeFrame::SbasBroadcast);
        for axis in 0..3 {
            let dp = (state.position_km[axis] - number(&row["position_km"][axis])).abs();
            let dv = (state.velocity_km_s[axis] - number(&row["velocity_km_s"][axis])).abs();
            max_position = max_position.max(dp);
            max_velocity = max_velocity.max(dv);
            assert!(dp < 1e-8, "dt={dt} axis={axis} position error={dp}");
            assert!(dv < 1e-12, "dt={dt} axis={axis} velocity error={dv}");
        }
        let dc = (state.clock_correction_s - number(&row["clock_correction_s"])).abs();
        max_clock = max_clock.max(dc);
        assert!(dc < 1e-18, "dt={dt} clock error={dc}");
    }
    println!("max position={max_position:.3e} km velocity={max_velocity:.3e} km/s clock={max_clock:.3e} s");
}

#[test]
fn distinct_axes_signed_time_and_explicit_zero() {
    let mut nav = real();
    let t0 = reference_epoch(&nav);
    edit(&mut nav, |eph| {
        for (axis, p, v, a) in [
            ('X', 100.0, 1.0, 0.1),
            ('Y', 200.0, 2.0, 0.0),
            ('Z', 300.0, 3.0, 0.3),
        ] {
            eph.orbits
                .insert(format!("satPos{axis}"), OrbitItem::F64(p));
            eph.orbits.insert(format!("vel{axis}"), OrbitItem::F64(v));
            eph.orbits.insert(format!("accel{axis}"), OrbitItem::F64(a));
        }
    });
    for (dt, p, v) in [
        (0.0, [100.0, 200.0, 300.0], [1.0, 2.0, 3.0]),
        (2.0, [102.2, 204.0, 306.6], [1.2, 2.0, 3.6]),
        (-2.0, [98.2, 196.0, 294.6], [0.8, 2.0, 2.4]),
    ] {
        let t = t0 + Duration::from_seconds(dt);
        let chosen = nav.nav_select_ephemeris(s27(), t, UnknownHealthPolicy::Reject);
        let state = chosen.chosen().unwrap().sbas_state_at(t).unwrap();
        for axis in 0..3 {
            assert!((state.position_km[axis] - p[axis]).abs() < 1e-12);
            assert!((state.velocity_km_s[axis] - v[axis]).abs() < 1e-12);
        }
    }
}

#[test]
fn missing_field_is_not_zero_and_expiry_is_strict() {
    let mut nav = real();
    let t0 = reference_epoch(&nav);
    edit(&mut nav, |eph| {
        eph.orbits.remove("accelY");
    });
    let report = nav.nav_select_ephemeris(s27(), t0, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::MissingOrbitField)
    );
    assert_eq!(
        report.candidates[0].sbas_state_at(t0).unwrap_err(),
        SbasError::MissingField("accelY")
    );
    assert_eq!(
        report.candidates[0].sbas_clock_correction_s(t0).unwrap(),
        1.117587089539e-8
    );
    let nav = real();
    for dt in [-360.0, 360.0] {
        let t = t0 + Duration::from_seconds(dt);
        let report = nav.nav_select_ephemeris(s27(), t, UnknownHealthPolicy::Reject);
        assert!(report.chosen().is_none());
        assert_eq!(
            report.candidates[0].rejection,
            Some(NavRejection::OutOfValidity)
        );
        assert_eq!(
            report.candidates[0].sbas_state_at(t).unwrap_err(),
            SbasError::OutOfValidity
        );
    }
}

#[test]
fn clock_uses_toc_and_ignores_transmission_time() {
    let mut nav = real();
    let t0 = reference_epoch(&nav);
    edit(&mut nav, |eph| {
        eph.clock_bias = 1e-7;
        eph.clock_drift = -2e-11;
        eph.clock_drift_rate = 9000.0;
        eph.orbits.insert("t_tm".into(), OrbitItem::F64(500_000.5));
    });
    let t = t0 + Duration::from_seconds(120.0);
    let chosen = nav.nav_select_ephemeris(s27(), t, UnknownHealthPolicy::Reject);
    let chosen = chosen.chosen().unwrap();
    assert_eq!(chosen.ephemeris.get_orbit_f64("t_tm"), Some(500_000.5));
    assert!((chosen.sbas_clock_correction_s(t).unwrap() - (1e-7 - 2e-11 * 120.0)).abs() < 1e-20);
}

#[test]
fn health_and_missing_health_reject() {
    let mut nav = real();
    let t0 = reference_epoch(&nav);
    edit(&mut nav, |eph| {
        eph.orbits.insert(
            "health".into(),
            OrbitItem::GeoHealth(rinex::navigation::geo::GeoHealth::GEO_URA_INDEX_IS_15),
        );
    });
    let report = nav.nav_select_ephemeris(s27(), t0, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::Unhealthy)
    );
    let mut nav = real();
    edit(&mut nav, |eph| {
        eph.orbits.remove("health");
    });
    let report = nav.nav_select_ephemeris(s27(), t0, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::UnknownHealth)
    );
    let mut nav = real();
    edit(&mut nav, |eph| {
        eph.orbits.insert(
            "health".into(),
            OrbitItem::GeoHealth(rinex::navigation::geo::GeoHealth::GEO_HEALTH_MT17_UNAVAILABLE),
        );
    });
    let report = nav.nav_select_ephemeris(s27(), t0, UnknownHealthPolicy::Reject);
    assert_eq!(
        report.candidates[0].rejection,
        Some(NavRejection::UnknownHealth)
    );
}

#[test]
fn same_version_write_read_preserves_transmission_time_and_state() {
    let mut nav = real();
    edit(&mut nav, |eph| {
        eph.orbits.insert("t_tm".into(), OrbitItem::F64(4519.25));
    });
    let path = std::env::temp_dir().join(format!("rinex-sbas-{}.rnx", std::process::id()));
    nav.to_file(&path).unwrap();
    let reread = Rinex::from_file(&path);
    std::fs::remove_file(&path).unwrap();
    let reread = reread.unwrap();
    let t = reference_epoch(&nav) + Duration::from_seconds(120.0);
    let original = nav.nav_select_ephemeris(s27(), t, UnknownHealthPolicy::Reject);
    let restored = reread.nav_select_ephemeris(s27(), t, UnknownHealthPolicy::Reject);
    let a = original.chosen().unwrap().sbas_state_at(t).unwrap();
    let b = restored.chosen().unwrap().sbas_state_at(t).unwrap();
    assert_eq!(
        restored.chosen().unwrap().ephemeris.get_orbit_f64("t_tm"),
        Some(4519.25)
    );
    assert_eq!(a.position_km, b.position_km);
    assert_eq!(a.velocity_km_s, b.velocity_km_s);
    assert_eq!(a.clock_correction_s, b.clock_correction_s);
}
