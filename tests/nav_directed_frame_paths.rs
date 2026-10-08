//! Selected real NAV records and explicit frame-evidence boundaries.
#![cfg(feature = "nav")]
use rinex::{
    navigation::{
        rinex::{
            selection::{NavRejection, UnknownHealthPolicy},
            spatial_state::{
                FrameError, FrameId, FrameRealization, FrameRequest, FrameTransformer,
                MethodPolicy, PositionStatus, SourceFrameIdentity, SpatialPoint, TransformOptions,
            },
        },
        OrbitItem,
    },
    prelude::{Epoch, Rinex, SV},
};
use std::str::FromStr;

const NAV: &str = "tests/fixtures/nav_mixed_2024131_first_epoch.rnx";
fn instant() -> Epoch {
    Epoch::from_str("2024-05-10T03:00:00 GPST").unwrap()
}

#[test]
fn missing_evidence_returns_error_without_relabeling_native_xyz() {
    let nav = Rinex::from_file(NAV).unwrap();
    for (sv, target, reason) in [
        (
            "C02",
            FrameId::Itrf2020,
            FrameError::UnknownSourceRealization,
        ),
        (
            "I03",
            FrameId::Itrf2014,
            FrameError::UnsupportedSource(SourceFrameIdentity::NavicBroadcastWgs84),
        ),
        (
            "E03",
            FrameId::QzssJgsItrf2014Aligned,
            FrameError::OutsideCatalogWindow,
        ),
    ] {
        let selected = nav.nav_select_ephemeris(
            SV::from_str(sv).unwrap(),
            instant(),
            UnknownHealthPolicy::Reject,
        );
        let native = selected
            .chosen()
            .unwrap()
            .spatial_state_at(instant())
            .unwrap();
        let xyz = native.state.position_km;
        let source = native.state.source();
        assert!(xyz.iter().all(|v| v.is_finite()));
        assert_eq!(
            native
                .state
                .to_frame(FrameRequest::Realization(target))
                .unwrap_err(),
            reason
        );
        assert_eq!(native.state.source(), source);
        assert_eq!(native.state.position_km, xyz);
    }
    let unknown = nav.nav_select_ephemeris(
        SV::from_str("C02").unwrap(),
        instant(),
        UnknownHealthPolicy::Reject,
    );
    assert_eq!(
        unknown
            .chosen()
            .unwrap()
            .spatial_state_at(instant())
            .unwrap()
            .state
            .realization(),
        FrameRealization::Unknown
    );
}

#[test]
fn strict_options_apply_to_evidenced_paths_only() {
    let nav = Rinex::from_file(NAV).unwrap();
    let selected = nav.nav_select_ephemeris(
        SV::from_str("E03").unwrap(),
        instant(),
        UnknownHealthPolicy::Reject,
    );
    let native = selected
        .chosen()
        .unwrap()
        .spatial_state_at(instant())
        .unwrap();
    let point = SpatialPoint::from_nav(&native.state).unwrap();
    let target = FrameRequest::Realization(FrameId::Itrf2020);
    let result = FrameTransformer
        .to_frame(&point, target, TransformOptions::default())
        .unwrap();
    assert_eq!(
        result.position_status(),
        PositionStatus::MarkedApproximation
    );
    assert_eq!(
        result.edge_ids,
        ["ESA:GGSP:GTRF23v01-ITRF2020:zero-offset-approx"]
    );
    assert!(result
        .cautions()
        .iter()
        .any(|note| note.contains("CAUTION")));
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
    let asserted = SpatialPoint::new(
        [10000.0, 20000.0, 30000.0],
        instant(),
        SourceFrameIdentity::Realization(FrameId::Itrf2020),
        None,
    )
    .unwrap();
    let numerical = FrameTransformer
        .to_frame(
            &asserted,
            FrameRequest::Realization(FrameId::Itrf2014),
            TransformOptions::default(),
        )
        .unwrap();
    assert_eq!(
        numerical.position_status(),
        PositionStatus::NumericalTransform
    );
    assert!(numerical
        .cautions()
        .iter()
        .any(|note| note.contains("NAV health")));
}

#[test]
fn unknown_health_requires_opt_in_and_invalid_data_still_rejects() {
    let mut nav = Rinex::from_file(NAV).unwrap();
    let sv = SV::from_str("G04").unwrap();
    for (key, frame) in nav.record.as_mut_nav().unwrap().iter_mut() {
        if key.sv == sv {
            frame.as_mut_ephemeris().unwrap().orbits.remove("health");
        }
    }
    let rejected = nav.nav_select_ephemeris(sv, instant(), UnknownHealthPolicy::Reject);
    assert!(rejected.chosen().is_none());
    assert!(rejected
        .candidates
        .iter()
        .any(|c| c.rejection == Some(NavRejection::UnknownHealth)));
    let allowed = nav.nav_select_ephemeris(sv, instant(), UnknownHealthPolicy::Allow);
    assert!(allowed
        .chosen()
        .unwrap()
        .spatial_state_at(instant())
        .is_ok());
    let mut invalid = nav.clone();
    for (key, frame) in invalid.record.as_mut_nav().unwrap().iter_mut() {
        if key.sv == sv {
            frame
                .as_mut_ephemeris()
                .unwrap()
                .orbits
                .insert("dataValidity".into(), OrbitItem::F64(1.0));
        }
    }
    let report = invalid.nav_select_ephemeris(sv, instant(), UnknownHealthPolicy::Allow);
    assert!(report.chosen().is_none());
    assert!(report
        .candidates
        .iter()
        .any(|c| c.rejection == Some(NavRejection::InvalidData)));
}
