//! SBAS GEO broadcast state in its native terrestrial axes.
//!
//! RINEX 4.02 Table A28 gives position (km), velocity (km/s), and constant
//! acceleration (km/s²) at the GPS-time epoch on the clock line. The third
//! clock slot is message transmission time, not quadratic clock drift.
//! This module does not apply SBAS differential or integrity corrections.
use super::selection::{NativeFrame, NavCandidate, NavRejection};
use crate::{navigation::NavMessageType, prelude::Epoch};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SbasError {
    UnsupportedMessage,
    Rejected(NavRejection),
    MissingField(&'static str),
    InvalidField(&'static str),
    OutOfValidity,
}

impl std::fmt::Display for SbasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SbasError {}

#[derive(Clone, Copy, Debug)]
pub struct SbasState {
    pub epoch: Epoch,
    pub frame: NativeFrame,
    pub position_km: [f64; 3],
    pub velocity_km_s: [f64; 3],
    /// aGf0 + aGf1*(t-Toc), seconds; no signal or SBAS correction applied.
    pub clock_correction_s: f64,
}

impl NavCandidate<'_> {
    fn sbas_dt_s(&self, t: Epoch) -> Result<f64, SbasError> {
        if !self.key.sv.constellation.is_sbas()
            || !matches!(
                self.key.msgtype,
                NavMessageType::SBAS | NavMessageType::LNAV
            )
            || self.key.subtype.is_some()
        {
            return Err(SbasError::UnsupportedMessage);
        }
        let t_ref = self
            .orbit_reference
            .ok_or(SbasError::MissingField("orbit reference"))?;
        let half = self
            .validity_half_window
            .ok_or(SbasError::MissingField("validity"))?;
        let dt = (t - t_ref).to_seconds();
        if !dt.is_finite() || dt.abs() >= half.to_seconds() {
            return Err(SbasError::OutOfValidity);
        }
        Ok(dt)
    }

    fn sbas_field(&self, name: &'static str) -> Result<f64, SbasError> {
        let value = self
            .ephemeris
            .get_orbit_f64(name)
            .ok_or(SbasError::MissingField(name))?;
        if !value.is_finite() {
            return Err(SbasError::InvalidField(name));
        }
        Ok(value)
    }

    /// Minimal SBAS broadcast clock correction at `t`, excluding the third
    /// clock-line slot (`t_tm`, message transmission time).
    pub fn sbas_clock_correction_s(&self, t: Epoch) -> Result<f64, SbasError> {
        let dt = self.sbas_dt_s(t)?;
        if let Some(
            reason @ (NavRejection::Unhealthy
            | NavRejection::UnknownHealth
            | NavRejection::InvalidData),
        ) = self.rejection
        {
            return Err(SbasError::Rejected(reason));
        }
        let a0 = self.ephemeris.clock_bias;
        let a1 = self.ephemeris.clock_drift;
        if !a0.is_finite() || !a1.is_finite() {
            return Err(SbasError::InvalidField("clock"));
        }
        Ok(a0 + a1 * dt)
    }

    /// Evaluate each broadcast axis independently within |t-Toc| < 360 s.
    /// The bound is a conservative selection policy, not a precision guarantee.
    pub fn sbas_state_at(&self, t: Epoch) -> Result<SbasState, SbasError> {
        let dt = self.sbas_dt_s(t)?;
        let position = [
            self.sbas_field("satPosX")?,
            self.sbas_field("satPosY")?,
            self.sbas_field("satPosZ")?,
        ];
        let velocity = [
            self.sbas_field("velX")?,
            self.sbas_field("velY")?,
            self.sbas_field("velZ")?,
        ];
        let acceleration = [
            self.sbas_field("accelX")?,
            self.sbas_field("accelY")?,
            self.sbas_field("accelZ")?,
        ];
        if let Some(reason) = self.rejection {
            return Err(SbasError::Rejected(reason));
        }
        let position_km = std::array::from_fn(|axis| {
            position[axis] + velocity[axis] * dt + 0.5 * acceleration[axis] * dt * dt
        });
        let velocity_km_s = std::array::from_fn(|axis| velocity[axis] + acceleration[axis] * dt);
        if !position_km
            .iter()
            .chain(velocity_km_s.iter())
            .all(|value| value.is_finite())
        {
            return Err(SbasError::InvalidField("propagated state"));
        }
        Ok(SbasState {
            epoch: t,
            frame: NativeFrame::SbasBroadcast,
            position_km,
            velocity_km_s,
            clock_correction_s: self.sbas_clock_correction_s(t)?,
        })
    }
}
