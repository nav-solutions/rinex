//! Real NavIC I02 native orbit remains available after frame rejection.
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
fn i02_native_reference_is_preserved_and_unverified_frame_is_rejected() {
    let nav = Rinex::from_file("tests/fixtures/nav_navic_i02_2023071.rnx").unwrap();
    let rows: serde_json::Value =
        serde_json::from_str(include_str!("reference/nav_navic_lnav_expected.json")).unwrap();
    let toe = Epoch::from_str("2023-03-12T00:00:00 GPST").unwrap();
    let source = SourceFrameIdentity::NavicBroadcastWgs84;
    for row in rows.as_array().unwrap() {
        let t = toe + Duration::from_seconds(row["offset_s"].as_f64().unwrap());
        let selected =
            nav.nav_select_ephemeris(SV::from_str("I02").unwrap(), t, UnknownHealthPolicy::Reject);
        let native = selected.chosen().unwrap().spatial_state_at(t).unwrap();
        assert_eq!(native.state.source(), source);
        assert_eq!(native.state.realization(), FrameRealization::Unknown);
        for axis in 0..3 {
            let expected = row["position_km"][axis].as_f64().unwrap();
            assert!((native.state.position_km[axis] - expected).abs() < 1e-6);
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
