//! Real first-epoch R02 source and independent three-edge frame reference.
#![cfg(feature = "nav")]
use rinex::{
    navigation::rinex::{
        selection::UnknownHealthPolicy,
        spatial_state::{
            FrameError, FrameId, FrameMethod, FrameRealization, FrameRequest, FrameTransformer,
            MethodPolicy, PositionStatus, SourceFrameIdentity, SpatialPoint, TransformOptions,
        },
    },
    prelude::{Epoch, Rinex, SV},
};
use std::str::FromStr;

const NAV: &str = "tests/fixtures/nav_mixed_2024131_first_epoch.rnx";
const EXPECTED: &str = include_str!("reference/nav_glonass_r02_mixed_frame_expected.json");

fn instant() -> Epoch {
    Epoch::from_str("2024-05-10T03:00:00 GPST").unwrap()
}

fn reference() -> serde_json::Value {
    serde_json::from_str(EXPECTED).unwrap()
}

fn xyz(row: &serde_json::Value, field: &str) -> [f64; 3] {
    std::array::from_fn(|i| row[field][i].as_f64().unwrap())
}

fn assert_xyz(actual: [f64; 3], expected: [f64; 3], tolerance_km: f64) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() < tolerance_km,
            "axis {axis}: actual={} expected={} tolerance_km={tolerance_km}",
            actual[axis],
            expected[axis]
        );
    }
}

fn r02_point() -> SpatialPoint {
    let nav = Rinex::from_file(NAV).unwrap();
    let report = nav.nav_select_ephemeris(
        SV::from_str("R02").unwrap(),
        instant(),
        UnknownHealthPolicy::Reject,
    );
    let chosen = report.chosen().unwrap();
    assert_eq!(
        chosen.key.epoch,
        Epoch::from_str("2024-05-10T02:45:00 UTC").unwrap()
    );
    let native = chosen.spatial_state_at(instant()).unwrap();
    assert_eq!(native.orbit_reference, Some(chosen.key.epoch));
    assert_eq!(
        native.state.realization(),
        FrameRealization::Known(FrameId::Pz90_11)
    );
    assert_eq!(
        native.state.source(),
        SourceFrameIdentity::GlonassBroadcastPz90
    );
    let expected = reference();
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
    SpatialPoint::from_nav(&native.state).unwrap()
}

#[test]
fn r02_reaches_g2296_with_three_ordered_edges_and_weakest_status() {
    let expected = reference();
    assert_eq!(
        expected["fixture_sha256"],
        "dce340cf859c06b7785596f379f1bbf39f8042042629309adc9fbdc2f322c69e"
    );
    assert_eq!(expected["propagation_seconds"], 882.0);
    let point = r02_point();
    for (request, expected_field, expected_edges) in [
        (
            FrameRequest::Realization(FrameId::Itrf2014),
            "approx_itrf2014_position_km",
            1,
        ),
        (
            FrameRequest::Realization(FrameId::Itrf2020),
            "approx_itrf2020_position_km",
            2,
        ),
        (FrameRequest::Wgs84, "approx_g2296_position_km", 3),
    ] {
        let result = FrameTransformer
            .to_frame(&point, request, TransformOptions::default())
            .unwrap();
        assert_eq!(result.epoch, instant());
        assert_eq!(result.method, FrameMethod::UnboundedApproximate);
        assert_eq!(
            result.position_status(),
            PositionStatus::MarkedApproximation
        );
        assert_eq!(result.edge_ids.len(), expected_edges);
        assert_eq!(result.info().len(), expected_edges);
        assert_eq!(result.info()[0].source, FrameId::Pz90_11);
        assert_eq!(result.info()[0].target, FrameId::Itrf2014);
        assert_eq!(result.info()[0].method, FrameMethod::UnboundedApproximate);
        assert_eq!(result.velocity_km_s, None);
        assert!(result.position_accuracy_note.unwrap().contains("CAUTION"));
        assert_xyz(result.position_km, xyz(&expected, expected_field), 1e-9);
        if expected_edges >= 2 {
            assert_eq!(result.info()[1].source, FrameId::Itrf2014);
            assert_eq!(result.info()[1].target, FrameId::Itrf2020);
            assert_eq!(result.info()[1].method, FrameMethod::Helmert);
        }
        if expected_edges == 3 {
            assert_eq!(result.info()[2].source, FrameId::Itrf2020);
            assert_eq!(result.info()[2].target, FrameId::Wgs84G2296);
            assert_eq!(
                result.target_realization,
                FrameRealization::Known(FrameId::Wgs84G2296)
            );
            assert_eq!(result.target, SourceFrameIdentity::GpsBroadcastWgs84);
        }
    }
}

#[test]
fn r02_strict_requests_reject_the_composed_approximation() {
    let point = r02_point();
    let request = FrameRequest::Wgs84;
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
                .to_frame(&point, request, options)
                .unwrap_err(),
            error
        );
    }
    assert!(matches!(FrameTransformer.to_frame(&point, request,
        TransformOptions { warnings_as_errors: true, ..Default::default() }),
        Err(FrameError::WarningRejected(note)) if note.contains("CAUTION")));
}

#[test]
fn composed_route_requires_both_pz_and_g2296_date_windows() {
    let expected = reference();
    for (date, allowed) in [
        ("2024-03-03T23:59:59 UTC", false),
        ("2024-03-04T00:00:00 UTC", true),
        ("2024-12-31T23:59:59 UTC", true),
        ("2025-01-01T00:00:00 UTC", false),
    ] {
        let point = SpatialPoint::new(
            xyz(&expected, "native_position_km"),
            Epoch::from_str(date).unwrap(),
            SourceFrameIdentity::Realization(FrameId::Pz90_11),
            None,
        )
        .unwrap();
        let result =
            FrameTransformer.to_frame(&point, FrameRequest::Wgs84, TransformOptions::default());
        if allowed {
            assert_eq!(
                result.unwrap().position_status(),
                PositionStatus::MarkedApproximation,
                "{date}"
            );
        } else {
            assert_eq!(
                result.unwrap_err(),
                FrameError::OutsideCatalogWindow,
                "{date}"
            );
        }
    }
}
