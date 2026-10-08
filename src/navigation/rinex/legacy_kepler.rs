//! Message-aware native fixed-frame state for traditional Kepler messages.
use crate::{
    navigation::{
        ephemeris::kepler::KeplerSolveError,
        rinex::selection::{NativeFrame, NavCandidate},
        NavMessageType,
    },
    prelude::Constellation,
    prelude::Epoch,
};

/// Broadcast position and its time derivative in the same native ECEF frame.
/// Clock excludes signal group delay and frame/signal-specific corrections.
#[derive(Clone, Copy, Debug)]
pub struct KeplerState {
    pub epoch: Epoch,
    pub position_km: [f64; 3],
    pub velocity_km_s: [f64; 3],
    pub clock_correction_s: f64,
    pub frame: NativeFrame,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeplerStateError {
    UnsupportedMessage,
    RejectedCandidate,
    OutOfValidity,
    MissingElement,
    InvalidElement,
    NonConvergence,
}

impl From<KeplerSolveError> for KeplerStateError {
    fn from(value: KeplerSolveError) -> Self {
        match value {
            KeplerSolveError::MissingElement => Self::MissingElement,
            KeplerSolveError::InvalidElement => Self::InvalidElement,
            KeplerSolveError::NonConvergence => Self::NonConvergence,
        }
    }
}

impl NavCandidate<'_> {
    /// Propagate the selected record at `t` in its native broadcast frame,
    /// with no inter-frame conversion. Clock excludes signal group delay.
    /// Covers GPS/QZSS/NavIC LNAV, Galileo INAV/FNAV, BeiDou D1 non-GEO and
    /// BeiDou D2 GEO. D1/D2 identify the orbit class in RINEX 4.
    pub fn kepler_state_at(&self, t: Epoch) -> Result<KeplerState, KeplerStateError> {
        use Constellation::*;
        use NavMessageType::*;
        if !matches!(
            (self.key.sv.constellation, self.key.msgtype),
            (GPS | QZSS | IRNSS, LNAV) | (Galileo, INAV | FNAV) | (BeiDou, D1 | D2)
        ) || (self.key.sv.constellation == BeiDou
            && ((self.key.msgtype == D1 && self.key.sv.is_beidou_geo())
                || (self.key.msgtype == D2 && !self.key.sv.is_beidou_geo())))
            || self.key.subtype.is_some()
        {
            return Err(KeplerStateError::UnsupportedMessage);
        }
        if self.rejection.is_some() {
            return Err(KeplerStateError::RejectedCandidate);
        }
        let toe = self
            .orbit_reference
            .ok_or(KeplerStateError::MissingElement)?;
        let half = self
            .validity_half_window
            .ok_or(KeplerStateError::MissingElement)?;
        if (t - toe).abs() >= half {
            return Err(KeplerStateError::OutOfValidity);
        }
        if (t - self.clock_reference).abs() >= half {
            return Err(KeplerStateError::OutOfValidity);
        }
        let helper = self.ephemeris.helper_checked_for_geo(
            self.key.sv,
            t,
            self.key.sv.constellation == BeiDou && self.key.msgtype == D2,
        )?;
        let (position, velocity) = if self.key.msgtype == D2 {
            helper.beidou_geo_ecef_pv()
        } else {
            helper
                .position_velocity()
                .ok_or(KeplerStateError::UnsupportedMessage)?
        };
        let dt_clock = (t - self.clock_reference).to_seconds();
        let eph = self.ephemeris;
        let clock_correction_s = eph.clock_bias
            + eph.clock_drift * dt_clock
            + eph.clock_drift_rate * dt_clock * dt_clock
            + helper.dtr;
        if !position
            .iter()
            .chain(velocity.iter())
            .all(|v| v.is_finite())
            || !clock_correction_s.is_finite()
        {
            return Err(KeplerStateError::InvalidElement);
        }
        let frame = self
            .native_frame
            .ok_or(KeplerStateError::UnsupportedMessage)?;
        Ok(KeplerState {
            epoch: t,
            position_km: position.into(),
            velocity_km_s: velocity.into(),
            clock_correction_s,
            frame,
        })
    }
}
