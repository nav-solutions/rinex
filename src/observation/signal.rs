use crate::{
    observation::LLIFlags,
    observation::SNR,
    prelude::{Observable, SV},
};

/// [SignalObservation] is the result of sampling one signal at
/// one point in time, by a GNSS receiver.
#[derive(Default, Clone, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SignalObservation {
    /// Satellite (signal source)
    pub satellite: SV,

    /// Actual measurement. Unit depends on [Observable].
    pub value: f64,

    /// [Observable]
    pub observable: Observable,

    /// Measurement status and phase tracker status (when present)
    pub lli_flags: Option<LLIFlags>,

    /// Possible S/N ratio estimate.
    pub signal_noise_ratio: Option<SNR>,
}

impl SignalObservation {
    /// Builds new signal observation.
    pub fn new(satellite: SV, observable: Observable, value: f64) -> Self {
        Self {
            value,
            satellite,
            observable,
            lli_flags: None,
            signal_noise_ratio: None,
        }
    }

    /// Copy and define [SNR]
    pub fn with_snr(&self, signal_noise_ratio: SNR) -> Self {
        let mut s = self.clone();
        s.signal_noise_ratio = Some(signal_noise_ratio);
        s
    }

    /// Copy and define [Observable]
    pub fn with_observable(&self, observable: Observable) -> Self {
        let mut s = self.clone();
        s.observable = observable.clone();
        s
    }

    /// Copy and define [LLIFlags]
    pub fn with_lli_flags(&self, flags: LLIFlags) -> Self {
        let mut s = self.clone();
        s.lli_flags = Some(flags);
        s
    }

    /// [Observation] is said OK when
    ///  - If LLI is present it must match [LLIFlags::OK_OR_UNKNOWN]
    ///  - If SNR is present, it must be [SNR::strong]
    ///  - NB: this is pessimistic, missing LLI and/or SNR defaults to OK.
    pub fn is_ok(self) -> bool {
        let lli_ok = self.lli_flags.unwrap_or(LLIFlags::OK_OR_UNKNOWN) == LLIFlags::OK_OR_UNKNOWN;
        let snr_ok = self.signal_noise_ratio.unwrap_or_default().strong();
        lli_ok && snr_ok
    }

    /// [Observation::is_ok] with additional SNR criteria to match (above or equals).
    /// SNR becomes mandatory otherwise we return false here.
    pub fn is_ok_snr(&self, min_snr: SNR) -> bool {
        if self
            .lli_flags
            .unwrap_or(LLIFlags::OK_OR_UNKNOWN)
            .intersects(LLIFlags::OK_OR_UNKNOWN)
        {
            if let Some(snr) = self.signal_noise_ratio {
                snr >= min_snr
            } else {
                false
            }
        } else {
            false
        }
    }
}
