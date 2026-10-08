//! Reduced public DLR NAV sample: selection, native state, and frame evidence.
#![cfg(feature = "nav")]
use rinex::{
    navigation::rinex::{
        selection::{NavRejection, UnknownHealthPolicy},
        spatial_state::{
            FrameError, FrameId, FrameRealization, FrameRequest, FrameTransformer, PositionStatus,
            SourceFrameIdentity, SpatialPoint, TransformOptions,
        },
    },
    prelude::{Epoch, Rinex, SV},
};
use std::str::FromStr;

const NAV: &str = "tests/fixtures/nav_mixed_2024131_first_epoch.rnx";

#[test]
fn representative_real_records_keep_selection_propagation_and_frame_errors_distinct() {
    let nav = Rinex::from_file(NAV).unwrap();
    assert_eq!(nav.nav_ephemeris_frames_iter().count(), 11);
    assert_eq!(nav.nav_parse_report().rejected_records(), 0);
    assert_eq!(nav.nav_parse_report().unsupported_records(), 0);
    let t = Epoch::from_str("2024-05-10T03:00:00 GPST").unwrap();
    for (sv, source, realization, expected_reason) in [
        (
            "G04",
            SourceFrameIdentity::GpsBroadcastWgs84,
            FrameRealization::Known(FrameId::Wgs84G2296),
            None,
        ),
        (
            "R02",
            SourceFrameIdentity::GlonassBroadcastPz90,
            FrameRealization::Known(FrameId::Pz90_11),
            None,
        ),
        (
            "E03",
            SourceFrameIdentity::GalileoBroadcastGtrf,
            FrameRealization::Known(FrameId::GalileoGtrf23v01),
            None,
        ),
        (
            "C02",
            SourceFrameIdentity::BeidouBroadcast,
            FrameRealization::Unknown,
            Some(FrameError::UnknownSourceRealization),
        ),
        (
            "I03",
            SourceFrameIdentity::NavicBroadcastWgs84,
            FrameRealization::Unknown,
            Some(FrameError::UnsupportedSource(
                SourceFrameIdentity::NavicBroadcastWgs84,
            )),
        ),
    ] {
        let sv = SV::from_str(sv).unwrap();
        let report = nav.nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject);
        let chosen = report.chosen().unwrap();
        let native = chosen.spatial_state_at(t).unwrap();
        assert_eq!(native.state.source(), source, "{sv}");
        assert_eq!(native.state.realization(), realization, "{sv}");
        assert!(
            native.state.position_km.iter().all(|x| x.is_finite()),
            "{sv}"
        );
        let point = SpatialPoint::from_nav(&native.state).unwrap();
        let converted =
            FrameTransformer.to_frame(&point, FrameRequest::Wgs84, TransformOptions::default());
        if sv == SV::from_str("R02").unwrap() || sv == SV::from_str("E03").unwrap() {
            let result = converted.unwrap();
            assert_eq!(
                result.position_status(),
                PositionStatus::MarkedApproximation
            );
            assert_eq!(result.edge_ids.len(), if sv.prn == 2 { 3 } else { 2 });
        } else if let Some(reason) = expected_reason {
            assert_eq!(converted.unwrap_err(), reason, "{sv}");
            assert!(native.state.position_km.iter().all(|x| x.is_finite()));
        } else {
            let result = converted.unwrap();
            assert_eq!(result.position_status(), PositionStatus::NativeIdentity);
            assert_eq!(result.position_km, native.state.position_km);
            assert_eq!(result.source_realization, realization);
        }
    }
    let j04 =
        nav.nav_select_ephemeris(SV::from_str("J04").unwrap(), t, UnknownHealthPolicy::Reject);
    assert!(j04.chosen().is_none());
    assert!(j04
        .candidates
        .iter()
        .any(|c| c.rejection == Some(NavRejection::Unhealthy)));
    assert!(j04
        .candidates
        .iter()
        .any(|c| c.rejection == Some(NavRejection::UnsupportedMessage)));
}
