#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::errors::NavRINEXParsingError;

/// Support Navigation Messages.
/// Refer to [Bibliography::RINEX4] definitions.
#[derive(Default, Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NavMessageType {
    /// Legacy NAV message
    #[default]
    LNAV,

    /// Glonass FDMA message
    FDMA,

    /// Galileo FNAV message
    FNAV,

    /// Galileo INAV message
    INAV,

    /// IFNV,
    IFNV,

    /// BeiDou D1 NAV message
    D1,

    /// BeiDou D2 NAV message
    D2,

    /// D1D2
    D1D2,

    /// SBAS NAV message
    SBAS,

    /// GPS / QZSS Civilian NAV message
    CNAV,

    /// BeiDou CNV1 message
    CNV1,

    /// GPS / QZSS / BeiDou CNV2 message
    CNV2,

    /// BeiDou CNV3 message
    CNV3,

    /// CNVX special marker
    CNVX,
}

impl std::str::FromStr for NavMessageType {
    type Err = NavRINEXParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let c = s.to_uppercase();
        let c = c.trim();
        match c {
            "LNAV" => Ok(Self::LNAV),
            "FDMA" => Ok(Self::FDMA),
            "FNAV" => Ok(Self::FNAV),
            "INAV" => Ok(Self::INAV),
            "IFNV" => Ok(Self::IFNV),
            "D1" => Ok(Self::D1),
            "D2" => Ok(Self::D2),
            "D1D2" => Ok(Self::D1D2),
            "SBAS" => Ok(Self::SBAS),
            "CNAV" => Ok(Self::CNAV),
            "CNV1" => Ok(Self::CNV1),
            "CNV2" => Ok(Self::CNV2),
            "CNV3" => Ok(Self::CNV3),
            "CNVX" => Ok(Self::CNVX),
            _ => Err(NavRINEXParsingError::InvalidMessageType),
        }
    }
}

impl std::fmt::Display for NavMessageType {
    /// Formats thie [NavMessageType] in a meaningful/readable fashion
    /// (not as found in RINEX records).
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::LNAV => write!(f, "Legacy radio message"),
            Self::FNAV => write!(f, "Galileo F-NAV message"),
            Self::INAV => write!(f, "Galileo I-NAV message"),
            Self::FDMA => write!(f, "Glonass FDMA message"),
            Self::IFNV => write!(f, "IFNV"),
            Self::D1 => write!(f, "BeiDou D1 message"),
            Self::D2 => write!(f, "BeiDou D2 message"),
            Self::D1D2 => write!(f, "BeiDou D1/D2 message"),
            Self::SBAS => write!(f, "SBAS message"),
            Self::CNAV => write!(f, "GPS/QZSS civilian message"),
            Self::CNV1 => write!(f, "BeiDou CNV-1 message"),
            Self::CNV2 => write!(f, "GPS/QZSS/BeiDou CNV-2 message"),
            Self::CNV3 => write!(f, "BeiDou CNV-3 message"),
            Self::CNVX => write!(f, "BeiDou CNV-X message"),
        }
    }
}

impl std::fmt::LowerHex for NavMessageType {
    /// Formats thie [NavMessageType] in a standardize 3 or 4 letter code,
    /// as found in RINEX records.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::LNAV => write!(f, "LNAV"),
            Self::FNAV => write!(f, "FNAV"),
            Self::INAV => write!(f, "INAV"),
            Self::FDMA => write!(f, "FDMA"),
            Self::IFNV => write!(f, "IFNV"),
            Self::D1 => write!(f, "D1"),
            Self::D2 => write!(f, "D2"),
            Self::D1D2 => write!(f, "D1D2"),
            Self::SBAS => write!(f, "SBAS"),
            Self::CNAV => write!(f, "CNAV"),
            Self::CNV1 => write!(f, "CNV1"),
            Self::CNV2 => write!(f, "CNV2"),
            Self::CNV3 => write!(f, "CNV3"),
            Self::CNVX => write!(f, "CNVX"),
        }
    }
}
