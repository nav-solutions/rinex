//! Selection and native propagation for supported broadcast ephemerides.
use crate::{
    navigation::{Ephemeris, NavKey, NavMessageType},
    prelude::{Constellation, Duration, Epoch, Rinex, SV},
};

/// GPS broadcast coordinates are in native WGS-84 axes. The RINEX record
/// does not identify a particular WGS-84 realization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeFrame {
    GpsBroadcastWgs84,
    /// GLONASS broadcast PZ-90 axes; the record does not identify a realization.
    GlonassBroadcastPz90,
    SbasBroadcast,
    QzssBroadcastJgs,
    NavicBroadcastWgs84,
    GalileoBroadcastGtrf,
    BeiDouBroadcastCgcs2000,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnknownHealthPolicy {
    Reject,
    Allow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavRejection {
    UnsupportedMessage,
    MissingOrbitReference,
    MissingOrbitField,
    UnknownHealth,
    Unhealthy,
    InvalidData,
    OutOfValidity,
    LowerRank,
    Unpropagatable,
}

#[derive(Clone, Copy, Debug)]
pub struct NativeGpsState {
    pub epoch: Epoch,
    pub frame: NativeFrame,
    pub position_km: [f64; 3],
    pub velocity_km_s: [f64; 3],
    /// Satellite clock correction without signal group delay, in seconds.
    pub clock_correction_s: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateError {
    Rejected(NavRejection),
    UnsupportedMessage,
    OutOfValidity,
    InvalidElements,
}

#[derive(Clone, Copy, Debug)]
pub struct NavCandidate<'a> {
    pub key: &'a NavKey,
    pub ephemeris: &'a Ephemeris,
    pub orbit_reference: Option<Epoch>,
    pub clock_reference: Epoch,
    pub validity_half_window: Option<Duration>,
    pub native_frame: Option<NativeFrame>,
    /// Interpreted message health: Some(true) healthy, Some(false) unhealthy.
    pub health: Option<bool>,
    pub rejection: Option<NavRejection>,
}

impl NavCandidate<'_> {
    /// Propagate the selected GPS LNAV record without changing its frame.
    pub fn native_state_at(&self, t: Epoch) -> Result<NativeGpsState, StateError> {
        if self.key.sv.constellation != Constellation::GPS
            || self.key.msgtype != NavMessageType::LNAV
            || self.key.subtype.is_some()
        {
            return Err(StateError::UnsupportedMessage);
        }
        if let Some(reason) = self.rejection {
            return Err(StateError::Rejected(reason));
        }
        let toe = self.orbit_reference.ok_or(StateError::InvalidElements)?;
        if (t - toe).abs() >= Duration::from_seconds(7200.0) {
            return Err(StateError::OutOfValidity);
        }
        if (t - self.clock_reference).abs() >= Duration::from_seconds(7200.0) {
            return Err(StateError::OutOfValidity);
        }
        let eph = self.ephemeris;
        let helper = eph
            .helper(self.key.sv, t)
            .ok_or(StateError::InvalidElements)?;
        let (position, velocity) = helper
            .position_velocity()
            .ok_or(StateError::InvalidElements)?;
        let dt = (t - self.clock_reference).to_seconds();
        let clock =
            eph.clock_bias + eph.clock_drift * dt + eph.clock_drift_rate * dt * dt + helper.dtr;
        if !position
            .iter()
            .chain(velocity.iter())
            .all(|v| v.is_finite())
            || !clock.is_finite()
        {
            return Err(StateError::InvalidElements);
        }
        Ok(NativeGpsState {
            epoch: t,
            frame: NativeFrame::GpsBroadcastWgs84,
            position_km: position.into(),
            velocity_km_s: velocity.into(),
            clock_correction_s: clock,
        })
    }
}

#[derive(Debug)]
pub struct NavSelection<'a> {
    pub candidates: Vec<NavCandidate<'a>>,
    pub selected: Option<usize>,
}

impl<'a> NavSelection<'a> {
    pub fn chosen(&self) -> Option<&NavCandidate<'a>> {
        self.selected.map(|index| &self.candidates[index])
    }
}

fn required_elements(eph: &Ephemeris) -> bool {
    const FIELDS: &[&str] = &[
        "sqrta", "e", "i0", "omega", "omega0", "m0", "toe", "week", "cuc", "cus", "cic", "cis",
        "crc", "crs", "deltaN", "idot", "omegaDot",
    ];
    FIELDS
        .iter()
        .all(|name| eph.get_orbit_f64(name).is_some_and(f64::is_finite))
        && eph.get_orbit_f64("sqrta").is_some_and(|v| v > 0.0)
        && eph
            .get_orbit_f64("e")
            .is_some_and(|v| (0.0..1.0).contains(&v))
        && [eph.clock_bias, eph.clock_drift, eph.clock_drift_rate]
            .into_iter()
            .all(f64::is_finite)
}

