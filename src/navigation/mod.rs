//! Navigation module
mod earth_orientation;
mod ephemeris;
mod frame;
mod header;
mod ionosphere;
mod message;
mod parsing;
mod time;

pub mod rinex;

pub(crate) mod formatting;

pub(crate) use formatting::format;
pub(crate) use parsing::{is_new_epoch, parse_epoch};

pub use crate::navigation::{
    earth_orientation::EarthOrientation,
    ephemeris::{flags::*, orbits::OrbitItem, Ephemeris},
    frame::{NavFrame, NavFrameType},
    header::HeaderFields,
    ionosphere::{
        BdModel, GloCdmaModel, IonosphereModel, KbModel, KbRegionCode, NavicKbModel,
        NavicNeqnModel, NavicNeqnRegion, NgModel, NgRegionFlags,
    },
    message::{NavMessageSubtype, NavMessageType},
    time::TimeOffset,
};

#[cfg(feature = "nav")]
pub use crate::navigation::ephemeris::kepler::{Helper, Kepler, Perturbations};

#[cfg(feature = "processing")]
pub(crate) mod mask; // mask Trait implementation

#[cfg(feature = "processing")]
pub(crate) mod decim; // decim Trait implementation

#[cfg(feature = "processing")]
pub(crate) mod repair; // repair Trait implementation

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use crate::prelude::{Constellation, Epoch, ParsingError, TimeScale, SV};

/// [TimeScale] of the navigation messages of a [Constellation].
/// NavIC messages are expressed in GPST (RINEX 4.02 Table A30).
pub(crate) fn timescale(constellation: Constellation) -> Result<TimeScale, ParsingError> {
    match constellation {
        Constellation::IRNSS => Ok(TimeScale::GPST),
        c => c.timescale().ok_or(ParsingError::NoTimescaleDefinition),
    }
}

/// STO identity fields from RINEX Table A30, ASCII and space-padded.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StoIdentity {
    pub time_system: [u8; 4],
    pub sbas_id: [u8; 18],
    pub utc_id: [u8; 18],
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NavKey {
    /// EPH time of clock; STO/EOP reference epoch; ION model epoch as [Epoch].
    pub epoch: Epoch,

    /// [SV] broadcasting this information.
    pub sv: SV,

    /// [NavMessageType] associated to following [NavFrame]
    pub msgtype: NavMessageType,

    /// [NavFrame] type following.
    pub frmtype: NavFrameType,

    /// [NavMessageSubtype], optional fourth field of the RINEX 4.02
    /// record header, telling apart otherwise identical records.
    pub subtype: Option<NavMessageSubtype>,

    /// Complete Galileo Data sources bitfield, including signal origin and
    /// clock reference. Other constellations and non-EPH frames use None.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub galileo_data_sources: Option<u32>,

    /// Decoded transmission/frame seconds. GLONASS FDMA uses day seconds in
    /// RINEX 2 and week seconds in RINEX 3/4; other messages use their system
    /// week. None denotes an allowed missing/unknown value, distinct from zero.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub transmission_time: Option<TransmissionTime>,

    /// Time relation and distinct SBAS/UTC indicators of an STO frame.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub sto_identity: Option<StoIdentity>,
}

/// Finite decoded transmission seconds used in broadcast identity.
/// Negative and out-of-week values are retained. Positive and negative zero
/// share one identity; no quantization or tolerance is applied.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct TransmissionTime(f64);

impl TransmissionTime {
    /// Construct finite seconds, normalizing signed zero.
    pub fn new(seconds: f64) -> Result<Self, ParsingError> {
        if !seconds.is_finite() {
            return Err(ParsingError::NavTransmissionTime);
        }
        Ok(Self(if seconds == 0.0 { 0.0 } else { seconds }))
    }

    /// Decoded seconds in the source message's day/week convention.
    pub fn seconds(self) -> f64 {
        self.0
    }
}

impl Eq for TransmissionTime {}
impl PartialOrd for TransmissionTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TransmissionTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}
impl std::hash::Hash for TransmissionTime {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.0.to_bits(), state);
    }
}
#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for TransmissionTime {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(f64::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl NavKey {
    /// Build/rebuild the complete broadcast key from decoded payload fields.
    /// Pass the source revision, epoch, satellite and message header fields.
    /// For legacy Galileo, LNAV is resolved from Data sources. STO requires
    /// `epoch` to equal its reference epoch. After editing identity fields,
    /// remove the old map entry and insert the frame under this rebuilt key.
    /// Old serialized keys with absent extensions must also be rebuilt.
    pub fn from_frame(
        version: crate::Version,
        epoch: crate::Epoch,
        sv: crate::SV,
        msgtype: NavMessageType,
        subtype: Option<NavMessageSubtype>,
        frame: &NavFrame,
    ) -> Result<Self, ParsingError> {
        parsing::key_from_frame(version, epoch, sv, msgtype, subtype, frame)
    }
}

/// Navigation data indexed by complete broadcast identity. Queries by epoch/SV
/// can return several messages; callers choose the appropriate broadcast.
/// Direct map edits must keep keys consistent with payloads (see [NavKey::from_frame]).
pub type Record = BTreeMap<NavKey, NavFrame>;
