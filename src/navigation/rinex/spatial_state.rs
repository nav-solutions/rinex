//! Message-aware native NAV state and a dated terrestrial-frame conversion.
//!
//! A frame family name alone is not a frame realization: GPS and NavIC
//! broadcasts can both carry WGS-84-family coordinates.
use super::{
    glonass_fdma::FdmaError,
    legacy_kepler::KeplerStateError,
    sbas::SbasError,
    selection::{NativeFrame, NavCandidate, NavRejection},
};
use crate::{
    navigation::{NavKey, NavMessageType},
    prelude::{Constellation, Epoch},
};

/// The broadcasting system that defines the native terrestrial axes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceFrameIdentity {
    GpsBroadcastWgs84,
    NavicBroadcastWgs84,
    QzssBroadcastJgs,
    GlonassBroadcastPz90,
    GalileoBroadcastGtrf,
    BeidouBroadcast,
    SbasBroadcast,
    /// A concrete terrestrial realization asserted by a non-NAV caller.
    Realization(FrameId),
}

/// Concrete Earth-fixed realizations supported by the fixed catalogue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameId {
    Wgs84G2296,
    Pz90_11,
    /// QZSS PNT JGS aligned to ITRF2014 in the documented 2021–2023 period.
    QzssJgsItrf2014Aligned,
    /// QZSS PNT JGS using the ITRF2020 reference from November 2023.
    QzssJgsItrf2020Aligned,
    /// Galileo GTRF23v01, aligned to ITRF2020 from 2023-05-05.
    GalileoGtrf23v01,
    /// BeiDou BDCS(2019v01), inferred for dated C10/C20 D1 and C05 D2 samples.
    Bdcs2019v01,
    Itrf2020,
    Itrf2014,
}

/// RINEX NAV alone does not identify a particular terrestrial-frame realization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameRealization {
    Unknown,
    Known(FrameId),
}

/// Minimal state needed for a later frame conversion, independent of the NAV file.
#[derive(Clone, Copy, Debug)]
pub struct BroadcastFixedState {
    pub epoch: Epoch,
    native_frame: NativeFrame,
    source: SourceFrameIdentity,
    realization: FrameRealization,
    source_evidence: Option<&'static str>,
    pub position_km: [f64; 3],
    pub velocity_km_s: [f64; 3],
}

/// Owned message identity and reference times accompany the native state.
#[derive(Clone, Copy, Debug)]
pub struct NavSpatialState {
    pub key: NavKey,
    pub orbit_reference: Option<Epoch>,
    pub clock_reference: Epoch,
    pub record_epoch: Epoch,
    pub state: BroadcastFixedState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpatialStateError {
    Rejected(NavRejection),
    UnsupportedMessage,
    Kepler(KeplerStateError),
    Fdma(FdmaError),
    Sbas(SbasError),
}

impl std::fmt::Display for SpatialStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SpatialStateError {}

impl NavCandidate<'_> {
    /// Reuse the selected message's verified propagator; do not reselect it.
    pub fn spatial_state_at(&self, t: Epoch) -> Result<NavSpatialState, SpatialStateError> {
        if let Some(reason) = self.rejection {
            return Err(SpatialStateError::Rejected(reason));
        }
        let (source, native_frame, position_km, velocity_km_s) =
            if self.key.sv.constellation.is_sbas()
                && matches!(
                    self.key.msgtype,
                    NavMessageType::SBAS | NavMessageType::LNAV
                )
            {
                let s = self.sbas_state_at(t).map_err(SpatialStateError::Sbas)?;
                (
                    SourceFrameIdentity::SbasBroadcast,
                    s.frame,
                    s.position_km,
                    s.velocity_km_s,
                )
            } else {
                match (self.key.sv.constellation, self.key.msgtype) {
                    (Constellation::Glonass, NavMessageType::FDMA | NavMessageType::LNAV) => {
                        let s = self.fdma_state_at(t).map_err(SpatialStateError::Fdma)?;
                        (
                            SourceFrameIdentity::GlonassBroadcastPz90,
                            s.frame,
                            s.position_km,
                            s.velocity_km_s,
                        )
                    },
                    (constellation, message)
                        if matches!(
                            (constellation, message),
                            (
                                Constellation::GPS | Constellation::QZSS | Constellation::IRNSS,
                                NavMessageType::LNAV
                            ) | (
                                Constellation::Galileo,
                                NavMessageType::INAV | NavMessageType::FNAV
                            ) | (
                                Constellation::BeiDou,
                                NavMessageType::D1 | NavMessageType::D2
                            )
                        ) =>
                    {
                        let source = match constellation {
                            Constellation::GPS => SourceFrameIdentity::GpsBroadcastWgs84,
                            Constellation::IRNSS => SourceFrameIdentity::NavicBroadcastWgs84,
                            Constellation::QZSS => SourceFrameIdentity::QzssBroadcastJgs,
                            Constellation::Galileo => SourceFrameIdentity::GalileoBroadcastGtrf,
                            Constellation::BeiDou => SourceFrameIdentity::BeidouBroadcast,
                            _ => return Err(SpatialStateError::UnsupportedMessage),
                        };
                        let s = self.kepler_state_at(t).map_err(SpatialStateError::Kepler)?;
                        (source, s.frame, s.position_km, s.velocity_km_s)
                    },
                    _ => return Err(SpatialStateError::UnsupportedMessage),
                }
            };
        // The record, orbit reference, and requested state instant must all
        // fall in the conservatively documented PZ-90.11 broadcast period.
        let (realization, source_evidence) = if source == SourceFrameIdentity::GpsBroadcastWgs84
            && self.orbit_reference.is_some_and(in_g2296_window)
            && in_g2296_window(self.key.epoch)
            && in_g2296_window(t)
        {
            (
                FrameRealization::Known(FrameId::Wgs84G2296),
                Some(G2296_SOURCE_EVIDENCE),
            )
        } else if source == SourceFrameIdentity::GlonassBroadcastPz90
            && self.orbit_reference.is_some_and(in_pz9011_window)
            && in_pz9011_window(self.key.epoch)
            && in_pz9011_window(t)
        {
            (
                FrameRealization::Known(FrameId::Pz90_11),
                Some(PZ9011_SOURCE_EVIDENCE),
            )
        } else if source == SourceFrameIdentity::QzssBroadcastJgs
            && self.key.msgtype == NavMessageType::LNAV
            && qzss_jgs2014_resolved(self.key.epoch, self.orbit_reference, t)
        {
            (
                FrameRealization::Known(FrameId::QzssJgsItrf2014Aligned),
                Some(QZSS_JGS2014_SOURCE_EVIDENCE),
            )
        } else if source == SourceFrameIdentity::QzssBroadcastJgs
            && self.key.msgtype == NavMessageType::LNAV
            && qzss_jgs2020_resolved(self.key.epoch, self.orbit_reference, t)
        {
            (
                FrameRealization::Known(FrameId::QzssJgsItrf2020Aligned),
                Some(QZSS_JGS2020_SOURCE_EVIDENCE),
            )
        } else if source == SourceFrameIdentity::GalileoBroadcastGtrf
            && matches!(
                self.key.msgtype,
                NavMessageType::INAV | NavMessageType::FNAV
            )
            && galileo_gtrf23_resolved(self.key.epoch, self.orbit_reference, t)
        {
            (
                FrameRealization::Known(FrameId::GalileoGtrf23v01),
                Some(GALILEO_GTRF23_SOURCE_EVIDENCE),
            )
        } else if source == SourceFrameIdentity::BeidouBroadcast
            && matches!(
                (self.key.msgtype, self.key.sv.prn),
                (NavMessageType::D1, 10 | 20) | (NavMessageType::D2, 5)
            )
            && bdcs2019_sample_resolved(self.key.epoch, self.orbit_reference, t)
        {
            (
                FrameRealization::Known(FrameId::Bdcs2019v01),
                Some(BDCS2019_SOURCE_EVIDENCE),
            )
        } else {
            (FrameRealization::Unknown, None)
        };
        Ok(NavSpatialState {
            key: *self.key,
            orbit_reference: self.orbit_reference,
            clock_reference: self.clock_reference,
            record_epoch: self.key.epoch,
            state: BroadcastFixedState {
                epoch: t,
                native_frame,
                source,
                realization,
                source_evidence,
                position_km,
                velocity_km_s,
            },
        })
    }
}

