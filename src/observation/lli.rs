use bitflags::bitflags;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    /// Measurement status and phase tracker status (when present)
    pub struct LLIFlags: u8 {
        /// Observation is sane or status is not known, assumed sane.
        const OK_OR_UNKNOWN = 0x00;

        /// Lock lost between previous observation and current observation,
        /// cycle slip is possible
        const LOCK_LOSS = 0x01;

        /// Half cycle slip event.
        const HALF_CYCLE_SLIP = 0x02;

        /// A/S activated on measurement system, expect signal quality degradations.
        const UNDER_ANTI_SPOOFING = 0x04;
    }
}
