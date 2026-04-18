#[cfg(doc)]
use crate::prelude::Rinex;

use crate::prelude::{Constellation, ParsingError};

/// [Type] describes all supported [Rinex] formats.
#[derive(Default, Copy, Clone, PartialEq, Debug, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Type {
    /// Observation RINEX ("Observation Data") describes
    /// signal observations and measurements. These files
    /// carry the raw measurements that the navigation process requires.
    #[default]
    Observation,

    /// Navigation RINEX ("Navigation Data") describes
    /// data (messages) broadcasted by satellites over radio.
    /// This is why it is oftentimes referred to as "Broadcast" navigation.
    /// This contains all data required by the navigation process:
    /// - Ephemeris to describe the state of each satellite
    /// - Description of the timescales
    /// - Description of some of the biases like
    Navigation,

    /// Meteo RINEX ("Meteo Data") describes meteo sensor measurements
    /// specifically. They can be used to enhance a possible
    /// environmental bias model by real field data.
    Meteo,

    /// Clock RINEX ("Clock Data") describe the state of ground
    /// or satellite clocks precisely.
    Clock,

    /// Antenna RINEX (or ANTEX) are special RINEX and serve
    /// as a database to describe and compensate antenna characteristics precisely.
    Antenna,
}

impl std::fmt::Display for Type {
    /// Formats this [Type] in a readable fashion
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Observation => write!(fmt, "Signal Observations"),
            Self::Navigation => write!(fmt, "Navigation Messages"),
            Self::Meteo => write!(fmt, "Meteo observations"),
            Self::Clock => write!(fmt, "Clock data"),
            Self::Antenna => write!(fmt, "Antenna database"),
        }
    }
}

impl std::fmt::UpperHex for Type {
    /// Formats this [Type] like in a RINEX Header
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Observation => write!(f, "OBSERVATION DATA"),
            Self::Navigation => write!(f, "NAVIGATION DATA"),
            Self::Meteo => write!(f, "METEOROLOGICAL DATA"),
            Self::Clock => write!(f, "CLOCK DATA"),
            Self::Antenna => write!(f, "ANTEX"),
        }
    }
}

impl std::fmt::LowerHex for Type {
    /// Formats this [Type] in a shortened (3 letter) fashion
    /// but similar to a RINEX header. For example, "OBS" for Observation data,
    /// or "NAV" for navigation data.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Observation => write!(f, "OBS"),
            Self::Navigation => write!(f, "NAV"),
            Self::Meteo => write!(f, "MET"),
            Self::Clock => write!(f, "CLK"),
            Self::Antenna => write!(f, "ATX"),
        }
    }
}

impl std::str::FromStr for Type {
    type Err = ParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();

        if s.eq("navigation data") || s.contains("nav data") {
            Ok(Self::Navigation)
        } else if s.eq("observation data") {
            Ok(Self::Observation)
        } else if s.eq("meteorological data") {
            Ok(Self::Meteo)
        } else if s.eq("clock data") || s.eq("c") {
            Ok(Self::Clock)
        } else if s.eq("antex") {
            Ok(Self::Antenna)
        } else {
            Err(ParsingError::TypeParsing)
        }
    }
}
