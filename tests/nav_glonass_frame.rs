//! Real R01 NAV to a dated, explicitly approximate ITRF2014 position.
#![cfg(feature = "nav")]

use rinex::{
    navigation::rinex::{
        selection::UnknownHealthPolicy,
        spatial_state::{
            FrameError, FrameId, FrameMethod, FrameRealization, FrameRequest, FrameTransformer,
            SourceBasis, SourceFrameIdentity, SpatialPoint, TransformOptions,
        },
    },
    prelude::{Epoch, Rinex, SV},
};
use std::str::FromStr;

fn epoch() -> Epoch {
    Epoch::from_str("2020-06-24T23:50:00 UTC").unwrap()
}

fn reference() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "reference/nav_glonass_frame_approx_expected.json"
    ))
    .unwrap()
}

fn xyz(row: &serde_json::Value, name: &str) -> [f64; 3] {
    std::array::from_fn(|i| row[name][i].as_f64().unwrap())
}

fn assert_xyz(actual: [f64; 3], expected: [f64; 3], tolerance_km: f64) {
    for i in 0..3 {
        assert!(
            (actual[i] - expected[i]).abs() < tolerance_km,
            "axis {i}: actual={} expected={} tolerance_km={tolerance_km}",
            actual[i],
            expected[i]
        );
    }
}

#[test]
fn real_r01_produces_marked_itrf2014_approximation() {
    let nav = Rinex::from_file("tests/fixtures/nav_glonass_r01_pz9011_2020176.rnx").unwrap();
    let selected = nav.nav_select_ephemeris(
        SV::from_str("R01").unwrap(),
        epoch(),
        UnknownHealthPolicy::Reject,
    );
    let native = selected
        .chosen()
        .unwrap()
        .spatial_state_at(epoch())
        .unwrap();
    let expected = reference();
    assert_eq!(
        expected["fixture_sha256"],
        "b456dd8d83756a5b6108495c5a7b334d599edcbadd9ef944daa2b9cd6caaec52"
    );
    assert_eq!(native.key.sv, SV::from_str("R01").unwrap());
    assert_eq!(
        native.record_epoch,
        Epoch::from_str("2020-06-24T23:45:00 UTC").unwrap()
    );
    assert_eq!(native.orbit_reference, Some(native.record_epoch));
    assert_eq!(
        native.state.source(),
        SourceFrameIdentity::GlonassBroadcastPz90
    );
    assert_eq!(
        native.state.realization(),
        FrameRealization::Known(FrameId::Pz90_11)
    );
    assert_xyz(
        native.state.position_km,
        xyz(&expected, "native_position_km"),
        1e-8,
    );
    assert_xyz(
        native.state.velocity_km_s,
        xyz(&expected, "native_velocity_km_s"),
        1e-9,
    );

    let result = native
        .state
        .to_frame(FrameRequest::Realization(FrameId::Itrf2014))
        .unwrap();
    assert_eq!(result.epoch, epoch());
    assert_eq!(
        result.source_realization,
        FrameRealization::Known(FrameId::Pz90_11)
    );
    assert_eq!(
        result.target_realization,
        FrameRealization::Known(FrameId::Itrf2014)
    );
    assert_eq!(result.source_basis, SourceBasis::NavMessageAndCatalogDate);
    assert_eq!(result.source_evidence, native.state.source_evidence());
    assert_eq!(result.method, FrameMethod::UnboundedApproximate);
    assert_eq!(
        result.edge_ids,
        &["ICG:2018:PZ90.11-to-ITRF2014:static-2010-approx"]
    );
    assert_eq!(
        result.info()[0].parameter_reference_epoch,
        "2010.0; parameters frozen at later coordinate epochs"
    );
    assert!(result.position_accuracy_note.unwrap().contains("CAUTION"));
    assert_eq!(result.velocity_km_s, None);
    assert_xyz(
        result.position_km,
        xyz(&expected, "approx_itrf2014_position_km"),
        1e-9,
    );
}

