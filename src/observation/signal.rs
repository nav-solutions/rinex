use crate::{
    observation::LliFlags,
    observation::SNR,
    prelude::{Observable, SV},
};

/// [SignalObservation] is the result of sampling one signal at
/// one point in time, by a GNSS receiver.
#[derive(Default, Clone, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SignalObservation {
    /// [SV] is the signal source
    pub sv: SV,
    /// Actual measurement. Unit depends on [Observable].
    pub value: f64,
    /// [Observable]
    pub observable: Observable,
    /// Lock loss indicator (when present)
    pub lli: Option<LliFlags>,
    /// SNR estimate (when present)
    pub snr: Option<SNR>,
}

impl SignalObservation {
    /// Builds new signal observation
    pub fn new(sv: SV, observable: Observable, value: f64) -> Self {
        Self {
            sv,
            observable,
            value,
            lli: None,
            snr: None,
        }
    }

    /// Copy and define [SNR]
    pub fn with_snr(&self, snr: SNR) -> Self {
        let mut s = self.clone();
        s.snr = Some(snr);
        s
    }

    /// [Observation] is said OK when
    ///  - If LLI is present it must match [LliFlags::OK_OR_UNKNOWN]
    ///  - If SNR is present, it must be [SNR::strong]
    ///  - NB: when both are missing, we still return OK.
    /// This allows method that Iterate over OK Epoch Data to consider
    /// data when SNR or LLI are missing.
    pub fn is_ok(self) -> bool {
        let lli_ok = self.lli.unwrap_or(LliFlags::OK_OR_UNKNOWN) == LliFlags::OK_OR_UNKNOWN;
        let snr_ok = self.snr.unwrap_or_default().strong();
        lli_ok && snr_ok
    }

    /// [Observation::is_ok] with additional SNR criteria to match (>=).
    /// SNR must then be present otherwise this is not OK.
    pub fn is_ok_snr(&self, min_snr: SNR) -> bool {
        if self
            .lli
            .unwrap_or(LliFlags::OK_OR_UNKNOWN)
            .intersects(LliFlags::OK_OR_UNKNOWN)
        {
            if let Some(snr) = self.snr {
                snr >= min_snr
            } else {
                false
            }
        } else {
            false
        }
    }
}