/// Request GPS broadcast WGS-84 at the coordinate epoch or a concrete realization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameRequest {
    Wgs84,
    Realization(FrameId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameMethod {
    Native,
    Helmert,
    Composite,
    /// Numerically evaluated position with no validated epoch/domain error bound.
    UnboundedApproximate,
}

/// What supports the returned position's target-frame label.
/// This describes the frame operation, not total satellite-position accuracy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PositionStatus {
    NativeIdentity,
    NumericalTransform,
    MarkedApproximation,
}

/// A target state. Unknown realization remains explicit rather than inventing Gxxxx.
#[derive(Clone, Debug)]
pub struct FrameResult {
    pub epoch: Epoch,
    pub source: SourceFrameIdentity,
    pub target: SourceFrameIdentity,
    pub target_realization: FrameRealization,
    pub source_realization: FrameRealization,
    /// Always inspect `position_status()` before interpreting this target XYZ.
    pub position_km: [f64; 3],
    pub velocity_km_s: Option<[f64; 3]>,
    pub method: FrameMethod,
    /// Frozen parameter catalogue; no runtime download or kernel is used.
    pub catalog_version: &'static str,
    /// Ordered parameter/operation IDs that produced the returned position.
    pub edge_ids: Vec<&'static str>,
    /// Ordered per-edge direction, method, provenance, and applicability.
    pub edge_info: Vec<FrameEdgeInfo>,
    pub source_basis: SourceBasis,
    /// NAV broadcast realization evidence, separate from the conversion edge.
    /// None for caller-asserted points and unresolved NAV sources.
    pub source_evidence: Option<&'static str>,
    /// A published operation accuracy is not a strict position upper bound.
    pub position_accuracy_note: Option<&'static str>,
    pub velocity_note: Option<&'static str>,
}

#[derive(Clone, Copy, Debug)]
pub struct FrameEdgeInfo {
    pub id: &'static str,
    pub source: FrameId,
    pub target: FrameId,
    pub method: FrameMethod,
    pub parameter_reference_epoch: &'static str,
    pub valid_window: &'static str,
    /// Domain permitted by this implementation, independent of published fit data.
    pub domain: &'static str,
    /// Cases exercised locally; this does not extend the operation's valid window.
    pub sample_validation: &'static str,
    pub source_url: &'static str,
    pub position_metric: &'static str,
    pub velocity_capability: &'static str,
}

impl FrameResult {
    /// Inspect every chosen edge without re-running the transformation.
    pub fn info(&self) -> &[FrameEdgeInfo] {
        &self.edge_info
    }

    /// Human-readable cautions for a returned evidenced path.
    pub fn cautions(&self) -> Vec<String> {
        let mut notes = Vec::new();
        if self.source_basis == SourceBasis::CallerAsserted {
            notes.push("CAUTION: source identity and ECEF XYZ were asserted by the caller; if treated as a broadcast satellite state, NAV health, data validity, and propagation were not checked".into());
        }
        if let Some(note) = self.position_accuracy_note {
            notes.push(note.into());
        }
        for edge in self
            .edge_info
            .iter()
            .filter(|edge| edge.method == FrameMethod::UnboundedApproximate)
        {
            notes.push(format!(
                "CAUTION: approximate edge {} ({:?} -> {:?}); {}; window={}; metric={}; source={}",
                edge.id,
                edge.source,
                edge.target,
                edge.parameter_reference_epoch,
                edge.valid_window,
                edge.position_metric,
                edge.source_url
            ));
        }
        if let Some(note) = self.velocity_note {
            notes.push(format!("CAUTION: {note}"));
        }
        notes
    }

