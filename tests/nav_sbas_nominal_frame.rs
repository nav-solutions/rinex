//! Real GAGAN S27 native polynomial remains available after frame rejection.
#![cfg(feature = "nav")]
use rinex::{
    navigation::rinex::{
        selection::UnknownHealthPolicy,
        spatial_state::{
            FrameError, FrameId, FrameRealization, FrameRequest, FrameTransformer,
            SourceFrameIdentity, SpatialPoint, TransformOptions,
        },
    },
    prelude::{Duration, Epoch, Rinex, SV},
};
use std::str::FromStr;

#[test]
fn s27_native_reference_is_preserved_and_unverified_frame_is_rejected() {
    let nav = Rinex::from_file("tests/fixtures/nav_sbas_s27_2023071.rnx").unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("reference/nav_sbas_s27_expected.json")).unwrap();
    let toc = Epoch::from_str("2023-03-12T01:15:44 GPST").unwrap();
    let source = SourceFrameIdentity::SbasBroadcast;
    for row in expected["rows"].as_array().unwrap() {
        let t = toc + Duration::from_seconds(row["dt_s"].as_f64().unwrap());
        let selected =
            nav.nav_select_ephemeris(SV::from_str("S27").unwrap(), t, UnknownHealthPolicy::Reject);
        let native = selected.chosen().unwrap().spatial_state_at(t).unwrap();
        assert_eq!(native.state.source(), source);
        assert_eq!(native.state.realization(), FrameRealization::Unknown);
        for axis in 0..3 {
            let position = row["position_km"][axis].as_f64().unwrap();
            let velocity = row["velocity_km_s"][axis].as_f64().unwrap();
            assert!((native.state.position_km[axis] - position).abs() < 1e-8);
            assert!((native.state.velocity_km_s[axis] - velocity).abs() < 1e-10);
        }
        let point = SpatialPoint::from_nav(&native.state).unwrap();
        for target in [
            FrameRequest::Wgs84,
            FrameRequest::Realization(FrameId::Itrf2014),
        ] {
            assert_eq!(
                native.state.to_frame(target).unwrap_err(),
                FrameError::UnsupportedSource(source)
            );
            assert_eq!(
                FrameTransformer
                    .to_frame(&point, target, TransformOptions::default())
                    .unwrap_err(),
                FrameError::UnsupportedSource(source)
            );
        }
    }
}
