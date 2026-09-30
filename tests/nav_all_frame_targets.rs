//! All installed frame IDs: dated reverse assumptions and mixed-NAV coverage.
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

const TARGETS: [FrameId; 8] = [
    FrameId::Wgs84G2296,
    FrameId::Itrf2020,
    FrameId::Itrf2014,
    FrameId::Pz90_11,
    FrameId::QzssJgsItrf2014Aligned,
    FrameId::QzssJgsItrf2020Aligned,
    FrameId::GalileoGtrf23v01,
    FrameId::Bdcs2019v01,
];

fn xyz(expected: &str, row: usize, field: &str) -> [f64; 3] {
    let data: serde_json::Value = serde_json::from_str(expected).unwrap();
    std::array::from_fn(|i| data[row][field][i].as_f64().unwrap())
}

#[test]
fn dated_reverse_zero_offset_edges_keep_real_reference_xyz_and_approximate_status() {
    let cases = [
        (
            FrameId::Itrf2014,
            FrameId::QzssJgsItrf2014Aligned,
            "2023-03-12T00:00:00 QZSST",
            xyz(
                include_str!("reference/nav_qzss_lnav_expected.json"),
                1,
                "position_km",
            ),
            "QZSS:PNT:ITRF2014-to-JGS-alignment:inverse-zero-offset-approx",
        ),
        (
            FrameId::Itrf2020,
            FrameId::QzssJgsItrf2020Aligned,
            "2024-05-10T03:00:00 GPST",
            [13527.734303005513, 20608.98850174074, -16393.79374112982],
            "QZSS:PNT:ITRF2020-to-JGS-alignment:inverse-zero-offset-approx",
        ),
        (
            FrameId::Itrf2020,
            FrameId::GalileoGtrf23v01,
            "2024-05-07T00:30:00 GST",
            xyz(
                include_str!("reference/nav_galileo_frame_expected.json"),
                1,
                "native_position_km",
            ),
            "ESA:GGSP:ITRF2020-to-GTRF23v01:inverse-zero-offset-approx",
        ),
        (
            FrameId::Itrf2014,
            FrameId::Bdcs2019v01,
            "2022-06-08T09:00:00 BDT",
            xyz(
                include_str!("reference/nav_bds_geo_frame_expected.json"),
                1,
                "itrf2014_position_km",
            ),
            "rinex:ITRF2014-BDCS2019v01:inverse-zero-offset-approx",
        ),
    ];
    for (source, target, instant, input, edge_id) in cases {
        let point = SpatialPoint::new(
            input,
            Epoch::from_str(instant).unwrap(),
            SourceFrameIdentity::Realization(source),
            None,
        )
        .unwrap();
        let request = FrameRequest::Realization(target);
        let result = FrameTransformer
            .to_frame(&point, request, TransformOptions::default())
            .unwrap();
        assert_eq!(result.position_km, input, "{target:?}");
        assert_eq!(
            result.position_status(),
            PositionStatus::MarkedApproximation,
            "{target:?}"
        );
        assert_eq!(result.edge_ids, [edge_id], "{target:?}");
        assert_eq!(result.info()[0].source, source);
        assert_eq!(result.info()[0].target, target);
        assert_eq!(result.source_realization, FrameRealization::Known(source));
        assert_eq!(result.target_realization, FrameRealization::Known(target));
        assert!(
            result
                .cautions()
                .iter()
                .any(|s| s.contains(edge_id) && s.contains("satellite")),
            "{target:?}"
        );
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
                error,
                "{target:?}"
            );
        }
        assert!(matches!(
            FrameTransformer.to_frame(
                &point,
                request,
                TransformOptions {
                    warnings_as_errors: true,
                    ..Default::default()
                }
            ),
            Err(FrameError::WarningRejected(_))
        ));
    }
}

#[test]
fn reverse_assumptions_stop_at_their_catalog_windows() {
    for (source, target, instant) in [
        (
            FrameId::Itrf2014,
            FrameId::QzssJgsItrf2014Aligned,
            "2024-05-10T03:00:00 GPST",
        ),
        (
            FrameId::Itrf2020,
            FrameId::QzssJgsItrf2020Aligned,
            "2023-03-12T00:00:00 QZSST",
        ),
        (
            FrameId::Itrf2020,
            FrameId::GalileoGtrf23v01,
            "2024-06-01T00:00:00 UTC",
        ),
        (
            FrameId::Itrf2014,
            FrameId::Bdcs2019v01,
            "2024-05-10T03:00:00 GPST",
        ),
    ] {
        let point = SpatialPoint::new(
            [14000.0, 23000.0, -16000.0],
            Epoch::from_str(instant).unwrap(),
            SourceFrameIdentity::Realization(source),
            None,
        )
        .unwrap();
        assert_eq!(
            FrameTransformer
                .to_frame(
                    &point,
                    FrameRequest::Realization(target),
                    TransformOptions::default()
                )
                .unwrap_err(),
            FrameError::OutsideCatalogWindow,
            "{target:?}"
        );
    }
}

#[test]
fn representative_nav_requests_cover_evidenced_paths_and_explain_missing_ones() {
    let nav = Rinex::from_file("tests/fixtures/nav_mixed_2024131_first_epoch.rnx").unwrap();
    let instant = Epoch::from_str("2024-05-10T03:00:00 GPST").unwrap();
    let selected = nav.nav_select_ephemeris(
        SV::from_str("E03").unwrap(),
        instant,
        UnknownHealthPolicy::Reject,
    );
    let native = selected
        .chosen()
        .unwrap()
        .spatial_state_at(instant)
        .unwrap();
    let point = SpatialPoint::from_nav(&native.state).unwrap();
    for target in TARGETS {
        let output = FrameTransformer.to_frame(
            &point,
            FrameRequest::Realization(target),
            TransformOptions::default(),
        );
        if target == FrameId::QzssJgsItrf2014Aligned || target == FrameId::Bdcs2019v01 {
            assert_eq!(
                output.unwrap_err(),
                FrameError::OutsideCatalogWindow,
                "{target:?}"
            );
        } else {
            let result = output.unwrap();
            assert_eq!(result.target_realization, FrameRealization::Known(target));
            assert!(!result.edge_ids.is_empty() || target == FrameId::GalileoGtrf23v01);
        }
    }
}
