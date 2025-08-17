use crate::{
    navigation::{EarthOrientation, Ephemeris, IonosphereModel, TimeOffset},
    prelude::ParsingError,
};

#[cfg(doc)]
use crate::prelude::TimeScale;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Navigation Frame classes, describes the [NavFrame] to follow.
#[derive(Default, Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NavFrameType {
    /// Ephemeris frames, exists in all RINEX revisions.
    #[default]
    Ephemeris,

    /// System Time Offset frames, only exist in newer revisions.
    SystemTimeOffset,

    /// Earth Orientation frames, only exist in newer revisions.
    EarthOrientation,

    /// Ionosphere models, only exist in newer revisions.
    IonosphereModel,
}

impl std::fmt::Display for NavFrameType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Ephemeris => f.write_str("EPH"),
            Self::SystemTimeOffset => f.write_str("STO"),
            Self::EarthOrientation => f.write_str("EOP"),
            Self::IonosphereModel => f.write_str("ION"),
        }
    }
}

impl std::str::FromStr for NavFrameType {
    type Err = ParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let c = s.to_uppercase();
        let c = c.trim();
        match c {
            "EPH" => Ok(Self::Ephemeris),
            "STO" => Ok(Self::SystemTimeOffset),
            "EOP" => Ok(Self::EarthOrientation),
            "ION" => Ok(Self::IonosphereModel),
            _ => Err(ParsingError::NavFrameClass),
        }
    }
}

/// [NavFrame] describes the navigation message to follow.
/// Several types exist
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum NavFrame {
    /// [Ephemeris] exist in all revisions and give
    /// the (interpreted) content of the radio message.
    /// Usually aligned at midnight and published at
    EPH(Ephemeris),

    /// [EarthOrientation] frames appeared in RINEXv4 to describe
    /// the Earth state precisely.
    EOP(EarthOrientation),

    /// [IonosphereModel]s were introduced in RINEXv4 to describe
    /// the state of the ionosphere during the day course. Until
    /// RINEXv3 (included), the model is refreshed once per RINEX publication,
    /// so usually a 24h period.
    ION(IonosphereModel),

    /// [TimeOffset] frames were introduced in RINEXv4 to describe
    /// the state of GNSS [Timescale]s more precisely during the course of the day.
    /// Until RINEXv3 (included), their state is updated one per RINEX publication,
    /// so typically once per day.
    STO(TimeOffset),
}

impl NavFrame {
    /// Unwrap this [NavFrame] as [Ephemeris] (if possible).
    pub fn as_ephemeris(&self) -> Option<&Ephemeris> {
        match self {
            Self::EPH(eph) => Some(eph),
            _ => None,
        }
    }

    /// Unwrap this [NavFrame] as mutable [Ephemeris] (if possible).
    pub fn as_mut_ephemeris(&mut self) -> Option<&mut Ephemeris> {
        match self {
            Self::EPH(eph) => Some(eph),
            _ => None,
        }
    }

    /// Unwrap this [NavFrame] as [TimeOffset] (if possible).
    pub fn as_system_time(&self) -> Option<&TimeOffset> {
        match self {
            Self::STO(fr) => Some(fr),
            _ => None,
        }
    }

    /// Unwrap this [NavFrame] as mutable [TimeOffset] (if possible).
    pub fn as_mut_system_time(&mut self) -> Option<&mut TimeOffset> {
        match self {
            Self::STO(fr) => Some(fr),
            _ => None,
        }
    }

    /// Unwrap this [NavFrame] as [IonosphereModel] (if possible).
    pub fn as_ionosphere_model(&self) -> Option<&IonosphereModel> {
        match self {
            Self::ION(fr) => Some(fr),
            _ => None,
        }
    }

    /// Unwrap this [NavFrame] as mutable [IonosphereModel] (if possible).
    pub fn as_mut_ionosphere_model(&mut self) -> Option<&mut IonosphereModel> {
        match self {
            Self::ION(fr) => Some(fr),
            _ => None,
        }
    }

    /// Unwrap this [NavFrame] as [EarthOrientation] parameters (if possible).
    pub fn as_earth_orientation(&self) -> Option<&EarthOrientation> {
        match self {
            Self::EOP(fr) => Some(fr),
            _ => None,
        }
    }

    /// Unwrap this [NavFrame] as mutable [EarthOrientation] parameters (if possible).
    pub fn as_mut_earth_orientation(&mut self) -> Option<&mut EarthOrientation> {
        match self {
            Self::EOP(fr) => Some(fr),
            _ => None,
        }
    }
}
