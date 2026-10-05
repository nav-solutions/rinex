#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::prelude::{Constellation, ParsingError};

#[cfg(doc)]
use crate::bibliography::Bibliography;

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
    /// NavIC L1 NAV message (RINEX 4.02)
    L1NV,
    /// Glonass L1OC CDMA message (RINEX 4.02)
    L1OC,
    /// Glonass L3OC CDMA message (RINEX 4.02)
    L3OC,
    /// LXOC: Glonass CDMA STO, EOP and ION messages (RINEX 4.02)
    LXOC,
}

impl std::str::FromStr for NavMessageType {
    type Err = ParsingError;
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
            "L1NV" => Ok(Self::L1NV),
            "L1OC" => Ok(Self::L1OC),
            "L3OC" => Ok(Self::L3OC),
            "LXOC" => Ok(Self::LXOC),
            _ => Err(ParsingError::NavMsgType),
        }
    }
}

impl NavMessageType {
    /// Returns true if this message, broadcast by `constellation`, has a
    /// representation in RINEX 2 and 3: the legacy message of each
    /// constellation, which those revisions do not name. The modern
    /// messages (CNAV, CNV1 to CNV3, L1NV, L1OC, L3OC) only exist in
    /// RINEX 4, as do the STO, EOP and ION message types.
    pub fn is_legacy(self, constellation: Constellation) -> bool {
        match self {
            Self::LNAV => true,
            Self::FDMA => constellation == Constellation::Glonass,
            Self::INAV | Self::FNAV => constellation == Constellation::Galileo,
            Self::D1 | Self::D2 => constellation == Constellation::BeiDou,
            Self::SBAS => constellation.is_sbas(),
            _ => false,
        }
    }
}

impl std::fmt::Display for NavMessageType {
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
            Self::L1NV => write!(f, "L1NV"),
            Self::L1OC => write!(f, "L1OC"),
            Self::L3OC => write!(f, "L3OC"),
            Self::LXOC => write!(f, "LXOC"),
        }
    }
}

/// Navigation message subtype, introduced by RINEX 4.02 as an optional
/// fourth field of the navigation record header line (section 5.4.1).
/// Subtypes are only defined for a few ION messages (Table 25) and
/// distinguish otherwise identical records.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NavMessageSubtype {
    /// QZSS CNVX ION: wide area coefficients
    WIDE,
    /// QZSS CNVX ION: Japan area coefficients
    JAPN,
    /// NavIC L1NV ION: Klobuchar model
    KLOB,
    /// NavIC L1NV ION: NeQuick-N model
    NEQN,
}

impl std::str::FromStr for NavMessageSubtype {
    type Err = ParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_uppercase().as_str() {
            "WIDE" => Ok(Self::WIDE),
            "JAPN" => Ok(Self::JAPN),
            "KLOB" => Ok(Self::KLOB),
            "NEQN" => Ok(Self::NEQN),
            _ => Err(ParsingError::NavMsgSubtype),
        }
    }
}

impl std::fmt::Display for NavMessageSubtype {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::WIDE => write!(f, "WIDE"),
            Self::JAPN => write!(f, "JAPN"),
            Self::KLOB => write!(f, "KLOB"),
            Self::NEQN => write!(f, "NEQN"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn message_type() {
        for (code, msgtype) in [
            ("LNAV", NavMessageType::LNAV),
            ("FDMA", NavMessageType::FDMA),
            ("CNVX", NavMessageType::CNVX),
            ("L1NV", NavMessageType::L1NV),
            ("L1OC", NavMessageType::L1OC),
            ("L3OC", NavMessageType::L3OC),
            ("LXOC", NavMessageType::LXOC),
        ] {
            assert_eq!(NavMessageType::from_str(code).unwrap(), msgtype);
            assert_eq!(msgtype.to_string(), code);
        }
        assert!(NavMessageType::from_str("LEG").is_err());
    }

    #[test]
    fn legacy_messages() {
        use crate::prelude::Constellation;

        for (msgtype, constellation, legacy) in [
            (NavMessageType::LNAV, Constellation::GPS, true),
            (NavMessageType::LNAV, Constellation::IRNSS, true),
            (NavMessageType::FDMA, Constellation::Glonass, true),
            (NavMessageType::FDMA, Constellation::GPS, false),
            (NavMessageType::INAV, Constellation::Galileo, true),
            (NavMessageType::FNAV, Constellation::Galileo, true),
            (NavMessageType::INAV, Constellation::Glonass, false),
            (NavMessageType::D1, Constellation::BeiDou, true),
            (NavMessageType::D2, Constellation::BeiDou, true),
            (NavMessageType::D2, Constellation::GPS, false),
            (NavMessageType::SBAS, Constellation::EGNOS, true),
            (NavMessageType::SBAS, Constellation::GPS, false),
            (NavMessageType::CNAV, Constellation::GPS, false),
            (NavMessageType::CNV2, Constellation::QZSS, false),
            (NavMessageType::CNV3, Constellation::BeiDou, false),
            (NavMessageType::IFNV, Constellation::Galileo, false),
            (NavMessageType::CNVX, Constellation::GPS, false),
            (NavMessageType::L1NV, Constellation::IRNSS, false),
            (NavMessageType::L1OC, Constellation::Glonass, false),
            (NavMessageType::L3OC, Constellation::Glonass, false),
            (NavMessageType::LXOC, Constellation::Glonass, false),
        ] {
            assert_eq!(
                msgtype.is_legacy(constellation),
                legacy,
                "{} {:?}",
                msgtype,
                constellation
            );
        }
    }

    #[test]
    fn message_subtype() {
        for (code, subtype) in [
            ("WIDE", NavMessageSubtype::WIDE),
            ("JAPN", NavMessageSubtype::JAPN),
            ("KLOB", NavMessageSubtype::KLOB),
            ("NEQN", NavMessageSubtype::NEQN),
        ] {
            assert_eq!(NavMessageSubtype::from_str(code).unwrap(), subtype);
            assert_eq!(subtype.to_string(), code);
        }
        assert!(NavMessageSubtype::from_str("").is_err());
        assert!(NavMessageSubtype::from_str("LNAV").is_err());
    }
}
