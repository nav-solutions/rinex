//! RINEX file format description

use crate::prelude::{Constellation, ParsingError};

/// [Type] describes all supported [RINEX] formats
#[derive(Default, Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Type {
    /// Describes Observation Data (OBS),
    /// Phase & Pseudo range measurements
    #[default]
    ObservationData,

    /// Describes Navigation Data (NAV)
    /// Ephemeris data, and other possible
    /// modern declinations
    NavigationData,

    /// Describes Meteorological data (MET)
    MeteoData,

    /// Clock Data (CLK)
    ClockData,

    /// Antenna Data (ATX or Antex) special RINEX format,
    /// contains sets of Antenna characterization coefficients.
    /// No database is furnished by this lib (way too big).
    /// Users interested in such calibrations / conversions / calculations,
    /// should use this parser as a mean to extract the antenna coefficients solely
    AntennaData,

    /// DORIS measurements.
    /// Things to note when processing DORIS files:
    ///   - header.receiver.sn is the DORIS chain #ID
    ///   - header.receiver.model is the DORIS instrument type
    ///   - header.receiver.firmware is the DORIS/DIODE version used
    /// The list of observables being published, is described in the dedicated
    /// header fields, and serve as Key index in the Record content.
    /// The API exposes measurements in TAI Timescale, we can't directly represent
    /// the DORIS timescale, therefore we apply the appropriate offsets.
    DORIS,
}

impl std::fmt::Display for Type {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ObservationData => write!(fmt, "OBS DATA"),
            Self::NavigationData => write!(fmt, "NAVIGATION DATA"),
            Self::MeteoData => write!(fmt, "METEO DATA"),
            Self::ClockData => write!(fmt, "CLOCK DATA"),
            Self::AntennaData => write!(fmt, "ANTEX"),
            Self::DORIS => write!(fmt, "DORIS"),
        }
    }
}

impl Type {
    /// Converts [Type] to standard descriptor
    pub fn to_standard_string(&self, constellation: Option<Constellation>) -> String {
        match *self {
            Self::ObservationData => String::from("OBSERVATION DATA"),
            Self::NavigationData => match constellation {
                Some(Constellation::Glonass) => String::from("Glonass NAV"),
                _ => String::from("NAV DATA"),
            },
            Self::MeteoData => String::from("METEOROLOGICAL DATA"),
            Self::ClockData => String::from("CLOCK DATA"),
            Self::AntennaData => String::from("ANTEX"),
            Self::DORIS => String::from("DORIS"),
        }
    }
}

impl std::str::FromStr for Type {
    type Err = ParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        if s.eq("navigation data") || s.contains("nav data") {
            Ok(Self::NavigationData)
        } else if s.eq("observation data") {
            Ok(Self::ObservationData)
        } else if s.eq("meteorological data") {
            Ok(Self::MeteoData)
        } else if s.eq("clock data") || s.eq("c") {
            Ok(Self::ClockData)
        } else if s.eq("antex") {
            Ok(Self::AntennaData)
        } else if s.eq("doris") || s.eq("d") {
            Ok(Self::DORIS)
        } else {
            Err(ParsingError::TypeParsing)
        }
    }
}
