//! One dated, independently calculated real GPS broadcast -> ITRF2014 path.
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

fn instant() -> Epoch {
    Epoch::from_str("2024-05-07T02:05:00 GPST").unwrap()
}

fn reference() -> serde_json::Value {
    serde_json::from_str(include_str!("reference/nav_frame_transform_expected.json")).unwrap()
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
fn real_g15_reaches_explicit_itrf2014_with_independent_position() {
    let nav = Rinex::from_file("tests/fixtures/nav_gps_g15_g2296_2024128.rnx").unwrap();
    let report = nav.nav_select_ephemeris(
        SV::from_str("G15").unwrap(),
        instant(),
        UnknownHealthPolicy::Reject,
    );
    let native = report
        .chosen()
        .unwrap()
        .spatial_state_at(instant())
        .unwrap();
    let expected = reference();
    assert_eq!(expected["sv"], "G15");
    assert_eq!(
        expected["fixture_sha256"],
        "c36899069ec3488fcf53b3a2bc001bf62ab8b87401fb83864ab999dc123af6dd"
    );
    assert_xyz(
        native.state.position_km,
        xyz(&expected, "native_position_km"),
        1e-8,
    );

    let point = SpatialPoint::from_nav(&native.state).unwrap();
    let transformed = FrameTransformer
        .to_frame(
            &point,
            FrameRequest::Realization(FrameId::Itrf2014),
            TransformOptions::default(),
        )
        .unwrap();
    assert_eq!(transformed.epoch, instant());
    assert_eq!(transformed.source, SourceFrameIdentity::GpsBroadcastWgs84);
    assert_eq!(
        transformed.source_realization,
        FrameRealization::Known(FrameId::Wgs84G2296)
    );
    assert_eq!(
        transformed.target,
        SourceFrameIdentity::Realization(FrameId::Itrf2014)
    );
    assert_eq!(
        transformed.target_realization,
        FrameRealization::Known(FrameId::Itrf2014)
    );
    assert_eq!(transformed.method, FrameMethod::Composite);
    assert_eq!(
        transformed.edge_ids,
        &["EPSG:10608", "ITRF2020:Table2:2015.0"]
    );
    assert_eq!(
        transformed.source_basis,
        SourceBasis::NavMessageAndCatalogDate
    );
    assert!(transformed.source_evidence.unwrap().contains("2024014"));
    assert_eq!(transformed.info()[0].source, FrameId::Wgs84G2296);
    assert_eq!(transformed.info()[0].target, FrameId::Itrf2020);
    assert_eq!(transformed.info()[1].target, FrameId::Itrf2014);
    assert_eq!(transformed.velocity_km_s, None);
    assert_xyz(
        transformed.position_km,
        xyz(&expected, "itrf2014_position_km"),
        1e-10,
    );
    assert!(
        (transformed.position_km[0] - native.state.position_km[0]).abs() > 1e-6,
        "the nonzero ITRF edge must change the satellite position"
    );
}

#[test]
fn direct_point_uses_the_same_entry_without_key_or_velocity() {
    let expected = reference();
    let point = SpatialPoint::new(
        xyz(&expected, "direct_itrf2020_position_km"),
        instant(),
        SourceFrameIdentity::Realization(FrameId::Itrf2020),
        None,
    )
    .unwrap();
    let transformed = FrameTransformer
        .to_frame(
            &point,
            FrameRequest::Realization(FrameId::Itrf2014),
            TransformOptions::default(),
        )
        .unwrap();
    assert_xyz(
        transformed.position_km,
        xyz(&expected, "direct_itrf2014_position_km"),
        1e-10,
    );
    assert_eq!(transformed.source_basis, SourceBasis::CallerAsserted);
    assert_eq!(transformed.source_evidence, None);
    assert_eq!(transformed.method, FrameMethod::Helmert);
    assert_eq!(transformed.velocity_km_s, None);
}

#[test]
fn missing_metadata_and_nonfinite_values_are_rejected() {
    let source = SourceFrameIdentity::Realization(FrameId::Itrf2020);
    assert_eq!(
        SpatialPoint::from_parts([1.0; 3], None, Some(source), None).unwrap_err(),
        FrameError::MissingEpoch
    );
    assert_eq!(
        SpatialPoint::from_parts([1.0; 3], Some(instant()), None, None).unwrap_err(),
        FrameError::MissingSource
    );
    assert_eq!(
        SpatialPoint::new([f64::NAN, 1.0, 2.0], instant(), source, None).unwrap_err(),
        FrameError::NonFiniteState
    );
    assert_eq!(
        SpatialPoint::new([1.0; 3], instant(), source, Some([0.0, f64::INFINITY, 0.0]))
            .unwrap_err(),
        FrameError::NonFiniteState
    );
}

#[test]
fn published_accuracy_is_not_accepted_as_a_strict_upper_bound() {
    let point = SpatialPoint::new(
        [10000.0; 3],
        instant(),
        SourceFrameIdentity::Realization(FrameId::Itrf2020),
        None,
    )
    .unwrap();
    for bound in [0.001, 1.0, 100.0] {
        assert_eq!(
            FrameTransformer
                .to_frame(
                    &point,
                    FrameRequest::Realization(FrameId::Itrf2014),
                    TransformOptions {
                        max_frame_operation_error_m: Some(bound),
                        ..Default::default()
                    },
                )
                .unwrap_err(),
            FrameError::PositionBoundUnavailable
        );
    }
    assert_eq!(
        FrameTransformer
            .to_frame(
                &point,
                FrameRequest::Realization(FrameId::Itrf2014),
                TransformOptions {
                    max_frame_operation_error_m: Some(f64::NAN),
                    ..Default::default()
                },
            )
            .unwrap_err(),
        FrameError::InvalidPositionBound
    );
}

#[test]
fn cross_frame_velocity_is_withheld_even_when_native_velocity_exists() {
    let point = SpatialPoint::new(
        [10000.0; 3],
        instant(),
        SourceFrameIdentity::GpsBroadcastWgs84,
        Some([1.0, 2.0, 3.0]),
    )
    .unwrap();
    assert_eq!(
        FrameTransformer
            .to_frame(
                &point,
                FrameRequest::Realization(FrameId::Itrf2014),
                TransformOptions {
                    require_velocity: true,
                    ..Default::default()
                },
            )
            .unwrap_err(),
        FrameError::VelocityUnavailable
    );
    let native = FrameTransformer
        .to_frame(&point, FrameRequest::Wgs84, TransformOptions::default())
        .unwrap();
    assert_eq!(native.velocity_km_s, Some([1.0, 2.0, 3.0]));
    assert_eq!(
        native.target_realization,
        FrameRealization::Known(FrameId::Wgs84G2296)
    );
}

#[test]
fn unknown_gps_version_and_other_broadcast_sources_fail_closed() {
    let old = Epoch::from_str("2022-06-08T01:00:00 GPST").unwrap();
    let gps = SpatialPoint::new(
        [10000.0; 3],
        old,
        SourceFrameIdentity::GpsBroadcastWgs84,
        None,
    )
    .unwrap();
    let identity = FrameTransformer
        .to_frame(&gps, FrameRequest::Wgs84, TransformOptions::default())
        .unwrap();
    assert_eq!(identity.target_realization, FrameRealization::Unknown);
    assert_eq!(
        FrameTransformer
            .to_frame(
                &gps,
                FrameRequest::Realization(FrameId::Itrf2014),
                TransformOptions::default(),
            )
            .unwrap_err(),
        FrameError::UnknownSourceRealization
    );
    let navic = SpatialPoint::new(
        [10000.0; 3],
        instant(),
        SourceFrameIdentity::NavicBroadcastWgs84,
        None,
    )
    .unwrap();
    assert_eq!(
        FrameTransformer
            .to_frame(&navic, FrameRequest::Wgs84, TransformOptions::default())
            .unwrap_err(),
        FrameError::UnsupportedSource(SourceFrameIdentity::NavicBroadcastWgs84)
    );
}

#[test]
fn navic_nav_state_gets_diagnostic_wgs84_without_entering_gps_frame_path() {
    let nav = Rinex::from_file("tests/fixtures/nav_navic_i02_2023071.rnx").unwrap();
    let t = Epoch::from_str("2023-03-12T00:00:00 GPST").unwrap();
    let report =
        nav.nav_select_ephemeris(SV::from_str("I02").unwrap(), t, UnknownHealthPolicy::Reject);
    let native = report.chosen().unwrap().spatial_state_at(t).unwrap();
    assert_eq!(
        native.state.source(),
        SourceFrameIdentity::NavicBroadcastWgs84
    );
    assert_eq!(
        native.state.to_frame(FrameRequest::Wgs84).unwrap_err(),
        FrameError::UnsupportedSource(SourceFrameIdentity::NavicBroadcastWgs84)
    );
    assert!(native.state.position_km.iter().all(|v| v.is_finite()));
}

#[test]
fn explicit_g2296_transform_stays_within_gps_catalogue_window() {
    let epoch = Epoch::from_str("2025-01-01T00:00:00 UTC").unwrap();
    let point = SpatialPoint::new(
        [10000.0; 3],
        epoch,
        SourceFrameIdentity::Realization(FrameId::Wgs84G2296),
        None,
    )
    .unwrap();
    assert_eq!(
        FrameTransformer
            .to_frame(
                &point,
                FrameRequest::Realization(FrameId::Itrf2020),
                TransformOptions::default(),
            )
            .unwrap_err(),
        FrameError::OutsideCatalogWindow
    );
}

#[test]
fn reverse_path_and_time_scale_preserve_the_same_instant() {
    let utc = Epoch::from_str("2024-05-07T02:04:42 UTC").unwrap();
    assert_eq!(instant(), utc);
    let point = SpatialPoint::new(
        [19224.62119621031, -2435.348503636455, 17751.84785453113],
        utc,
        SourceFrameIdentity::Realization(FrameId::Itrf2020),
        None,
    )
    .unwrap();
    let forward = FrameTransformer
        .to_frame(
            &point,
            FrameRequest::Realization(FrameId::Itrf2014),
            TransformOptions::default(),
        )
        .unwrap();
    let reverse_input = SpatialPoint::new(
        forward.position_km,
        utc,
        SourceFrameIdentity::Realization(FrameId::Itrf2014),
        None,
    )
    .unwrap();
    let reverse = FrameTransformer
        .to_frame(
            &reverse_input,
            FrameRequest::Realization(FrameId::Itrf2020),
            TransformOptions::default(),
        )
        .unwrap();
    assert_xyz(reverse.position_km, point.position_km, 1e-9);
    assert_eq!(reverse.epoch, utc);
    assert_eq!(reverse.info()[0].source, FrameId::Itrf2014);
    assert_eq!(reverse.info()[0].target, FrameId::Itrf2020);

    let broadcast = FrameTransformer
        .to_frame(
            &reverse_input,
            FrameRequest::Wgs84,
            TransformOptions::default(),
        )
        .unwrap();
    assert_eq!(broadcast.target, SourceFrameIdentity::GpsBroadcastWgs84);
    assert_eq!(
        broadcast.target_realization,
        FrameRealization::Known(FrameId::Wgs84G2296)
    );
    assert_eq!(broadcast.info()[1].id, "EPSG:10608:inverse");
}

#[test]
fn itrf_only_edge_uses_its_own_window() {
    let too_early = Epoch::from_str("2024-03-03T23:59:59 UTC").unwrap();
    let point = SpatialPoint::new(
        [10000.0; 3],
        too_early,
        SourceFrameIdentity::Realization(FrameId::Itrf2020),
        None,
    )
    .unwrap();
    let transformed = FrameTransformer
        .to_frame(
            &point,
            FrameRequest::Realization(FrameId::Itrf2014),
            TransformOptions::default(),
        )
        .unwrap();
    assert_eq!(transformed.info()[0].source, FrameId::Itrf2020);
    assert_eq!(transformed.info()[0].target, FrameId::Itrf2014);
    let too_early = Epoch::from_str("2014-12-31T23:59:59 UTC").unwrap();
    let early = SpatialPoint::new(point.position_km, too_early, point.source, None).unwrap();
    assert_eq!(
        FrameTransformer
            .to_frame(
                &early,
                FrameRequest::Realization(FrameId::Itrf2014),
                TransformOptions::default(),
            )
            .unwrap_err(),
        FrameError::OutsideCatalogWindow
    );
}
