//! Native six-vector and GPS WGS-84 identity path on public NAV records.
#![cfg(feature = "nav")]
use rinex::{
    navigation::rinex::{
        selection::{NativeFrame, UnknownHealthPolicy},
        spatial_state::{
            FrameError, FrameMethod, FrameRealization, FrameRequest, SourceFrameIdentity,
        },
    },
    prelude::{Duration, Rinex, SV},
};
use std::str::FromStr;

#[test]
fn real_g02_keeps_key_epoch_and_independent_native_six_vector() {
    let nav = Rinex::from_file("tests/fixtures/nav_legacy_kms_2022159.rnx").unwrap();
    let sv = SV::from_str("G02").unwrap();
    let (key, eph) = nav
        .nav_ephemeris_frames_iter()
        .find(|(k, _)| k.sv == sv)
        .unwrap();
    let toe = eph.toe(sv).unwrap();
    let t = toe + Duration::from_seconds(300.0);
    let selected = nav.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
    let native = selected.chosen().unwrap().spatial_state_at(t).unwrap();
    assert_eq!(native.key, *key);
    assert_eq!(native.record_epoch, key.epoch);
    assert_eq!(native.orbit_reference, Some(toe));
    assert_eq!(native.clock_reference, key.epoch);
    assert_eq!(native.state.epoch, t);
    assert_eq!(native.state.native_frame(), NativeFrame::GpsBroadcastWgs84);
    assert_eq!(
        native.state.source(),
        SourceFrameIdentity::GpsBroadcastWgs84
    );
    assert_eq!(native.state.realization(), FrameRealization::Unknown);

    // Frozen before this wrapper: the independent NAV_LEGACY reference parses
    // RINEX slots itself and evaluates ToE + 300 s without this Rust path.
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("reference/nav_legacy_expected.json")).unwrap();
    let expected = reference["G02 LNAV"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["offset_s"].as_f64() == Some(300.0))
        .unwrap();
    for axis in 0..3 {
        let p = expected["position_km"][axis].as_f64().unwrap();
        let v = expected["velocity_km_s"][axis].as_f64().unwrap();
        assert!((native.state.position_km[axis] - p).abs() < 1e-6);
        assert!((native.state.velocity_km_s[axis] - v).abs() < 1e-7);
    }

    // Conversion requires only the self-contained state, never key/file data.
    let state = native.state;
    drop(nav);
    let output = state.to_frame(FrameRequest::Wgs84).unwrap();
    assert_eq!(output.epoch, t);
    assert_eq!(output.target, SourceFrameIdentity::GpsBroadcastWgs84);
    assert_eq!(output.target_realization, FrameRealization::Unknown);
    assert_eq!(output.method, FrameMethod::Native);
    assert_eq!(output.position_km, state.position_km);
    assert_eq!(output.velocity_km_s, Some(state.velocity_km_s));
}

#[test]
fn real_navic_wgs84_family_is_not_gps_broadcast_identity() {
    let nav = Rinex::from_file("tests/fixtures/nav_navic_i02_2023071.rnx").unwrap();
    let sv = SV::from_str("I02").unwrap();
    let (key, _) = nav.nav_ephemeris_frames_iter().next().unwrap();
    let t = key.epoch;
    let report = nav.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
    let native = report.chosen().unwrap().spatial_state_at(t).unwrap();
    assert_eq!(
        native.state.native_frame(),
        NativeFrame::NavicBroadcastWgs84
    );
    assert_eq!(
        native.state.source(),
        SourceFrameIdentity::NavicBroadcastWgs84
    );
    assert_eq!(
        native.state.to_frame(FrameRequest::Wgs84).unwrap_err(),
        FrameError::UnsupportedSource(SourceFrameIdentity::NavicBroadcastWgs84)
    );
    assert!(native
        .state
        .position_km
        .iter()
        .all(|value| value.is_finite()));
}

#[test]
fn real_fdma_and_sbas_keep_their_message_and_source_identity() {
    for (file, sv, expected) in [
        (
            "data/NAV/V4/rinex402_examples_MN.rnx",
            "R01",
            SourceFrameIdentity::GlonassBroadcastPz90,
        ),
        (
            "tests/fixtures/nav_sbas_s27_2023071.rnx",
            "S27",
            SourceFrameIdentity::SbasBroadcast,
        ),
    ] {
        let nav = Rinex::from_file(file).unwrap();
        let sv = SV::from_str(sv).unwrap();
        let (key, _) = nav
            .nav_ephemeris_frames_iter()
            .find(|(k, _)| k.sv == sv)
            .unwrap();
        let t = key.epoch;
        let report = nav.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
        let native = report.chosen().unwrap().spatial_state_at(t).unwrap();
        assert_eq!(native.key, *key);
        assert_eq!(native.state.source(), expected);
        assert_eq!(native.state.epoch, t);
        let expected_error = if expected == SourceFrameIdentity::GlonassBroadcastPz90 {
            assert_eq!(
                native.state.realization(),
                FrameRealization::Known(rinex::navigation::rinex::spatial_state::FrameId::Pz90_11)
            );
            FrameError::OutsideCatalogWindow
        } else {
            FrameError::UnsupportedSource(expected)
        };
        assert_eq!(
            native.state.to_frame(FrameRequest::Wgs84).unwrap_err(),
            expected_error
        );
        assert!(native
            .state
            .position_km
            .iter()
            .all(|value| value.is_finite()));
    }
}

#[test]
fn nonfinite_six_vector_cannot_be_returned_as_native() {
    let nav = Rinex::from_file("tests/fixtures/nav_legacy_kms_2022159.rnx").unwrap();
    let sv = SV::from_str("G02").unwrap();
    let (key, _) = nav
        .nav_ephemeris_frames_iter()
        .find(|(k, _)| k.sv == sv)
        .unwrap();
    let report = nav.nav_select_ephemeris(sv, key.epoch, UnknownHealthPolicy::Reject);
    let mut state = report
        .chosen()
        .unwrap()
        .spatial_state_at(key.epoch)
        .unwrap()
        .state;
    state.velocity_km_s[1] = f64::NAN;
    assert_eq!(
        state.to_frame(FrameRequest::Wgs84).unwrap_err(),
        FrameError::NonFiniteState
    );
}
