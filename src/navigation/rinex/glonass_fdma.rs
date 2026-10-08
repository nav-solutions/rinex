//! GLONASS FDMA broadcast state in its native PZ-90 Earth-fixed axes.
//!
//! RINEX 4.02 Table A15 supplies km, km/s and km/s² at a UTC epoch.
//! The equations below are the rotating-frame form of the GLONASS ICD
//! central-gravity + J2 + broadcast lunisolar-perturbation model (see
//! ESA Navipedia, GLONASS Satellite Coordinates Computation). Velocity is
//! the time derivative in these rotating axes, not inertial velocity.
use super::selection::{NativeFrame, NavCandidate, NavRejection};
use crate::{
    navigation::NavMessageType,
    prelude::{Constellation, Epoch},
};

const MU_KM3_S2: f64 = 398_600.44;
const EARTH_RADIUS_KM: f64 = 6_378.136;
const J2: f64 = 1.082_625_7e-3;
const EARTH_RATE_RAD_S: f64 = 7.292_115e-5;
const DEFAULT_STEP_S: f64 = 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FdmaError {
    UnsupportedMessage,
    Rejected(NavRejection),
    MissingField(&'static str),
    InvalidField(&'static str),
    OutOfValidity,
    InvalidStep,
}

impl std::fmt::Display for FdmaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for FdmaError {}

#[derive(Clone, Copy, Debug)]
pub struct FdmaState {
    pub epoch: Epoch,
    pub frame: NativeFrame,
    pub position_km: [f64; 3],
    pub velocity_km_s: [f64; 3],
    /// Broadcast satellite clock offset in seconds. This excludes TauC,
    /// signal-dependent delays, and any frame or light-time corrections.
    pub clock_correction_s: f64,
}

impl NavCandidate<'_> {
    fn fdma_dt_s(&self, t: Epoch) -> Result<f64, FdmaError> {
        if self.key.sv.constellation != Constellation::Glonass
            || !matches!(
                self.key.msgtype,
                NavMessageType::FDMA | NavMessageType::LNAV
            )
            || self.key.subtype.is_some()
        {
            return Err(FdmaError::UnsupportedMessage);
        }
        let t_ref = self
            .orbit_reference
            .ok_or(FdmaError::MissingField("orbit reference"))?;
        let dt = (t - t_ref).to_seconds();
        if !dt.is_finite() || dt.abs() >= 900.0 {
            return Err(FdmaError::OutOfValidity);
        }
        Ok(dt)
    }

    /// RINEX -TauN + GammaN*(t-Tb), in seconds. The message frame time is
    /// stored separately and has no quadratic clock role.
    pub fn fdma_clock_correction_s(&self, t: Epoch) -> Result<f64, FdmaError> {
        self.fdma_dt_s(t)?;
        let bias = self.ephemeris.clock_bias;
        let drift = self.ephemeris.clock_drift;
        if !bias.is_finite() || !drift.is_finite() {
            return Err(FdmaError::InvalidField("clock"));
        }
        if let Some(
            reason @ (NavRejection::Unhealthy
            | NavRejection::UnknownHealth
            | NavRejection::InvalidData),
        ) = self.rejection
        {
            return Err(FdmaError::Rejected(reason));
        }
        Ok(bias + drift * (t - self.key.epoch).to_seconds())
    }

    /// Propagate a GLONASS FDMA (or legacy LNAV) record within |t-t_ref| < 900 s.
    /// The RINEX epoch is already UTC; no three-hour offset is applied.
    pub fn fdma_state_at(&self, t: Epoch) -> Result<FdmaState, FdmaError> {
        self.fdma_state_at_with_step(t, DEFAULT_STEP_S)
    }

    /// Same model with a chosen maximum RK4 step, useful for convergence checks.
    pub fn fdma_state_at_with_step(&self, t: Epoch, step_s: f64) -> Result<FdmaState, FdmaError> {
        if !step_s.is_finite() || step_s <= 0.0 {
            return Err(FdmaError::InvalidStep);
        }
        let dt = self.fdma_dt_s(t)?;
        if dt.abs() / step_s > 10_000.0 {
            return Err(FdmaError::InvalidStep);
        }
        let read = |field| {
            let value = self
                .ephemeris
                .get_orbit_f64(field)
                .ok_or(FdmaError::MissingField(field))?;
            if !value.is_finite() {
                return Err(FdmaError::InvalidField(field));
            }
            Ok(value)
        };
        let mut state = [
            read("satPosX")?,
            read("satPosY")?,
            read("satPosZ")?,
            read("velX")?,
            read("velY")?,
            read("velZ")?,
        ];
        let perturbation = [read("accelX")?, read("accelY")?, read("accelZ")?];
        let radius2 = state[..3].iter().map(|v| v * v).sum::<f64>();
        if radius2 < EARTH_RADIUS_KM * EARTH_RADIUS_KM {
            return Err(FdmaError::InvalidField("position radius"));
        }
        if let Some(reason) = self.rejection {
            return Err(FdmaError::Rejected(reason));
        }
        let mut remaining = dt;
        while remaining.abs() > 1e-10 {
            let step = remaining.signum() * remaining.abs().min(step_s);
            state = rk4(state, perturbation, step);
            remaining -= step;
        }
        if !state.iter().all(|v| v.is_finite()) {
            return Err(FdmaError::InvalidField("propagated state"));
        }
        let clock_correction_s = self.fdma_clock_correction_s(t)?;
        Ok(FdmaState {
            epoch: t,
            frame: NativeFrame::GlonassBroadcastPz90,
            position_km: [state[0], state[1], state[2]],
            velocity_km_s: [state[3], state[4], state[5]],
            clock_correction_s,
        })
    }
}

pub(crate) fn fields_present(eph: &crate::navigation::Ephemeris) -> bool {
    [
        "satPosX", "satPosY", "satPosZ", "velX", "velY", "velZ", "accelX", "accelY", "accelZ",
    ]
    .into_iter()
    .all(|name| eph.get_orbit_f64(name).is_some_and(f64::is_finite))
        && [eph.clock_bias, eph.clock_drift]
            .into_iter()
            .all(f64::is_finite)
}

fn derivative(s: [f64; 6], acc: [f64; 3]) -> [f64; 6] {
    let r2 = s[0] * s[0] + s[1] * s[1] + s[2] * s[2];
    let r3 = r2 * r2.sqrt();
    let j = 1.5 * J2 * MU_KM3_S2 * EARTH_RADIUS_KM.powi(2) / (r2 * r3);
    let z2 = 5.0 * s[2] * s[2] / r2;
    let c = -MU_KM3_S2 / r3 - j * (1.0 - z2);
    let omega = EARTH_RATE_RAD_S;
    [
        s[3],
        s[4],
        s[5],
        (c + omega * omega) * s[0] + 2.0 * omega * s[4] + acc[0],
        (c + omega * omega) * s[1] - 2.0 * omega * s[3] + acc[1],
        (c - 2.0 * j) * s[2] + acc[2],
    ]
}

fn add_scaled(s: [f64; 6], k: [f64; 6], scale: f64) -> [f64; 6] {
    std::array::from_fn(|i| s[i] + scale * k[i])
}

fn rk4(s: [f64; 6], acc: [f64; 3], h: f64) -> [f64; 6] {
    let k1 = derivative(s, acc);
    let k2 = derivative(add_scaled(s, k1, h / 2.0), acc);
    let k3 = derivative(add_scaled(s, k2, h / 2.0), acc);
    let k4 = derivative(add_scaled(s, k3, h), acc);
    std::array::from_fn(|i| s[i] + h * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]) / 6.0)
}
