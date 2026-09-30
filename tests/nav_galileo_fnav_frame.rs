//! N09d: real Galileo E08 F/NAV through the dated GTRF23v01 path.
#![cfg(feature = "nav")]

use rinex::{
    navigation::{
        rinex::{
            selection::UnknownHealthPolicy,
            spatial_state::{
                FrameError, FrameId, FrameMethod, FrameRealization, FrameRequest, FrameTransformer,
                MethodPolicy, SourceBasis, SourceFrameIdentity, SpatialPoint, TransformOptions,
            },
        },
        NavMessageType,
    },
    prelude::{Duration, Epoch, Rinex, SV},
};
use std::str::FromStr;

const FIXTURE: &str = "tests/fixtures/nav_galileo_e08_fnav_2024128.rnx";
const EXPECTED: &str = include_str!("reference/nav_galileo_fnav_frame_expected.json");

fn xyz(row: &serde_json::Value, field: &str) -> [f64; 3] {
    std::array::from_fn(|i| row[field][i].as_f64().unwrap())
}

fn assert_xyz(actual: [f64; 3], expected: [f64; 3], tolerance_km: f64) -> f64 {
    let mut maximum: f64 = 0.0;
    for axis in 0..3 {
        let difference = (actual[axis] - expected[axis]).abs();
        maximum = maximum.max(difference);
        assert!(
            difference < tolerance_km,
            "axis {axis}: actual={} expected={} difference={difference} km",
            actual[axis],
            expected[axis]
        );
    }
    maximum
}

fn toe() -> Epoch {
    Epoch::from_str("2024-05-07T00:40:00 GST").unwrap()
}

#[test]
fn real_e08_fnav_reaches_marked_itrf2014_at_its_own_epoch() {
    let nav = Rinex::from_file(FIXTURE).unwrap();
    let sv = SV::from_str("E08").unwrap();
    let reference: serde_json::Value = serde_json::from_str(EXPECTED).unwrap();
    let mut maximum_km: f64 = 0.0;

    for row in reference.as_array().unwrap() {
        let epoch = toe() + Duration::from_seconds(row["offset_s"].as_f64().unwrap());
        let selected = nav.nav_select_ephemeris(sv, epoch, UnknownHealthPolicy::Reject);
        let native = selected.chosen().unwrap().spatial_state_at(epoch).unwrap();
        assert_eq!(native.key.sv, sv);
        assert_eq!(native.key.msgtype, NavMessageType::FNAV);
        assert_eq!(native.record_epoch, toe());
        assert_eq!(native.orbit_reference, Some(toe()));
        assert_eq!(
            native.state.source(),
            SourceFrameIdentity::GalileoBroadcastGtrf
        );
        assert_eq!(
            native.state.realization(),
            FrameRealization::Known(FrameId::GalileoGtrf23v01)
        );
        assert!(native
            .state
            .source_evidence()
            .unwrap()
            .contains("GTRF23v01"));
        maximum_km = maximum_km.max(assert_xyz(
            native.state.position_km,
            xyz(row, "native_position_km"),
            1e-6,
        ));

        let result = FrameTransformer
            .to_frame(
                &SpatialPoint::from_nav(&native.state).unwrap(),
                FrameRequest::Realization(FrameId::Itrf2014),
                TransformOptions::default(),
            )
            .unwrap();
        assert_eq!(result.epoch, epoch);
        assert_eq!(result.source, SourceFrameIdentity::GalileoBroadcastGtrf);
        assert_eq!(
            result.source_realization,
            FrameRealization::Known(FrameId::GalileoGtrf23v01)
        );
        assert_eq!(
            result.target,
            SourceFrameIdentity::Realization(FrameId::Itrf2014)
        );
        assert_eq!(result.source_basis, SourceBasis::NavMessageAndCatalogDate);
        assert_eq!(result.source_evidence, native.state.source_evidence());
        assert_eq!(result.method, FrameMethod::UnboundedApproximate);
        assert_eq!(
            result.edge_ids,
            &[
                "ESA:GGSP:GTRF23v01-ITRF2020:zero-offset-approx",
                "ITRF2020:Table2:2015.0",
            ]
        );
        assert_eq!(result.info()[0].source, FrameId::GalileoGtrf23v01);
        assert_eq!(result.info()[0].target, FrameId::Itrf2020);
        assert_eq!(result.info()[1].target, FrameId::Itrf2014);
        assert!(result.position_accuracy_note.unwrap().contains("CAUTION"));
        assert_eq!(result.velocity_km_s, None);
        maximum_km = maximum_km.max(assert_xyz(
            result.position_km,
            xyz(row, "itrf2014_position_km"),
            1e-6,
        ));
    }
    println!("maximum XYZ difference from independent reference: {maximum_km:.3e} km");
}