    /// Inspect this before treating `position_km` as a coordinate in `target`.
    /// A concrete target ID alone does not establish a physical frame relation.
    pub fn position_status(&self) -> PositionStatus {
        match self.method {
            FrameMethod::Native => PositionStatus::NativeIdentity,
            FrameMethod::Helmert | FrameMethod::Composite => PositionStatus::NumericalTransform,
            FrameMethod::UnboundedApproximate => PositionStatus::MarkedApproximation,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceBasis {
    NavMessageAndCatalogDate,
    /// The caller supplied the source identity; the library did not verify it.
    /// A dated catalogue may infer a concrete GPS broadcast realization from it.
    CallerAsserted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodPolicy {
    /// Allows marked, unbounded approximations when no validated path exists.
    BestAvailable,
    /// Requires a validated numerical path; excludes approximate edges.
    NumericalOnly,
}

#[derive(Clone, Copy, Debug)]
pub struct TransformOptions {
    pub method: MethodPolicy,
    /// Strict upper bound for the frame operation alone, in metres.
    pub max_frame_operation_error_m: Option<f64>,
    pub require_velocity: bool,
    /// Reject marked approximations.
    /// Informational accuracy/velocity notes on numerical paths are retained.
    pub warnings_as_errors: bool,
}

impl Default for TransformOptions {
    fn default() -> Self {
        Self {
            method: MethodPolicy::BestAvailable,
            max_frame_operation_error_m: None,
            require_velocity: false,
            warnings_as_errors: false,
        }
    }
}

/// Earth-fixed XYZ and coordinate epoch. A caller-provided source is an assertion.
/// Units are km and km/s; no NavKey, file path, or receiver metadata is needed.
#[derive(Clone, Copy, Debug)]
pub struct SpatialPoint {
    pub epoch: Epoch,
    pub source: SourceFrameIdentity,
    pub position_km: [f64; 3],
    pub velocity_km_s: Option<[f64; 3]>,
    source_basis: SourceBasis,
    realization: FrameRealization,
    source_evidence: Option<&'static str>,
    nav_snapshot: Option<NavPointSnapshot>,
}

#[derive(Clone, Copy, Debug)]
struct NavPointSnapshot {
    epoch: Epoch,
    source: SourceFrameIdentity,
    position_km: [f64; 3],
    velocity_km_s: Option<[f64; 3]>,
}

impl SpatialPoint {
    pub fn new(
        position_km: [f64; 3],
        epoch: Epoch,
        source: SourceFrameIdentity,
        velocity_km_s: Option<[f64; 3]>,
    ) -> Result<Self, FrameError> {
        Self::from_parts(position_km, Some(epoch), Some(source), velocity_km_s)
    }

    /// Also allows ingestion boundaries to report missing metadata explicitly.
    pub fn from_parts(
        position_km: [f64; 3],
        epoch: Option<Epoch>,
        source: Option<SourceFrameIdentity>,
        velocity_km_s: Option<[f64; 3]>,
    ) -> Result<Self, FrameError> {
        let epoch = epoch.ok_or(FrameError::MissingEpoch)?;
        let source = source.ok_or(FrameError::MissingSource)?;
        if !position_km
            .iter()
            .chain(velocity_km_s.iter().flatten())
            .all(|v| v.is_finite())
        {
            return Err(FrameError::NonFiniteState);
        }
        Ok(Self {
            epoch,
            source,
            position_km,
            velocity_km_s,
            source_basis: SourceBasis::CallerAsserted,
            realization: match source {
                SourceFrameIdentity::Realization(id) => FrameRealization::Known(id),
                _ => FrameRealization::Unknown,
            },
            source_evidence: None,
            nav_snapshot: None,
        })
    }

    /// Adapt an already propagated NAV state without selecting or propagating again.
    pub fn from_nav(state: &BroadcastFixedState) -> Result<Self, FrameError> {
        let mut point = Self::new(
            state.position_km,
            state.epoch,
            state.source,
            Some(state.velocity_km_s),
        )?;
        point.source_basis = SourceBasis::NavMessageAndCatalogDate;
        point.realization = state.realization;
        point.source_evidence = state.source_evidence;
        point.nav_snapshot = Some(NavPointSnapshot {
            epoch: point.epoch,
            source: point.source,
            position_km: point.position_km,
            velocity_km_s: point.velocity_km_s,
        });
        Ok(point)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameError {
    MissingSource,
    MissingEpoch,
    UnsupportedSource(SourceFrameIdentity),
    UnknownSourceRealization,
    OutsideCatalogWindow,
    NoPath,
    PositionBoundUnavailable,
    InvalidPositionBound,
    ApproximationExcluded,
    DomainNotApplicable,
    VelocityUnavailable,
    InconsistentFrame,
    NonFiniteState,
    WarningRejected(&'static str),
}
impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for FrameError {}

impl BroadcastFixedState {
    pub fn native_frame(&self) -> NativeFrame {
        self.native_frame
    }

    pub fn source(&self) -> SourceFrameIdentity {
        self.source
    }

    pub fn realization(&self) -> FrameRealization {
        self.realization
    }

    pub fn source_evidence(&self) -> Option<&'static str> {
        self.source_evidence
    }

    /// Use the same fixed frame converter as generic spatial points.
    /// This convenience call keeps the native state and its NavKey untouched.
    pub fn to_frame(&self, target: FrameRequest) -> Result<FrameResult, FrameError> {
        let expected = match self.source {
            SourceFrameIdentity::GpsBroadcastWgs84 => NativeFrame::GpsBroadcastWgs84,
            SourceFrameIdentity::NavicBroadcastWgs84 => NativeFrame::NavicBroadcastWgs84,
            SourceFrameIdentity::SbasBroadcast => NativeFrame::SbasBroadcast,
            SourceFrameIdentity::GlonassBroadcastPz90 => NativeFrame::GlonassBroadcastPz90,
            SourceFrameIdentity::QzssBroadcastJgs => NativeFrame::QzssBroadcastJgs,
            SourceFrameIdentity::GalileoBroadcastGtrf => NativeFrame::GalileoBroadcastGtrf,
            SourceFrameIdentity::BeidouBroadcast => NativeFrame::BeiDouBroadcastCgcs2000,
            SourceFrameIdentity::Realization(_) => self.native_frame,
        };
        if self.native_frame != expected {
            return Err(FrameError::InconsistentFrame);
        }
        FrameTransformer.to_frame(
            &SpatialPoint::from_nav(self)?,
            target,
            TransformOptions::default(),
        )
    }
}

const CATALOG_VERSION: &str = "rinex-nav-frame-catalog-v3";
const PZ9011_SOURCE_EVIDENCE: &str = "ICG17:2023:GNSS-TRFs:p9; ICG11:2016:PZ90.11-introduction:p12";
const G2296_SOURCE_EVIDENCE: &str = "https://www.navcen.uscg.gov/gps-constellation (NANU 2024014)";
const PZ9011_TO_ITRF2014: &str = "ICG:2018:PZ90.11-to-ITRF2014:static-2010-approx";
const ITRF2014_TO_PZ9011: &str = "ICG:2018:ITRF2014-to-PZ90.11:inverse-static-2010-approx";
const QZSS_JGS2014_SOURCE_EVIDENCE: &str = "QZSS:PNT-coordinate-system:2023-11-10:JGS-ITRF2014-period; QZSS:PNT-update-complete:2021-02-15";
const QZSS_JGS2014_TO_ITRF2014: &str = "QZSS:PNT:JGS-ITRF2014-alignment:zero-offset-approx";
const ITRF2014_TO_QZSS_JGS2014: &str =
    "QZSS:PNT:ITRF2014-to-JGS-alignment:inverse-zero-offset-approx";
const QZSS_JGS2020_SOURCE_EVIDENCE: &str = "https://qzss.go.jp/en/technical/dod/pnt/coordinate-system.html (broadcast JGS; ITRF2020 applied from 2023-11-09)";
const QZSS_JGS2020_TO_ITRF2020: &str = "QZSS:PNT:JGS-ITRF2020-alignment:zero-offset-approx";
const ITRF2020_TO_QZSS_JGS2020: &str =
    "QZSS:PNT:ITRF2020-to-JGS-alignment:inverse-zero-offset-approx";
const GALILEO_GTRF23_SOURCE_EVIDENCE: &str =
    "ESA:GGSP:GTRF23v01:applicable-2023-05-05; ICG18:2024:planned-GTRF-update";
const GTRF23_TO_ITRF2020: &str = "ESA:GGSP:GTRF23v01-ITRF2020:zero-offset-approx";
const ITRF2020_TO_GTRF23: &str = "ESA:GGSP:ITRF2020-to-GTRF23v01:inverse-zero-offset-approx";
const BDCS2019_SOURCE_EVIDENCE: &str =
    "CSNO:BDCS:2019v01; IGS:2022-workshop:2019v01-current:date-inferred";
const BDCS2019_TO_ITRF2014: &str = "rinex:BDCS2019v01-ITRF2014:zero-offset-approx";
const ITRF2014_TO_BDCS2019: &str = "rinex:ITRF2014-BDCS2019v01:inverse-zero-offset-approx";
const G2296_TO_ITRF2020: &str = "EPSG:10608";
const ITRF2020_TO_ITRF2014: &str = "ITRF2020:Table2:2015.0";
const NATIVE_EDGES: &[&str] = &[];
const G2296_INFO: FrameEdgeInfo = FrameEdgeInfo {
    id: G2296_TO_ITRF2020,
    source: FrameId::Wgs84G2296,
    target: FrameId::Itrf2020,
    method: FrameMethod::Helmert,
    parameter_reference_epoch: "2024.0",
    valid_window: "2024-03-04 through 2024-12-31 UTC (catalogue restriction)",
    domain: "Earth-fixed XYZ; satellite operation bound not established",
    sample_validation: "2024-05 real GPS and independent numerical reference",
    source_url: "https://epsg.io/10608",
    position_metric: "EPSG operation accuracy 0.01 m at 2024.0; not a strict satellite upper bound",
    velocity_capability: "unvalidated",
};
const G2296_INVERSE_INFO: FrameEdgeInfo = FrameEdgeInfo {
    id: "EPSG:10608:inverse",
    source: FrameId::Itrf2020,
    target: FrameId::Wgs84G2296,
    ..G2296_INFO
};
const ITRF_INFO: FrameEdgeInfo = FrameEdgeInfo {
    id: ITRF2020_TO_ITRF2014,
    source: FrameId::Itrf2020,
    target: FrameId::Itrf2014,
    method: FrameMethod::Helmert,
    parameter_reference_epoch: "2015.0",
    valid_window: "2015-01-01 through 2026-12-31 UTC (catalogue restriction)",
    domain: "Earth-fixed XYZ; satellite operation bound not established",
    sample_validation: "2024-05 numerical and reverse-path reference",
    source_url: "https://itrf.ign.fr/en/solutions/itrf2020",
    position_metric: "published parameter uncertainties; no strict satellite upper bound",
    velocity_capability: "rates published; target velocity unvalidated",
};
const ITRF_INVERSE_INFO: FrameEdgeInfo = FrameEdgeInfo {
    id: "ITRF2020:Table2:2015.0:inverse",
    source: FrameId::Itrf2014,
    target: FrameId::Itrf2020,
    ..ITRF_INFO
};
const PZ9011_INFO: FrameEdgeInfo = FrameEdgeInfo {
    id: PZ9011_TO_ITRF2014,
    source: FrameId::Pz90_11,
    target: FrameId::Itrf2014,
    method: FrameMethod::UnboundedApproximate,
    parameter_reference_epoch: "2010.0; parameters frozen at later coordinate epochs",
    valid_window:
        "2014-01-15 through 2024-12-31 UTC (library approximation window, not publication validity)",
    domain: "Earth-fixed XYZ; satellite positions are marked approximate",
    sample_validation: "2024-05 GLONASS R02 real fixture and independent arithmetic",
    source_url: "https://www.unoosa.org/documents/pdf/icg/2018/icg13/wgd/wgd_24.pdf",
    position_metric: "2010.0 ground-station fit RMS 0.012 m; no satellite or epoch error bound",
    velocity_capability: "not established",
};
const PZ9011_INVERSE_INFO: FrameEdgeInfo = FrameEdgeInfo {
    id: ITRF2014_TO_PZ9011,
    source: FrameId::Itrf2014,
    target: FrameId::Pz90_11,
    sample_validation: "2020-06 independent inverse arithmetic of R01 reference coordinates",
    ..PZ9011_INFO
};
const QZSS_JGS2014_INFO: FrameEdgeInfo = FrameEdgeInfo {
    id: QZSS_JGS2014_TO_ITRF2014,
    source: FrameId::QzssJgsItrf2014Aligned,
    target: FrameId::Itrf2014,
    method: FrameMethod::UnboundedApproximate,
    parameter_reference_epoch: "none; zero-offset approximation of documented ITRF2014 alignment",
    valid_window: "2021-02-16 through 2023-11-08 UTC (conservative library window)",
    domain: "Earth-fixed XYZ; JGS broadcast alignment approximation",
    sample_validation: "2023-03 J02 real fixture",
    source_url: "https://qzss.go.jp/en/technical/dod/pnt/coordinate-system.html",
    position_metric:
        "PNT monitor-station offset within 0.02 m (95%); no satellite or strict upper bound",
    velocity_capability: "not established",
};
const QZSS_JGS2014_INVERSE_INFO: FrameEdgeInfo = FrameEdgeInfo {
    id: ITRF2014_TO_QZSS_JGS2014,
    source: FrameId::Itrf2014,
    target: FrameId::QzssJgsItrf2014Aligned,
    parameter_reference_epoch:
        "none; inverse of the marked zero-offset JGS/ITRF2014 alignment assumption",
    sample_validation: "2023-03 J02 reference XYZ and reverse zero-offset arithmetic",
    ..QZSS_JGS2014_INFO
};
const QZSS_JGS2020_INFO: FrameEdgeInfo = FrameEdgeInfo {
    id: QZSS_JGS2020_TO_ITRF2020,
    source: FrameId::QzssJgsItrf2020Aligned,
    target: FrameId::Itrf2020,
    method: FrameMethod::UnboundedApproximate,
    parameter_reference_epoch: "none; zero-offset approximation of documented ITRF2020 alignment",
    valid_window: "2023-11-11 through 2024-12-31 UTC (conservative library window)",
    domain: "Earth-fixed XYZ; JGS broadcast alignment approximation",
    sample_validation: "2024-05 caller-asserted JGS point",
    source_url: "https://qzss.go.jp/en/technical/dod/pnt/coordinate-system.html",
    position_metric:
        "PNT monitor-station offset within 0.02 m (95%); no satellite or strict upper bound",
    velocity_capability: "not established",
};
const QZSS_JGS2020_INVERSE_INFO: FrameEdgeInfo = FrameEdgeInfo {
    id: ITRF2020_TO_QZSS_JGS2020,
    source: FrameId::Itrf2020,
    target: FrameId::QzssJgsItrf2020Aligned,
    parameter_reference_epoch:
        "none; inverse of the marked zero-offset JGS/ITRF2020 alignment assumption",
    sample_validation: "2024-05 caller-asserted ITRF2020 point",
    ..QZSS_JGS2020_INFO
};
const GTRF23_INFO: FrameEdgeInfo = FrameEdgeInfo {
    id: GTRF23_TO_ITRF2020,
    source: FrameId::GalileoGtrf23v01,
    target: FrameId::Itrf2020,
    method: FrameMethod::UnboundedApproximate,
    parameter_reference_epoch: "none; zero-offset approximation of GTRF23v01 alignment",
    valid_window: "2024-05-01 through 2024-05-31 UTC (narrow library inference window)",
    domain: "Earth-fixed XYZ; GTRF broadcast alignment approximation",
    sample_validation: "2024-05 E03 real fixture",
    source_url: "https://navigation-office.esa.int/attachments/32835744/1/GTRF_IGS_Stop_6.pdf",
    position_metric: "GTRF station alignment <0.03 m (2 sigma); no satellite or strict upper bound",
    velocity_capability: "not established",
};
const GTRF23_INVERSE_INFO: FrameEdgeInfo = FrameEdgeInfo {
    id: ITRF2020_TO_GTRF23,
    source: FrameId::Itrf2020,
    target: FrameId::GalileoGtrf23v01,
    parameter_reference_epoch:
        "none; inverse of the marked zero-offset GTRF23v01/ITRF2020 alignment assumption",
    sample_validation: "2024-05 E03 reference XYZ and reverse zero-offset arithmetic",
    ..GTRF23_INFO
};
const BDCS2019_INFO: FrameEdgeInfo = FrameEdgeInfo {
    id: BDCS2019_TO_ITRF2014,
    source: FrameId::Bdcs2019v01,
    target: FrameId::Itrf2014,
    method: FrameMethod::UnboundedApproximate,
    parameter_reference_epoch: "2019v01 alignment; zero-offset approximation, no dated rates",
    valid_window: "2022-06-01 through 2022-06-30 UTC (narrow library inference window)",
    domain: "Earth-fixed XYZ; dated BDCS broadcast inference",
    sample_validation: "2022-06 C05/C10/C20 real fixtures",
    source_url: "https://files.igs.org/pub/resource/pubs/workshop/2022/TourdelIGS4_05_Hu.pdf",
    position_metric: "2019 station alignment, not a satellite or 2022 strict error bound",
    velocity_capability: "not established",
};
const BDCS2019_INVERSE_INFO: FrameEdgeInfo = FrameEdgeInfo {
    id: ITRF2014_TO_BDCS2019,
    source: FrameId::Itrf2014,
    target: FrameId::Bdcs2019v01,
    parameter_reference_epoch:
        "2019v01; inverse of the marked zero-offset BDCS/ITRF2014 alignment assumption",
    sample_validation: "2022-06 C05/C10/C20 reference XYZ and reverse zero-offset arithmetic",
    ..BDCS2019_INFO
};
const NATIVE_INFO: &[FrameEdgeInfo] = &[];
/// Fixed, offline, narrowly dated terrestrial-frame parameter catalogue.
/// It uses no ANISE frame or kernel: these GNSS realizations are not ANISE frames.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameTransformer;

impl FrameTransformer {
    pub const fn catalog_version(&self) -> &'static str {
        CATALOG_VERSION
    }

    /// Convert a generic Earth-fixed point when an evidenced route exists.
    /// The coordinate epoch is preserved; unavailable routes return `FrameError`.
    /// G2296 is resolved only for 2024-03-04 through 2024-12-31, after the
    /// operational GPS update completed. The ITRF edge has its own window.
    /// Cross-frame velocity is withheld until
    /// the full chain has an independently checked velocity reference.
    pub fn to_frame(
        &self,
        point: &SpatialPoint,
        request: FrameRequest,
        options: TransformOptions,
    ) -> Result<FrameResult, FrameError> {
        if point.source_basis == SourceBasis::NavMessageAndCatalogDate
            && !point.nav_snapshot.is_some_and(|snapshot| {
                snapshot.epoch == point.epoch
                    && snapshot.source == point.source
                    && snapshot.position_km == point.position_km
                    && snapshot.velocity_km_s == point.velocity_km_s
            })
        {
            return Err(FrameError::InconsistentFrame);
        }
        let result = self.to_frame_inner(point, request, options)?;
        if options.warnings_as_errors
            && result.position_status() == PositionStatus::MarkedApproximation
        {
            return Err(FrameError::WarningRejected(
                result
                    .position_accuracy_note
                    .or(result.velocity_note)
                    .unwrap_or("CAUTION: unbounded frame approximation"),
            ));
        }
        Ok(result)
    }

    fn to_frame_inner(
        &self,
        point: &SpatialPoint,
        request: FrameRequest,
        options: TransformOptions,
    ) -> Result<FrameResult, FrameError> {
        if !point
            .position_km
            .iter()
            .chain(point.velocity_km_s.iter().flatten())
            .all(|v| v.is_finite())
        {
            return Err(FrameError::NonFiniteState);
        }
        if let Some(limit) = options.max_frame_operation_error_m {
            if !limit.is_finite() || limit < 0.0 {
                return Err(FrameError::InvalidPositionBound);
            }
        }

        // Bare WGS84 requests against a GPS broadcast state are exact identity
        // requests, even when the specific historical realization is unknown.
        if request == FrameRequest::Wgs84 && point.source == SourceFrameIdentity::GpsBroadcastWgs84
        {
            let realization = if resolved_g2296(point) {
                FrameRealization::Known(FrameId::Wgs84G2296)
            } else {
                FrameRealization::Unknown
            };
            return native_result(
                point,
                SourceFrameIdentity::GpsBroadcastWgs84,
                realization,
                options,
            );
        }

        let source_id = match point.source {
            SourceFrameIdentity::Realization(id) => id,
            SourceFrameIdentity::GlonassBroadcastPz90
                if point.realization == FrameRealization::Known(FrameId::Pz90_11) =>
            {
                FrameId::Pz90_11
            },
            SourceFrameIdentity::GlonassBroadcastPz90 => {
                return Err(FrameError::UnknownSourceRealization)
            },
            SourceFrameIdentity::QzssBroadcastJgs
                if point.realization
                    == FrameRealization::Known(FrameId::QzssJgsItrf2014Aligned) =>
            {
                FrameId::QzssJgsItrf2014Aligned
            },
            SourceFrameIdentity::QzssBroadcastJgs
                if point.realization
                    == FrameRealization::Known(FrameId::QzssJgsItrf2020Aligned) =>
            {
                FrameId::QzssJgsItrf2020Aligned
            },
            SourceFrameIdentity::QzssBroadcastJgs => {
                return Err(FrameError::UnknownSourceRealization)
            },
            SourceFrameIdentity::GalileoBroadcastGtrf
                if point.realization == FrameRealization::Known(FrameId::GalileoGtrf23v01) =>
            {
                FrameId::GalileoGtrf23v01
            },
            SourceFrameIdentity::GalileoBroadcastGtrf => {
                return Err(FrameError::UnknownSourceRealization)
            },
            SourceFrameIdentity::BeidouBroadcast
                if point.realization == FrameRealization::Known(FrameId::Bdcs2019v01) =>
            {
                FrameId::Bdcs2019v01
            },
            SourceFrameIdentity::BeidouBroadcast => {
                return Err(FrameError::UnknownSourceRealization)
            },
            SourceFrameIdentity::GpsBroadcastWgs84 if resolved_g2296(point) => FrameId::Wgs84G2296,
            SourceFrameIdentity::GpsBroadcastWgs84 => {
                return Err(FrameError::UnknownSourceRealization)
            },
            other => return Err(FrameError::UnsupportedSource(other)),
        };
        let target_id = match request {
            FrameRequest::Wgs84 if in_g2296_window(point.epoch) => FrameId::Wgs84G2296,
            FrameRequest::Wgs84 => return Err(FrameError::OutsideCatalogWindow),
            FrameRequest::Realization(id) => id,
        };
        let target_identity = match request {
            FrameRequest::Wgs84 => SourceFrameIdentity::GpsBroadcastWgs84,
            FrameRequest::Realization(id) => SourceFrameIdentity::Realization(id),
        };
        if source_id == target_id {
            return native_result(
                point,
                target_identity,
                FrameRealization::Known(target_id),
                options,
            );
        }
        let path = find_catalog_path(source_id, target_id, point.epoch)?;
        let approximate = path
            .iter()
            .any(|edge| edge.method == FrameMethod::UnboundedApproximate);
        qualify_path(
            PathEvidence {
                method: if approximate {
                    FrameMethod::UnboundedApproximate
                } else {
                    FrameMethod::Helmert
                },
                domain: PathDomain::AnyEarthFixed,
            },
            point,
            options,
        )?;
        if options.require_velocity {
            return Err(FrameError::VelocityUnavailable);
        }
        let mut position_km = point.position_km;
        for edge in &path {
            position_km = apply_catalog_edge(edge.id, position_km, point.epoch);
            if !position_km.iter().all(|v| v.is_finite()) {
                return Err(FrameError::NonFiniteState);
            }
        }
        let note = if approximate {
            "CAUTION: this frame route includes an unbounded approximation; inspect every ordered edge and its caution. No strict frame-operation error bound for a satellite position is established."
        } else if path
            .iter()
            .any(|edge| edge.source == FrameId::Wgs84G2296 || edge.target == FrameId::Wgs84G2296)
        {
            "EPSG:10608 operation accuracy and ITRF parameter uncertainties are not strict frame-operation upper bounds for a satellite position."
        } else {
            "ITRF2020 Table 2 parameter uncertainties are not a strict frame-operation upper bound for a satellite position."
        };
        Ok(FrameResult {
            epoch: point.epoch,
            source: point.source,
            target: target_identity,
            source_realization: FrameRealization::Known(source_id),
            target_realization: FrameRealization::Known(target_id),
            position_km,
            velocity_km_s: None,
            method: if approximate {
                FrameMethod::UnboundedApproximate
            } else if path.len() == 1 {
                FrameMethod::Helmert
            } else {
                FrameMethod::Composite
            },
            catalog_version: CATALOG_VERSION,
            edge_ids: path.iter().map(|edge| edge.id).collect(),
            edge_info: path,
            source_basis: point.source_basis,
            source_evidence: point.source_evidence,
            position_accuracy_note: Some(note),
            velocity_note: Some("cross-frame velocity is not validated"),
        })
    }
}

/// Installed directed operations. Reverse approximate edges require separate evidence.
const CATALOG_EDGES: &[FrameEdgeInfo] = &[
    G2296_INFO,
    G2296_INVERSE_INFO,
    ITRF_INFO,
    ITRF_INVERSE_INFO,
    PZ9011_INFO,
    PZ9011_INVERSE_INFO,
    QZSS_JGS2014_INFO,
    QZSS_JGS2014_INVERSE_INFO,
    QZSS_JGS2020_INFO,
    QZSS_JGS2020_INVERSE_INFO,
    GTRF23_INFO,
    GTRF23_INVERSE_INFO,
    BDCS2019_INFO,
    BDCS2019_INVERSE_INFO,
];

fn edge_window(edge: FrameEdgeInfo, epoch: Epoch) -> bool {
    match edge.id {
        G2296_TO_ITRF2020 | "EPSG:10608:inverse" => in_g2296_window(epoch),
        ITRF2020_TO_ITRF2014 | "ITRF2020:Table2:2015.0:inverse" => in_itrf_window(epoch),
        PZ9011_TO_ITRF2014 | ITRF2014_TO_PZ9011 => in_pz9011_window(epoch) && in_itrf_window(epoch),
        QZSS_JGS2014_TO_ITRF2014 | ITRF2014_TO_QZSS_JGS2014 => in_qzss_jgs2014_window(epoch),
        QZSS_JGS2020_TO_ITRF2020 | ITRF2020_TO_QZSS_JGS2020 => in_qzss_jgs2020_window(epoch),
        GTRF23_TO_ITRF2020 | ITRF2020_TO_GTRF23 => in_gtrf23_sample_window(epoch),
        BDCS2019_TO_ITRF2014 | ITRF2014_TO_BDCS2019 => in_bdcs2019_sample_window(epoch),
        _ => false,
    }
}

fn catalog_paths(
    source: FrameId,
    target: FrameId,
    epoch: Epoch,
    dated: bool,
) -> Vec<Vec<FrameEdgeInfo>> {
    let mut pending = vec![Vec::<FrameEdgeInfo>::new()];
    let mut found = Vec::new();
    while let Some(path) = pending.pop() {
        let current = path.last().map_or(source, |edge| edge.target);
        for edge in CATALOG_EDGES.iter().copied() {
            if edge.source != current || (dated && !edge_window(edge, epoch)) {
                continue;
            }
            if edge.target == source || path.iter().any(|prior| prior.source == edge.target) {
                continue;
            }
            let mut next = path.clone();
            next.push(edge);
            if edge.target == target {
                found.push(next);
            } else if next.len() < 8 {
                pending.push(next);
            }
        }
    }
    found.sort_by_key(|path| {
        (
            path.iter()
                .any(|edge| edge.method == FrameMethod::UnboundedApproximate),
            path.len(),
            path.iter()
                .map(|edge| edge.id)
                .collect::<Vec<_>>()
                .join("|"),
        )
    });
    found
}

fn find_catalog_path(
    source: FrameId,
    target: FrameId,
    epoch: Epoch,
) -> Result<Vec<FrameEdgeInfo>, FrameError> {
    if let Some(path) = catalog_paths(source, target, epoch, true)
        .into_iter()
        .next()
    {
        return Ok(path);
    }
    if !catalog_paths(source, target, epoch, false).is_empty() {
        Err(FrameError::OutsideCatalogWindow)
    } else {
        Err(FrameError::NoPath)
    }
}

fn apply_catalog_edge(id: &str, xyz: [f64; 3], epoch: Epoch) -> [f64; 3] {
    match id {
        G2296_TO_ITRF2020
        | "EPSG:10608:inverse"
        | QZSS_JGS2014_TO_ITRF2014
        | ITRF2014_TO_QZSS_JGS2014
        | QZSS_JGS2020_TO_ITRF2020
        | ITRF2020_TO_QZSS_JGS2020
        | GTRF23_TO_ITRF2020
        | ITRF2020_TO_GTRF23
        | BDCS2019_TO_ITRF2014
        | ITRF2014_TO_BDCS2019 => xyz,
        ITRF2020_TO_ITRF2014 => itrf2020_to_2014(xyz, epoch),
        "ITRF2020:Table2:2015.0:inverse" => itrf2014_to_2020(xyz, epoch),
        PZ9011_TO_ITRF2014 => pz9011_to_itrf2014_approx(xyz),
        ITRF2014_TO_PZ9011 => itrf2014_to_pz9011_approx(xyz),
        _ => unreachable!("only installed edges are evaluated"),
    }
}

#[allow(dead_code)] // A satellite-only edge is exercised with a synthetic candidate.
#[derive(Clone, Copy)]
enum PathDomain {
    AnyEarthFixed,
    NavBroadcastSatellite,
}

#[derive(Clone, Copy)]
struct PathEvidence {
    method: FrameMethod,
    domain: PathDomain,
}

/// Shared option gate for the installed numerical and unbounded paths.
fn qualify_path(
    evidence: PathEvidence,
    point: &SpatialPoint,
    options: TransformOptions,
) -> Result<(), FrameError> {
    if options.method == MethodPolicy::NumericalOnly
        && evidence.method == FrameMethod::UnboundedApproximate
    {
        return Err(FrameError::ApproximationExcluded);
    }
    match evidence.domain {
        PathDomain::AnyEarthFixed => {},
        PathDomain::NavBroadcastSatellite => {
            if point.source_basis != SourceBasis::NavMessageAndCatalogDate {
                return Err(FrameError::DomainNotApplicable);
            }
        },
    }
    if options.max_frame_operation_error_m.is_some() {
        return Err(FrameError::PositionBoundUnavailable);
    }
    Ok(())
}

fn native_result(
    point: &SpatialPoint,
    target: SourceFrameIdentity,
    realization: FrameRealization,
    options: TransformOptions,
) -> Result<FrameResult, FrameError> {
    if options.require_velocity && point.velocity_km_s.is_none() {
        return Err(FrameError::VelocityUnavailable);
    }
    Ok(FrameResult {
        epoch: point.epoch,
        source: point.source,
        target,
        source_realization: realization,
        target_realization: realization,
        position_km: point.position_km,
        velocity_km_s: point.velocity_km_s,
        method: FrameMethod::Native,
        catalog_version: CATALOG_VERSION,
        edge_ids: NATIVE_EDGES.to_vec(),
        edge_info: NATIVE_INFO.to_vec(),
        source_basis: point.source_basis,
        source_evidence: point.source_evidence,
        position_accuracy_note: None,
        velocity_note: None,
    })
}

fn in_g2296_window(epoch: Epoch) -> bool {
    epoch >= Epoch::from_gregorian_utc(2024, 3, 4, 0, 0, 0, 0)
        && epoch < Epoch::from_gregorian_utc(2025, 1, 1, 0, 0, 0, 0)
}

fn resolved_g2296(point: &SpatialPoint) -> bool {
    in_g2296_window(point.epoch)
        && (point.source_basis == SourceBasis::CallerAsserted
            || point.realization == FrameRealization::Known(FrameId::Wgs84G2296))
}

fn in_pz9011_window(epoch: Epoch) -> bool {
    epoch >= Epoch::from_gregorian_utc(2014, 1, 15, 0, 0, 0, 0)
        && epoch < Epoch::from_gregorian_utc(2025, 1, 1, 0, 0, 0, 0)
}

fn in_qzss_jgs2014_window(epoch: Epoch) -> bool {
    // The coordinate history lists Japanese change dates, not exact instants.
    // Start after the 2021 change and stop before the 2023 change work.
    epoch >= Epoch::from_gregorian_utc(2021, 2, 16, 0, 0, 0, 0)
        && epoch < Epoch::from_gregorian_utc(2023, 11, 9, 0, 0, 0, 0)
}

fn in_qzss_jgs2020_window(epoch: Epoch) -> bool {
    // QZSS lists a change date, not a UTC instant. Start after that date's
    // work; the library restricts this inferred relation to the dated 2024 interval.
    epoch >= Epoch::from_gregorian_utc(2023, 11, 11, 0, 0, 0, 0)
        && epoch < Epoch::from_gregorian_utc(2025, 1, 1, 0, 0, 0, 0)
}

fn qzss_jgs2020_resolved(record: Epoch, orbit: Option<Epoch>, evaluation: Epoch) -> bool {
    in_qzss_jgs2020_window(record)
        && orbit.is_some_and(in_qzss_jgs2020_window)
        && in_qzss_jgs2020_window(evaluation)
}

fn in_gtrf23_sample_window(epoch: Epoch) -> bool {
    // ESA dates the start to 2023-05-05. The 2024 ICG report says another
    // update was still planned. Limit this inferred interval to sample month.
    epoch >= Epoch::from_gregorian_utc(2024, 5, 1, 0, 0, 0, 0)
        && epoch < Epoch::from_gregorian_utc(2024, 6, 1, 0, 0, 0, 0)
}

fn in_bdcs2019_sample_window(epoch: Epoch) -> bool {
    // IGS workshop material still calls 2019v01 the current solution in 2022.
    // Limit the inferred operational assignment to the real C10/C20/C05 sample month.
    epoch >= Epoch::from_gregorian_utc(2022, 6, 1, 0, 0, 0, 0)
        && epoch < Epoch::from_gregorian_utc(2022, 7, 1, 0, 0, 0, 0)
}

fn bdcs2019_sample_resolved(
    record_epoch: Epoch,
    orbit_reference: Option<Epoch>,
    evaluation_epoch: Epoch,
) -> bool {
    in_bdcs2019_sample_window(record_epoch)
        && orbit_reference.is_some_and(in_bdcs2019_sample_window)
        && in_bdcs2019_sample_window(evaluation_epoch)
}

fn galileo_gtrf23_resolved(
    record_epoch: Epoch,
    orbit_reference: Option<Epoch>,
    evaluation_epoch: Epoch,
) -> bool {
    in_gtrf23_sample_window(record_epoch)
        && orbit_reference.is_some_and(in_gtrf23_sample_window)
        && in_gtrf23_sample_window(evaluation_epoch)
}

fn qzss_jgs2014_resolved(
    record_epoch: Epoch,
    orbit_reference: Option<Epoch>,
    evaluation_epoch: Epoch,
) -> bool {
    in_qzss_jgs2014_window(record_epoch)
        && orbit_reference.is_some_and(in_qzss_jgs2014_window)
        && in_qzss_jgs2014_window(evaluation_epoch)
}

fn pz9011_to_itrf2014_approx(position_km: [f64; 3]) -> [f64; 3] {
    // ICG-13 2018 wgd_24.pdf pp. 5, 8: coordinate-frame rotation convention.
    // Translation is metres, rotation is milliarcseconds, scale is 10^-6.
    // The published scale is zero to the displayed precision. Apply these
    // 2010.0 parameters unchanged at the point epoch only as an approximation.
    let [x, y, z] = position_km.map(|v| v * 1000.0);
    let ([rx, ry, rz], translation_m) = pz9011_parameters();
    [
        (x + rz * y - ry * z + translation_m[0]) / 1000.0,
        (y - rz * x + rx * z + translation_m[1]) / 1000.0,
        (z + ry * x - rx * y + translation_m[2]) / 1000.0,
    ]
}

fn pz9011_parameters() -> ([f64; 3], [f64; 3]) {
    let mas_to_rad = std::f64::consts::PI / (180.0 * 3600.0 * 1000.0);
    (
        [0.035 * mas_to_rad, -0.087 * mas_to_rad, 0.036 * mas_to_rad],
        [-0.0053, -0.0040, -0.0032],
    )
}

fn itrf2014_to_pz9011_approx(position_km: [f64; 3]) -> [f64; 3] {
    // Exactly invert the installed linearized forward operation, rather than
    // merely negating its parameters. If A = I - cross(r), then
    // A^-1 = (I + cross(r) + r*r^T) / (1 + r*r^T).
    let (r, translation_m) = pz9011_parameters();
    let u: [f64; 3] = std::array::from_fn(|i| position_km[i] * 1000.0 - translation_m[i]);
    let cross = [
        r[1] * u[2] - r[2] * u[1],
        r[2] * u[0] - r[0] * u[2],
        r[0] * u[1] - r[1] * u[0],
    ];
    let dot = r.iter().zip(u).map(|(a, b)| a * b).sum::<f64>();
    let denominator = 1.0 + r.iter().map(|v| v * v).sum::<f64>();
    std::array::from_fn(|i| (u[i] + cross[i] + r[i] * dot) / denominator / 1000.0)
}

fn in_itrf_window(epoch: Epoch) -> bool {
    epoch >= Epoch::from_gregorian_utc(2015, 1, 1, 0, 0, 0, 0)
        && epoch < Epoch::from_gregorian_utc(2027, 1, 1, 0, 0, 0, 0)
}

fn itrf2020_parameters(epoch: Epoch) -> ([f64; 3], f64) {
    let reference = Epoch::from_gregorian_utc(2015, 1, 1, 0, 0, 0, 0);
    let years = (epoch - reference).to_seconds() / 31_557_600.0;
    // IERS ITRF2020 Table 2: ITRF2014 minus ITRF2020, position-vector
    // convention. Translation and rate in mm and mm/Julian year, scale in ppb.
    // The published rotations and rotation rates are zero.
    ([-1.4, -0.9 - 0.1 * years, 1.4 + 0.2 * years], 1.0 - 0.42e-9)
}

fn itrf2020_to_2014(position_km: [f64; 3], epoch: Epoch) -> [f64; 3] {
    let (translation_mm, scale) = itrf2020_parameters(epoch);
    std::array::from_fn(|i| scale * position_km[i] + translation_mm[i] * 1e-6)
}

fn itrf2014_to_2020(position_km: [f64; 3], epoch: Epoch) -> [f64; 3] {
    let (translation_mm, scale) = itrf2020_parameters(epoch);
    std::array::from_fn(|i| (position_km[i] - translation_mm[i] * 1e-6) / scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qzss_jgs2020_source_requires_all_three_instants_in_conservative_window() {
        let inside = Epoch::from_gregorian_utc(2024, 5, 10, 0, 0, 0, 0);
        let first = Epoch::from_gregorian_utc(2023, 11, 11, 0, 0, 0, 0);
        let last = Epoch::from_gregorian_utc(2024, 12, 31, 23, 59, 59, 0);
        let before = Epoch::from_gregorian_utc(2023, 11, 10, 23, 59, 59, 0);
        let after = Epoch::from_gregorian_utc(2025, 1, 1, 0, 0, 0, 0);
        assert!(qzss_jgs2020_resolved(first, Some(inside), last));
        assert!(!qzss_jgs2020_resolved(before, Some(inside), inside));
        assert!(!qzss_jgs2020_resolved(inside, Some(after), inside));
        assert!(!qzss_jgs2020_resolved(inside, Some(inside), after));
        assert!(!qzss_jgs2020_resolved(inside, None, inside));
    }

    #[test]
    fn bdcs2019_source_requires_all_three_instants_inside_sample_window() {
        let inside = Epoch::from_gregorian_utc(2022, 6, 8, 7, 0, 0, 0);
        let before = Epoch::from_gregorian_utc(2022, 5, 31, 23, 59, 59, 0);
        let first = Epoch::from_gregorian_utc(2022, 6, 1, 0, 0, 0, 0);
        let last = Epoch::from_gregorian_utc(2022, 6, 30, 23, 59, 59, 0);
        let after = Epoch::from_gregorian_utc(2022, 7, 1, 0, 0, 0, 0);
        assert!(bdcs2019_sample_resolved(first, Some(inside), last));
        assert!(!bdcs2019_sample_resolved(before, Some(inside), inside));
        assert!(!bdcs2019_sample_resolved(inside, Some(after), inside));
        assert!(!bdcs2019_sample_resolved(inside, Some(inside), after));
        assert!(!bdcs2019_sample_resolved(inside, None, inside));
    }

    #[test]
    fn galileo_gtrf23_source_requires_all_three_instants_inside_sample_window() {
        let inside = Epoch::from_gregorian_utc(2024, 5, 7, 0, 29, 42, 0);
        let before = Epoch::from_gregorian_utc(2024, 4, 30, 23, 59, 59, 0);
        let first = Epoch::from_gregorian_utc(2024, 5, 1, 0, 0, 0, 0);
        let last = Epoch::from_gregorian_utc(2024, 5, 31, 23, 59, 59, 0);
        let after = Epoch::from_gregorian_utc(2024, 6, 1, 0, 0, 0, 0);
        assert!(galileo_gtrf23_resolved(first, Some(inside), last));
        assert!(!galileo_gtrf23_resolved(before, Some(inside), inside));
        assert!(!galileo_gtrf23_resolved(inside, Some(after), inside));
        assert!(!galileo_gtrf23_resolved(inside, Some(inside), after));
        assert!(!galileo_gtrf23_resolved(inside, None, inside));
    }

    #[test]
    fn qzss_jgs2014_source_requires_all_three_instants_inside_the_conservative_window() {
        let inside = Epoch::from_gregorian_utc(2023, 3, 12, 0, 0, 0, 0);
        let before = Epoch::from_gregorian_utc(2021, 2, 15, 23, 59, 59, 0);
        let first = Epoch::from_gregorian_utc(2021, 2, 16, 0, 0, 0, 0);
        let last = Epoch::from_gregorian_utc(2023, 11, 8, 23, 59, 59, 0);
        let after = Epoch::from_gregorian_utc(2023, 11, 9, 0, 0, 0, 0);
        assert!(qzss_jgs2014_resolved(first, Some(inside), last));
        assert!(!qzss_jgs2014_resolved(before, Some(inside), inside));
        assert!(!qzss_jgs2014_resolved(inside, Some(after), inside));
        assert!(!qzss_jgs2014_resolved(inside, Some(inside), after));
        assert!(!qzss_jgs2014_resolved(inside, None, inside));
    }

    #[test]
    fn unbounded_approximation_is_filtered_by_method_domain_and_bound() {
        let direct = SpatialPoint::new(
            [10000.0; 3],
            Epoch::from_gregorian_utc(2024, 5, 7, 0, 0, 0, 0),
            SourceFrameIdentity::Realization(FrameId::Itrf2020),
            None,
        )
        .unwrap();
        let evidence = PathEvidence {
            method: FrameMethod::UnboundedApproximate,
            domain: PathDomain::NavBroadcastSatellite,
        };
        assert_eq!(
            qualify_path(evidence, &direct, TransformOptions::default()),
            Err(FrameError::DomainNotApplicable)
        );
        let mut nav = direct;
        nav.source_basis = SourceBasis::NavMessageAndCatalogDate;
        assert_eq!(
            qualify_path(
                evidence,
                &nav,
                TransformOptions {
                    method: MethodPolicy::NumericalOnly,
                    ..Default::default()
                }
            ),
            Err(FrameError::ApproximationExcluded)
        );
        assert_eq!(
            qualify_path(
                evidence,
                &nav,
                TransformOptions {
                    max_frame_operation_error_m: Some(2.0),
                    ..Default::default()
                }
            ),
            Err(FrameError::PositionBoundUnavailable)
        );
    }
}
