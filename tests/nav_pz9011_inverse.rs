//! Invert the installed PZ-90.11/ITRF2014 operation and preserve its limits.
#![cfg(feature = "nav")]

use rinex::{
    navigation::rinex::{
        selection::UnknownHealthPolicy,
        spatial_state::{
            FrameError, FrameId, FrameRealization, FrameRequest, FrameTransformer, MethodPolicy,
            PositionStatus, SourceFrameIdentity, SpatialPoint, TransformOptions,
        },
    },
    prelude::{Epoch, Rinex, SV},
};
use std::str::FromStr;

const INVERSE_ID: &str = "ICG:2018:ITRF2014-to-PZ90.11:inverse-static-2010-approx";

fn epoch() -> Epoch {
    Epoch::from_str("2020-06-24T23:50:00 UTC").unwrap()
}

fn reference_xyz(name: &str) -> [f64; 3] {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "reference/nav_glonass_frame_approx_expected.json"
    ))
    .unwrap();
    std::array::from_fn(|i| data[name][i].as_f64().unwrap())
}

fn assert_xyz(actual: [f64; 3], expected: [f64; 3]) {
    for i in 0..3 {
        assert!(
            (actual[i] - expected[i]).abs() < 1e-9,
            "axis {i}: actual={} expected={}",
            actual[i],
            expected[i]
        );
    }
}

#[test]
fn inverse_of_published_r01_reference_recovers_independent_native_xyz() {
    // See nav_pz9011_inverse.py: the expected native point and the ITRF2014
    // input were independently derived from real R01 RINEX broadcast slots.
    let point = SpatialPoint::new(
        reference_xyz("approx_itrf2014_position_km"),
        epoch(),
        SourceFrameIdentity::Realization(FrameId::Itrf2014),
        None,
    )
    .unwrap();
    let result = FrameTransformer
        .to_frame(
            &point,
            FrameRequest::Realization(FrameId::Pz90_11),
            TransformOptions::default(),
        )
        .unwrap();
    assert_xyz(result.position_km, reference_xyz("native_position_km"));
    assert_eq!(
        result.position_status(),
        PositionStatus::MarkedApproximation
    );
    assert_eq!(result.edge_ids, [INVERSE_ID]);
    assert_eq!(result.info()[0].source, FrameId::Itrf2014);
    assert_eq!(result.info()[0].target, FrameId::Pz90_11);
    assert_eq!(
        result.source_realization,
        FrameRealization::Known(FrameId::Itrf2014)
    );
    assert_eq!(
        result.target_realization,
        FrameRealization::Known(FrameId::Pz90_11)
    );
    assert!(result.velocity_km_s.is_none());
    assert!(result
        .cautions()
        .iter()
        .any(|s| s.contains("2010.0") && s.contains("inverse")));
}

#[test]
fn reverse_edge_is_dated_and_strict_options_reject_unsupported_accuracy() {
    let point = SpatialPoint::new(
        reference_xyz("approx_itrf2014_position_km"),
        epoch(),
        SourceFrameIdentity::Realization(FrameId::Itrf2014),
        None,
    )
    .unwrap();
    let target = FrameRequest::Realization(FrameId::Pz90_11);
    for (options, error) in [
        (
            TransformOptions {
                method: MethodPolicy::NumericalOnly,
                ..Default::default()
            },
            FrameError::ApproximationExcluded,
        ),
        (
            TransformOptions {
                max_frame_operation_error_m: Some(1.0),
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
                .to_frame(&point, target, options)
                .unwrap_err(),
            error
        );
    }
    assert!(matches!(
        FrameTransformer.to_frame(
            &point,
            target,
            TransformOptions {
                warnings_as_errors: true,
                ..Default::default()
            }
        ),
        Err(FrameError::WarningRejected(_))
    ));
    let outside = SpatialPoint::new(
        point.position_km,
        Epoch::from_str("2025-01-01T00:00:00 UTC").unwrap(),
        point.source,
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
fn selected_e03_reaches_pz9011_through_ordered_approximate_edges() {
    let nav = Rinex::from_file("tests/fixtures/nav_mixed_2024131_first_epoch.rnx").unwrap();
    let evaluation = Epoch::from_str("2024-05-10T03:00:00 GPST").unwrap();
    let report = nav.nav_select_ephemeris(
        SV::from_str("E03").unwrap(),
        evaluation,
        UnknownHealthPolicy::Reject,
    );
    let native = report
        .chosen()
        .unwrap()
        .spatial_state_at(evaluation)
        .unwrap();
    let point = SpatialPoint::from_nav(&native.state).unwrap();
    let result = FrameTransformer
        .to_frame(
            &point,
            FrameRequest::Realization(FrameId::Pz90_11),
            TransformOptions::default(),
        )
        .unwrap();
    assert_eq!(
        result.position_status(),
        PositionStatus::MarkedApproximation
    );
    assert_eq!(result.edge_ids.len(), 3);
    assert_eq!(
        result.edge_ids[0],
        "ESA:GGSP:GTRF23v01-ITRF2020:zero-offset-approx"
    );
    assert_eq!(result.edge_ids[1], "ITRF2020:Table2:2015.0");
    assert_eq!(result.edge_ids[2], INVERSE_ID);
    assert!(result
        .position_km
        .iter()
        .all(|coordinate| coordinate.is_finite()));
    let cautions = result.cautions();
    assert!(cautions.iter().any(|s| s.contains("GTRF23v01-ITRF2020")));
    assert!(cautions
        .iter()
        .any(|s| s.contains(INVERSE_ID) && s.contains("2010.0")));
}
