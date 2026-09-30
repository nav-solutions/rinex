//! N09d: QZSS J02 LNAV JGS to marked ITRF2014 position approximation.
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

const FIXTURE: &str = "tests/fixtures/nav_qzss_j02_2023071.rnx";
const EXPECTED: &str = include_str!("reference/nav_qzss_lnav_expected.json");

fn position(row: &serde_json::Value) -> [f64; 3] {
    std::array::from_fn(|i| row["position_km"][i].as_f64().unwrap())
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

#[test]
fn real_j02_reaches_marked_itrf2014_approximation_at_its_own_epoch() {
    let nav = Rinex::from_file(FIXTURE).unwrap();
    let sv = SV::from_str("J02").unwrap();
    let reference: serde_json::Value = serde_json::from_str(EXPECTED).unwrap();
    let toe = Epoch::from_str("2023-03-12T00:00:00 QZSST").unwrap();
    let target = FrameRequest::Realization(FrameId::Itrf2014);
    let mut maximum_km: f64 = 0.0;

    for row in reference.as_array().unwrap() {
        let epoch = toe + Duration::from_seconds(row["offset_s"].as_f64().unwrap());
        let selected = nav.nav_select_ephemeris(sv, epoch, UnknownHealthPolicy::Reject);
        let native = selected.chosen().unwrap().spatial_state_at(epoch).unwrap();
        assert_eq!(native.key.sv, sv);
        assert_eq!(native.key.msgtype, NavMessageType::LNAV);
        assert_eq!(native.record_epoch, toe);
        assert_eq!(native.orbit_reference, Some(toe));
        assert_eq!(
            native.state.realization(),
            FrameRealization::Known(FrameId::QzssJgsItrf2014Aligned)
        );
        assert!(native.state.source_evidence().unwrap().contains("QZSS:PNT"));

        // The independent Python reference reads the raw RINEX fields and
        // propagates J02. Zero correction is the explicitly marked model, so
        // these same independently computed numbers are its target XYZ.
        maximum_km = maximum_km.max(assert_xyz(native.state.position_km, position(row), 1e-6));
        let result = FrameTransformer
            .to_frame(
                &SpatialPoint::from_nav(&native.state).unwrap(),
                target,
                TransformOptions::default(),
            )
            .unwrap();
        assert_eq!(result.epoch, epoch);
        assert_eq!(result.source, SourceFrameIdentity::QzssBroadcastJgs);
        assert_eq!(
            result.source_realization,
            FrameRealization::Known(FrameId::QzssJgsItrf2014Aligned)
        );
        assert_eq!(
            result.target,
            SourceFrameIdentity::Realization(FrameId::Itrf2014)
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
            &["QZSS:PNT:JGS-ITRF2014-alignment:zero-offset-approx"]
        );
        assert_eq!(result.info()[0].source, FrameId::QzssJgsItrf2014Aligned);
        assert_eq!(result.info()[0].target, FrameId::Itrf2014);
        assert_eq!(result.info()[0].method, FrameMethod::UnboundedApproximate);
        assert!(result.position_accuracy_note.unwrap().contains("CAUTION"));
        assert_eq!(result.velocity_km_s, None);
        maximum_km = maximum_km.max(assert_xyz(result.position_km, position(row), 1e-6));
    }
    println!("maximum XYZ difference from independent broadcast reference: {maximum_km:.3e} km");
}

#[test]
fn caller_asserted_jgs_uses_the_same_edge_and_rejects_unsupported_requests() {
    let reference: serde_json::Value = serde_json::from_str(EXPECTED).unwrap();
    let native_km = position(&reference[1]);
    let epoch = Epoch::from_str("2023-03-12T00:00:00 QZSST").unwrap();
    let asserted = SpatialPoint::new(
        native_km,
        epoch,
        SourceFrameIdentity::Realization(FrameId::QzssJgsItrf2014Aligned),
        Some([0.1, 0.2, 0.3]),
    )
    .unwrap();
    let target = FrameRequest::Realization(FrameId::Itrf2014);
    let result = FrameTransformer
        .to_frame(&asserted, target, TransformOptions::default())
        .unwrap();
    assert_xyz(result.position_km, native_km, 1e-12);
    assert_eq!(result.method, FrameMethod::UnboundedApproximate);
    assert_eq!(result.source_basis, SourceBasis::CallerAsserted);
    assert_eq!(result.source_evidence, None);
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
        epoch,
        SourceFrameIdentity::QzssBroadcastJgs,
        None,
    )
    .unwrap();
    assert_eq!(
        FrameTransformer
            .to_frame(&unknown, target, TransformOptions::default())
            .unwrap_err(),
        FrameError::UnknownSourceRealization
    );
    let composed = FrameTransformer
        .to_frame(
            &asserted,
            FrameRequest::Realization(FrameId::Itrf2020),
            TransformOptions::default(),
        )
        .unwrap();
    assert_eq!(
        composed.position_status(),
        rinex::navigation::rinex::spatial_state::PositionStatus::MarkedApproximation
    );
    assert_eq!(composed.edge_ids.len(), 2);
    assert_eq!(composed.info()[0].source, FrameId::QzssJgsItrf2014Aligned);
    assert_eq!(composed.info()[1].target, FrameId::Itrf2020);
}

#[test]
fn asserted_jgs_outside_documented_window_is_rejected() {
    for epoch in ["2021-02-15T23:59:59 UTC", "2023-11-09T00:00:00 UTC"] {
        let point = SpatialPoint::new(
            [10000.0, 20000.0, 30000.0],
            Epoch::from_str(epoch).unwrap(),
            SourceFrameIdentity::Realization(FrameId::QzssJgsItrf2014Aligned),
            None,
        )
        .unwrap();
        assert_eq!(
            FrameTransformer
                .to_frame(
                    &point,
                    FrameRequest::Realization(FrameId::Itrf2014),
                    TransformOptions::default(),
                )
                .unwrap_err(),
            FrameError::OutsideCatalogWindow
        );
    }
}
