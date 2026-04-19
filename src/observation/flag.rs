use std::str::FromStr;

use crate::errors::ObsRINEXParsingError;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// `EpochFlag` validates an epoch,
/// or describes possible events that occurred
#[derive(Copy, Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EpochFlag {
    /// Epoch is sane. All observations to follow
    /// can safely be used in a navigation algorithm.
    #[default]
    Ok,

    /// Power failure since previous epoch: the receiver
    /// might be in unstable state and you should expect a loss of precision.
    /// It is not recommended to use the following observations
    /// in a precise navigation algorithm.
    PowerFailure,

    /// Antenna is being moved at current epoch yet the receiver
    /// is still providing data: you should expect a degradation in the precision.
    /// It is not recommended to use the following observations
    /// in a precise navigation algorithm.
    AntennaBeingMoved,

    /// Site has changed, received has moved since last epoch, possibly
    /// after a long dap (period without any measurements).
    /// The parser should expect the presence of special Header
    /// markers and update the new site position accordingly.
    /// The number of measurements describes the number of header lines
    /// to follow. Comments may still exist.
    /// Once all possible markers have been parsed and updated, the
    /// parser should continue the parsing process.
    /// This is currently not supported by this library.
    /// Track our github issues to see the progress on that.
    NewSiteOccupation,

    /// New information to come after this epoch. This is used
    /// to update the information provided by the Header (static information)
    /// and is currently not supported by this library.
    /// Track our github issues to see the progress on that.
    /// The number of measurements describes the number of header fields to follow.
    /// The reader should grab all following static information and then continue
    /// the parsing process.
    HeaderInformationFollows,

    /// External event - significant event in this epoch
    ExternalEvent,

    /// Cycle slip event declared at this epoch.
    CycleSlip,
}

impl EpochFlag {
    /// Returns true if [`Epoch`] attached to this flag is valid
    pub fn is_ok(self) -> bool {
        self == Self::Ok
    }
}

impl FromStr for EpochFlag {
    type Err = ObsRINEXParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(EpochFlag::Ok),
            "1" => Ok(EpochFlag::PowerFailure),
            "2" => Ok(EpochFlag::AntennaBeingMoved),
            "3" => Ok(EpochFlag::NewSiteOccupation),
            "4" => Ok(EpochFlag::HeaderInformationFollows),
            "5" => Ok(EpochFlag::ExternalEvent),
            "6" => Ok(EpochFlag::CycleSlip),
            _ => Err(ObsRINEXParsingError::EpochFlag),
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