#[test]
fn fnav_direct_point_and_strict_requests_follow_the_same_catalogue_edge() {
    let reference: serde_json::Value = serde_json::from_str(EXPECTED).unwrap();
    let row = &reference[1];
    let native_km = xyz(row, "native_position_km");
    let asserted = SpatialPoint::new(
        native_km,
        toe(),
        SourceFrameIdentity::Realization(FrameId::GalileoGtrf23v01),
        Some([0.1, 0.2, 0.3]),
    )
    .unwrap();
    let target = FrameRequest::Realization(FrameId::Itrf2014);
    let result = FrameTransformer
        .to_frame(&asserted, target, TransformOptions::default())
        .unwrap();
    assert_xyz(result.position_km, xyz(row, "itrf2014_position_km"), 1e-9);
    assert_eq!(result.source_basis, SourceBasis::CallerAsserted);
    assert_eq!(result.method, FrameMethod::UnboundedApproximate);
    assert_eq!(result.velocity_km_s, None);

    for (options, expected) in [
        (
            TransformOptions {
                method: MethodPolicy::NumericalOnly,
                ..Default::default()
            },
            FrameError::ApproximationExcluded,
        ),
        (
            TransformOptions {
                max_frame_operation_error_m: Some(100.0),
                ..Default::default()
            },
            FrameError::PositionBoundUnavailable,
        ),
        (
            TransformOptions {
                require_velocity: true,
                ..Default::default()
            },
            FrameError::VelocityUnavailable,
        ),
    ] {
        assert_eq!(
            FrameTransformer
                .to_frame(&asserted, target, options)
                .unwrap_err(),
            expected
        );
    }

    let unknown = SpatialPoint::new(
        native_km,
        toe(),
        SourceFrameIdentity::GalileoBroadcastGtrf,
        None,
    )
    .unwrap();
    assert_eq!(
        FrameTransformer
            .to_frame(&unknown, target, TransformOptions::default())
            .unwrap_err(),
        FrameError::UnknownSourceRealization
    );
    let outside = SpatialPoint::new(
        native_km,
        Epoch::from_str("2024-06-01T00:00:00 UTC").unwrap(),
        SourceFrameIdentity::Realization(FrameId::GalileoGtrf23v01),
        None,
    )
    .unwrap();
    assert_eq!(
        FrameTransformer
            .to_frame(&outside, target, TransformOptions::default())
            .unwrap_err(),
        FrameError::OutsideCatalogWindow
    );
}

#[test]
fn older_e08_fnav_stays_unknown() {
    let mut nav = Rinex::from_file("tests/fixtures/nav_legacy_kms_2022159.rnx").unwrap();
    let sv = SV::from_str("E08").unwrap();
    nav.record
        .as_mut_nav()
        .unwrap()
        .retain(|key, _| key.sv == sv && key.msgtype == NavMessageType::FNAV);
    let epoch = Epoch::from_str("2022-06-08T09:40:00 GST").unwrap();
    let selected = nav.nav_select_ephemeris(sv, epoch, UnknownHealthPolicy::Reject);
    let native = selected.chosen().unwrap().spatial_state_at(epoch).unwrap();
    assert_eq!(native.key.msgtype, NavMessageType::FNAV);
    assert_eq!(native.state.realization(), FrameRealization::Unknown);
    assert_eq!(
        native
            .state
            .to_frame(FrameRequest::Realization(FrameId::Itrf2014))
            .unwrap_err(),
        FrameError::UnknownSourceRealization
    );
    assert!(native.state.position_km.iter().all(|v| v.is_finite()));
}