#[test]
fn asserted_pz9011_uses_same_edge_and_strict_requests_reject_it() {
    let expected = reference();
    let point = SpatialPoint::new(
        xyz(&expected, "native_position_km"),
        epoch(),
        SourceFrameIdentity::Realization(FrameId::Pz90_11),
        None,
    )
    .unwrap();
    let target = FrameRequest::Realization(FrameId::Itrf2014);
    let result = FrameTransformer
        .to_frame(&point, target, TransformOptions::default())
        .unwrap();
    assert_xyz(
        result.position_km,
        xyz(&expected, "approx_itrf2014_position_km"),
        1e-9,
    );
    assert_eq!(result.source_basis, SourceBasis::CallerAsserted);
    assert_eq!(result.source_evidence, None);
    for (options, error) in [
        (
            TransformOptions {
                method: rinex::navigation::rinex::spatial_state::MethodPolicy::NumericalOnly,
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
                .to_frame(&point, target, options)
                .unwrap_err(),
            error
        );
    }
    assert!(matches!(
        FrameTransformer.to_frame(
            &point,
            target,
            TransformOptions { warnings_as_errors: true, ..Default::default() },
        ),
        Err(FrameError::WarningRejected(note)) if note.contains("CAUTION")
    ));
    let unresolved = SpatialPoint::new(
        point.position_km,
        epoch(),
        SourceFrameIdentity::GlonassBroadcastPz90,
        None,
    )
    .unwrap();
    assert_eq!(
        FrameTransformer
            .to_frame(&unresolved, target, TransformOptions::default())
            .unwrap_err(),
        FrameError::UnknownSourceRealization
    );
    let outside = SpatialPoint::new(
        point.position_km,
        Epoch::from_str("2025-01-01T00:00:00 UTC").unwrap(),
        SourceFrameIdentity::Realization(FrameId::Pz90_11),
        None,
    )
    .unwrap();
    assert_eq!(
        FrameTransformer
            .to_frame(&outside, target, TransformOptions::default())
            .unwrap_err(),
        FrameError::OutsideCatalogWindow
    );
    let composed = FrameTransformer
        .to_frame(
            &point,
            FrameRequest::Realization(FrameId::Itrf2020),
            TransformOptions::default(),
        )
        .unwrap();
    assert_eq!(composed.method, FrameMethod::UnboundedApproximate);
    assert_eq!(composed.edge_ids.len(), 2);
}

#[test]
fn nav_record_orbit_reference_and_evaluation_must_be_in_pz9011_window() {
    let nav = Rinex::from_file("tests/fixtures/nav_glonass_r01_pz9011_2020176.rnx").unwrap();
    let selection = nav.nav_select_ephemeris(
        SV::from_str("R01").unwrap(),
        epoch(),
        UnknownHealthPolicy::Reject,
    );
    let real = selection.chosen().unwrap();
    for (record, orbit, evaluation) in [
        (
            "2014-01-14T23:59:59 UTC",
            "2014-01-15T00:00:00 UTC",
            "2014-01-15T00:00:01 UTC",
        ),
        (
            "2014-01-15T00:00:00 UTC",
            "2014-01-14T23:59:59 UTC",
            "2014-01-15T00:00:01 UTC",
        ),
        (
            "2014-01-15T00:00:00 UTC",
            "2014-01-15T00:00:00 UTC",
            "2014-01-14T23:59:59 UTC",
        ),
        (
            "2024-12-31T23:59:59 UTC",
            "2025-01-01T00:00:00 UTC",
            "2024-12-31T23:59:59 UTC",
        ),
    ] {
        let mut key = *real.key;
        key.epoch = Epoch::from_str(record).unwrap();
        let mut candidate = *real;
        candidate.key = &key;
        candidate.clock_reference = key.epoch;
        candidate.orbit_reference = Some(Epoch::from_str(orbit).unwrap());
        let state = candidate
            .spatial_state_at(Epoch::from_str(evaluation).unwrap())
            .unwrap();
        assert_eq!(state.state.realization(), FrameRealization::Unknown);
        assert_eq!(state.state.source_evidence(), None);
        assert_eq!(
            state
                .state
                .to_frame(FrameRequest::Realization(FrameId::Itrf2014))
                .unwrap_err(),
            FrameError::UnknownSourceRealization
        );
        assert!(state.state.position_km.iter().all(|v| v.is_finite()));
    }
}
