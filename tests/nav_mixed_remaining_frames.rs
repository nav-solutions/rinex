//! Remaining dated first-epoch frame paths and explicit evidence limits.
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

const NAV: &str = "tests/fixtures/nav_mixed_2024131_first_epoch.rnx";

fn epoch() -> Epoch {
    Epoch::from_str("2024-05-10T03:00:00 GPST").unwrap()
}

fn assert_xyz(actual: [f64; 3], expected: [f64; 3]) {
    for (got, want) in actual.into_iter().zip(expected) {
        assert!((got - want).abs() < 1e-8, "{got} != {want} km");
    }
}

#[test]
fn e03_reaches_g2296_with_independent_raw_orbit_reference() {
    let nav = Rinex::from_file(NAV).unwrap();
    let selected = nav.nav_select_ephemeris(
        SV::from_str("E03").unwrap(),
        epoch(),
        UnknownHealthPolicy::Reject,
    );
    let native = selected
        .chosen()
        .unwrap()
        .spatial_state_at(epoch())
        .unwrap();
    assert_eq!(
        native.state.realization(),
        FrameRealization::Known(FrameId::GalileoGtrf23v01)
    );
    // tests/reference/nav_galileo_e03_mixed_frame.py reads the selected raw block
    // at original source line 870 and independently evaluates the orbit.
    let expected = [13527.734303005513, 20608.988501740743, -16393.79374112982];
    assert_xyz(native.state.position_km, expected);
    let point = SpatialPoint::from_nav(&native.state).unwrap();
    let result = FrameTransformer
        .to_frame(&point, FrameRequest::Wgs84, TransformOptions::default())
        .unwrap();
    assert_xyz(result.position_km, expected);
    assert_eq!(
        result.position_status(),
        PositionStatus::MarkedApproximation
    );
    assert_eq!(
        result.edge_ids,
        &[
            "ESA:GGSP:GTRF23v01-ITRF2020:zero-offset-approx",
            "EPSG:10608:inverse"
        ]
    );
    assert_eq!(result.info()[0].source, FrameId::GalileoGtrf23v01);
    assert_eq!(result.info()[1].target, FrameId::Wgs84G2296);
    assert!(result.position_accuracy_note.unwrap().contains("CAUTION"));
    assert_eq!(result.velocity_km_s, None);
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
                warnings_as_errors: true,
                ..Default::default()
            },
            FrameError::WarningRejected(result.position_accuracy_note.unwrap()),
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
                .to_frame(&point, FrameRequest::Wgs84, options)
                .unwrap_err(),
            error
        );
    }
}

#[test]
fn jgs2020_alignment_is_dated_approximation_and_j04_remains_unselected() {
    let nav = Rinex::from_file(NAV).unwrap();
    let selected = nav.nav_select_ephemeris(
        SV::from_str("J04").unwrap(),
        epoch(),
        UnknownHealthPolicy::Reject,
    );
    assert!(selected.chosen().is_none());
    let xyz = [10000.0, 20000.0, 30000.0];
    let asserted = SpatialPoint::new(
        xyz,
        epoch(),
        SourceFrameIdentity::Realization(FrameId::QzssJgsItrf2020Aligned),
        Some([0.1, 0.2, 0.3]),
    )
    .unwrap();
    let result = FrameTransformer
        .to_frame(&asserted, FrameRequest::Wgs84, TransformOptions::default())
        .unwrap();
    assert_eq!(result.position_km, xyz);
    assert_eq!(
        result.source_realization,
        FrameRealization::Known(FrameId::QzssJgsItrf2020Aligned)
    );
    assert_eq!(
        result.position_status(),
        PositionStatus::MarkedApproximation
    );
    assert_eq!(
        result.edge_ids,
        &[
            "QZSS:PNT:JGS-ITRF2020-alignment:zero-offset-approx",
            "EPSG:10608:inverse"
        ]
    );
    assert_eq!(result.velocity_km_s, None);
    // ITRF2020 Table 2 evaluated independently at 2024-05-10 02:59:42 UTC.
    let itrf2014 = FrameTransformer
        .to_frame(
            &asserted,
            FrameRequest::Realization(FrameId::Itrf2014),
            TransformOptions::default(),
        )
        .unwrap();
    assert_xyz(
        itrf2014.position_km,
        [9999.9999944, 19999.99998976444, 29999.999990671113],
    );
    assert_eq!(
        itrf2014.position_status(),
        PositionStatus::MarkedApproximation
    );
    assert_eq!(itrf2014.edge_ids.len(), 2);
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
                warnings_as_errors: true,
                ..Default::default()
            },
            FrameError::WarningRejected(result.position_accuracy_note.unwrap()),
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
                .to_frame(&asserted, FrameRequest::Wgs84, options)
                .unwrap_err(),
            error
        );
    }
    for text in ["2023-11-10T23:59:59 UTC", "2025-01-01T00:00:00 UTC"] {
        let point = SpatialPoint::new(
            xyz,
            Epoch::from_str(text).unwrap(),
            SourceFrameIdentity::Realization(FrameId::QzssJgsItrf2020Aligned),
            None,
        )
        .unwrap();
        assert_eq!(
            FrameTransformer
                .to_frame(
                    &point,
                    FrameRequest::Realization(FrameId::Itrf2020),
                    TransformOptions::default()
                )
                .unwrap_err(),
            FrameError::OutsideCatalogWindow
        );
    }
    let unresolved =
        SpatialPoint::new(xyz, epoch(), SourceFrameIdentity::QzssBroadcastJgs, None).unwrap();
    assert_eq!(
        FrameTransformer
            .to_frame(
                &unresolved,
                FrameRequest::Wgs84,
                TransformOptions::default()
            )
            .unwrap_err(),
        FrameError::UnknownSourceRealization
    );
}

#[test]
fn unresolved_2024_sources_return_original_error_and_keep_native_xyz() {
    let nav = Rinex::from_file(NAV).unwrap();
    for (label, error) in [
        ("C02", FrameError::UnknownSourceRealization),
        (
            "I03",
            FrameError::UnsupportedSource(SourceFrameIdentity::NavicBroadcastWgs84),
        ),
    ] {
        let selected = nav.nav_select_ephemeris(
            SV::from_str(label).unwrap(),
            epoch(),
            UnknownHealthPolicy::Reject,
        );
        let native = selected
            .chosen()
            .unwrap()
            .spatial_state_at(epoch())
            .unwrap();
        assert!(native.state.position_km.iter().all(|v| v.is_finite()));
        let point = SpatialPoint::from_nav(&native.state).unwrap();
        assert_eq!(
            FrameTransformer
                .to_frame(&point, FrameRequest::Wgs84, TransformOptions::default())
                .unwrap_err(),
            error
        );
        assert_eq!(
            native.state.to_frame(FrameRequest::Wgs84).unwrap_err(),
            error
        );
        assert_eq!(native.state.realization(), FrameRealization::Unknown);
    }
}
