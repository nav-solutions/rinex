use crate::prelude::ParsingError;
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// `EpochFlag` validates an epoch,
/// or describes possible events that occurred
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EpochFlag {
    /// Epoch is sane
    Ok,

    /// Power failure since previous epoch
    PowerFailure,

    /// Antenna is being moved at current epoch
    AntennaBeingMoved,

    /// Site has changed, received has moved since last epoch
    NewSiteOccupation,

    /// New information to come after this epoch
    HeaderInformationFollows,

    /// External event - significant event in this epoch
    ExternalEvent,

    /// Cycle slip at this epoch
    CycleSlip,
}

impl Default for EpochFlag {
    fn default() -> Self {
        Self::Ok
    }
}

impl EpochFlag {
    /// Returns true if [`Epoch`] attached to this flag is valid
    pub fn is_ok(self) -> bool {
        self == Self::Ok
    }
}

impl FromStr for EpochFlag {
    type Err = ParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(EpochFlag::Ok),
            "1" => Ok(EpochFlag::PowerFailure),
            "2" => Ok(EpochFlag::AntennaBeingMoved),
            "3" => Ok(EpochFlag::NewSiteOccupation),
            "4" => Ok(EpochFlag::HeaderInformationFollows),
            "5" => Ok(EpochFlag::ExternalEvent),
            "6" => Ok(EpochFlag::CycleSlip),
            _ => Err(ParsingError::EpochFlag),
        }
    }
}

impl std::fmt::Display for EpochFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EpochFlag::Ok => "0".fmt(f),
            EpochFlag::PowerFailure => "1".fmt(f),
            EpochFlag::AntennaBeingMoved => "2".fmt(f),
            EpochFlag::NewSiteOccupation => "3".fmt(f),
            EpochFlag::HeaderInformationFollows => "4".fmt(f),
            EpochFlag::ExternalEvent => "5".fmt(f),
            EpochFlag::CycleSlip => "6".fmt(f),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_default() {
        assert_eq!(EpochFlag::default(), EpochFlag::Ok);
    }
    #[test]
    fn from_str() {
        assert_eq!(EpochFlag::from_str("0").unwrap(), EpochFlag::Ok);
        assert_eq!(EpochFlag::from_str("1").unwrap(), EpochFlag::PowerFailure);
        assert_eq!(
            EpochFlag::from_str("2").unwrap(),
            EpochFlag::AntennaBeingMoved
        );
        assert_eq!(
            EpochFlag::from_str("3").unwrap(),
            EpochFlag::NewSiteOccupation
        );
        assert_eq!(
            EpochFlag::from_str("4").unwrap(),
            EpochFlag::HeaderInformationFollows
        );
        assert_eq!(EpochFlag::from_str("5").unwrap(), EpochFlag::ExternalEvent);
        assert_eq!(EpochFlag::from_str("6").unwrap(), EpochFlag::CycleSlip);
        assert!(EpochFlag::from_str("7").is_err());
    }
    #[test]
    fn to_str() {
        assert_eq!(format!("{}", EpochFlag::Ok), "0");
        assert_eq!(format!("{}", EpochFlag::PowerFailure), "1");
        assert_eq!(format!("{}", EpochFlag::AntennaBeingMoved), "2");
        assert_eq!(format!("{}", EpochFlag::NewSiteOccupation), "3");
        assert_eq!(format!("{}", EpochFlag::HeaderInformationFollows), "4");
        assert_eq!(format!("{}", EpochFlag::ExternalEvent), "5");
        assert_eq!(format!("{}", EpochFlag::CycleSlip), "6");
    }
}