impl Rinex {
    /// Select a GPS LNAV record usable by [`NavCandidate::native_state_at`].
    /// Other NAV messages are reported as unsupported; equal-distance ties
    /// are resolved by the complete navigation key.
    pub fn nav_select_gps_lnav(
        &self,
        sv: SV,
        t: Epoch,
        unknown: UnknownHealthPolicy,
    ) -> NavSelection<'_> {
        let mut candidates = Vec::new();
        let mut best: Option<(usize, Duration, NavKey)> = None;
        for (key, eph) in self
            .nav_ephemeris_frames_iter()
            .filter(|(key, _)| key.sv == sv)
        {
            let supported = key.sv.constellation == Constellation::GPS
                && key.msgtype == NavMessageType::LNAV
                && key.subtype.is_none();
            let toe = if supported { eph.toe(sv) } else { None };
            let health = eph
                .orbits
                .get("health")
                .and_then(|item| item.as_gps_qzss_l1l2l5_health_flag())
                .map(|flag| flag.healthy());
            let mut rejection = if !supported {
                Some(NavRejection::UnsupportedMessage)
            } else if toe.is_none() {
                Some(NavRejection::MissingOrbitReference)
            } else if !required_elements(eph) {
                Some(NavRejection::MissingOrbitField)
            } else if health == Some(false) {
                Some(NavRejection::Unhealthy)
            } else if health.is_none() && unknown == UnknownHealthPolicy::Reject {
                Some(NavRejection::UnknownHealth)
            } else if (t - toe.unwrap()).abs() >= Duration::from_seconds(7200.0) {
                Some(NavRejection::OutOfValidity)
            } else if (t - key.epoch).abs() >= Duration::from_seconds(7200.0) {
                Some(NavRejection::OutOfValidity)
            } else {
                None
            };
            let index = candidates.len();
            let mut candidate = NavCandidate {
                key,
                ephemeris: eph,
                orbit_reference: toe,
                clock_reference: key.epoch,
                validity_half_window: supported.then_some(Duration::from_seconds(7200.0)),
                native_frame: supported.then_some(NativeFrame::GpsBroadcastWgs84),
                health,
                rejection,
            };
            if rejection.is_none() && candidate.native_state_at(t).is_err() {
                rejection = Some(NavRejection::Unpropagatable);
                candidate.rejection = rejection;
            }
            candidates.push(candidate);
            if rejection.is_none() {
                let rank = ((t - toe.unwrap()).abs(), *key);
                if best.as_ref().is_none_or(|(_, d, k)| rank < (*d, *k)) {
                    best = Some((index, rank.0, rank.1));
                }
            }
        }
        let selected = best.map(|v| v.0);
        for (index, candidate) in candidates.iter_mut().enumerate() {
            if candidate.rejection.is_none() && selected != Some(index) {
                candidate.rejection = Some(NavRejection::LowerRank);
            }
        }
        NavSelection {
            candidates,
            selected,
        }
    }

    /// Select a GLONASS FDMA (or legacy LNAV) record usable for native state.
    /// The exclusive 900 s half-window is an integration policy, not an
    /// accuracy guarantee. Other GLONASS message families are rejected.
    pub fn nav_select_glonass_fdma(
        &self,
        sv: SV,
        t: Epoch,
        unknown: UnknownHealthPolicy,
    ) -> NavSelection<'_> {
        use crate::navigation::glonass::GlonassHealth;

        let mut candidates = Vec::new();
        let mut best: Option<(usize, Duration, u8, NavKey)> = None;
        for (key, eph) in self
            .nav_ephemeris_frames_iter()
            .filter(|(key, _)| key.sv == sv)
        {
            let supported = key.sv.constellation == Constellation::Glonass
                && (if self.header.version.major >= 4 {
                    key.msgtype == NavMessageType::FDMA
                } else {
                    key.msgtype == NavMessageType::LNAV
                })
                && key.subtype.is_none();
            let health = eph
                .orbits
                .get("health")
                .and_then(|item| item.as_glonass_health_flag())
                .map(|flag| !flag.intersects(GlonassHealth::UNHEALTHY));
            let mut rejection = if !supported {
                Some(NavRejection::UnsupportedMessage)
            } else if !super::glonass_fdma::fields_present(eph) {
                Some(NavRejection::MissingOrbitField)
            } else if health == Some(false) {
                Some(NavRejection::Unhealthy)
            } else if health.is_none() && unknown == UnknownHealthPolicy::Reject {
                Some(NavRejection::UnknownHealth)
            } else if eph.get_orbit_f64("dataValidity").is_some_and(|v| v != 0.0) {
                Some(NavRejection::InvalidData)
            } else if (t - key.epoch).abs() >= Duration::from_seconds(900.0) {
                Some(NavRejection::OutOfValidity)
            } else {
                None
            };
            let index = candidates.len();
            let mut candidate = NavCandidate {
                key,
                ephemeris: eph,
                orbit_reference: supported.then_some(key.epoch),
                clock_reference: key.epoch,
                validity_half_window: supported.then_some(Duration::from_seconds(900.0)),
                native_frame: supported.then_some(NativeFrame::GlonassBroadcastPz90),
                health,
                rejection,
            };
            if rejection.is_none() && candidate.fdma_state_at(t).is_err() {
                rejection = Some(NavRejection::Unpropagatable);
                candidate.rejection = rejection;
            }
            candidates.push(candidate);
            if rejection.is_none() {
                let rank = (
                    (t - key.epoch).abs(),
                    u8::from(key.msgtype == NavMessageType::LNAV),
                    *key,
                );
                if best.as_ref().is_none_or(|(_, d, p, k)| rank < (*d, *p, *k)) {
                    best = Some((index, rank.0, rank.1, rank.2));
                }
            }
        }
        let selected = best.map(|v| v.0);
        for (index, candidate) in candidates.iter_mut().enumerate() {
            if candidate.rejection.is_none() && selected != Some(index) {
                candidate.rejection = Some(NavRejection::LowerRank);
            }
        }
        NavSelection {
            candidates,
            selected,
        }
    }
}
