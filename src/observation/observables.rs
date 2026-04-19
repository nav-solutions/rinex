//! Radio signal Observables
use crate::{
    errors::ObsRINEXParsingError,
    prelude::{Carrier, Constellation, Error},
};

use arrayvec::ArrayString;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::str::FromStr;

/// [Observable] describes all possible Radio signal observations.
/// These are specific to Observation RINEX (or compressed CRINEX) files.
/// Meteo RINEX files have their dedicated list of observables.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Hash, Ord, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Observable {
    /// Doppler shift.
    Doppler(ArrayString<3>),

    /// Carrier phase range (not cycles!) as meters of signal propagation.
    PhaseRange(ArrayString<3>),

    /// Decoded pseudo range expressed in meters of signal propoagation.
    PseudoRange(ArrayString<3>),

    /// SSI: signal received strength estimated at the receiver level,
    /// in decibels.
    SSI(ArrayString<3>),
}

impl Default for Observable {
    /// Builds a default "L1C" [Observable] to describe
    /// the phase of the L1 signal.
    fn default() -> Self {
        Self::from_str("L1C").unwrap()
    }
}

impl Observable {
    /// Returns true if Self and rhs describe the same physical observation.
    /// For example, L1C and L2C would return true, because they are both
    /// carrier phase observations.
    pub fn same_physics(&self, rhs: &Observable) -> bool {
        match self {
            Self::SSI(_) => matches!(rhs, Self::SSI(_)),
            Self::PhaseRange(_) => matches!(rhs, Self::PhaseRange(_)),
            Self::Doppler(_) => matches!(rhs, Self::Doppler(_)),
            Self::PseudoRange(_) => matches!(rhs, Self::PseudoRange(_)),
        }
    }

    /// Returns true if this [Observable] is a Phase Range estimate
    pub fn is_phase_range_observable(&self) -> bool {
        matches!(self, Self::PhaseRange(_))
    }

    /// Returns true if this [Observable] is a decoded Pseudo Range
    pub fn is_pseudo_range_observable(&self) -> bool {
        matches!(self, Self::PseudoRange(_))
    }

    /// Returns true if this [Observable] is a doppler measurement
    pub fn is_doppler_observable(&self) -> bool {
        matches!(self, Self::Doppler(_))
    }

    /// Returns true if this [Observable] is an SSI measurement
    pub fn is_ssi_observable(&self) -> bool {
        matches!(self, Self::SSI(_))
    }

    /// Returns the 2 or 3 letter code, in RINEX standardized format
    pub fn code(&self) -> Option<String> {
        match self {
            Self::PhaseRange(c) | Self::Doppler(c) | Self::SSI(c) | Self::PseudoRange(c) => {
                if c.len() == 3 {
                    Some(c[1..].to_string())
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    /// Tries to convert into [Carrier] frequency.
    pub fn to_carrier(&self, c: Constellation) -> Result<Carrier, Error> {
        Carrier::from_observable(c, self)
    }

    /// Tries to create a Pseudo Range [Observable] from
    /// provided signal frequency in MHz and provided [Constellation].
    /// This requires a 1kHz accuracy on given frequency.
    pub fn from_pseudo_range_frequency_mega_hz(
        constellation: Constellation,
        frequency_mega_hz: f64,
    ) -> Result<Self, Error> {
        let carrier = Carrier::from_frequency_mega_hz(frequency_mega_hz)?;

        match constellation {
            Constellation::GPS => match carrier {
                Carrier::L1 => return Ok(Self::from_str("C1C").unwrap()),
                Carrier::L2 => return Ok(Self::from_str("C2C").unwrap()),
                Carrier::L5 => return Ok(Self::from_str("C5X").unwrap()),
                _ => return Err(Error::UnknownGPSFrequency),
            },
            Constellation::Galileo => match carrier {
                Carrier::L1 => return Ok(Self::from_str("C1C").unwrap()),
                Carrier::L5 => return Ok(Self::from_str("C5X").unwrap()),
                Carrier::E6 => return Ok(Self::from_str("C6X").unwrap()),
                Carrier::E5b => return Ok(Self::from_str("C7X").unwrap()),
                Carrier::E5a5b => return Ok(Self::from_str("C8X").unwrap()),
                _ => return Err(Error::UnknownGalileoFrequency),
            },
            Constellation::QZSS => match carrier {
                Carrier::L1 => return Ok(Self::from_str("C1C").unwrap()),
                Carrier::L2 => return Ok(Self::from_str("C2X").unwrap()),
                Carrier::L5 => return Ok(Self::from_str("C5X").unwrap()),
                Carrier::E6 => return Ok(Self::from_str("C6X").unwrap()),
                _ => return Err(Error::UnknownQzssFrequency),
            },
            Constellation::BeiDou => match carrier {
                Carrier::B1 => return Ok(Self::from_str("C2X").unwrap()),
                Carrier::L1 => return Ok(Self::from_str("C1X").unwrap()),
                Carrier::L5 => return Ok(Self::from_str("C5X").unwrap()),
                Carrier::E5b => return Ok(Self::from_str("C7X").unwrap()),
                Carrier::E5a5b => return Ok(Self::from_str("C8X").unwrap()),
                Carrier::B3 => return Ok(Self::from_str("C6X").unwrap()),
                _ => return Err(Error::UnknownBDSFrequency),
            },
            Constellation::Glonass => match carrier {
                Carrier::G1(_) => return Ok(Self::from_str("C1C").unwrap()),
                Carrier::G2(_) => return Ok(Self::from_str("C2X").unwrap()),
                Carrier::G3 => return Ok(Self::from_str("C3X").unwrap()),
                Carrier::G1a => return Ok(Self::from_str("C4X").unwrap()),
                Carrier::G2a => return Ok(Self::from_str("C6X").unwrap()),
                _ => return Err(Error::UnknownGlonassFrequency),
            },
            Constellation::IRNSS => match carrier {
                Carrier::L5 => return Ok(Self::from_str("C5X").unwrap()),
                Carrier::S => return Ok(Self::from_str("C9X").unwrap()),
                _ => return Err(Error::UnknownIRNSSFrequency),
            },
            _ => {},
        }

        if constellation.is_sbas() {
            match carrier {
                Carrier::L1 => Ok(Self::from_str("C1C").unwrap()),
                Carrier::L5 => Ok(Self::from_str("C5X").unwrap()),
                _ => Err(Error::UnknownSBASFrequency),
            }
        } else {
            Err(Error::UnknownFrequency)
        }
    }

    /// Tries to create a Phase Range [Observable] from
    /// provided signal frequency in MHz and provided [Constellation].
    /// This requires a 1kHz accuracy on given frequency.
    pub fn from_phase_range_frequency_mega_hz(
        constellation: Constellation,
        frequency_mega_hz: f64,
    ) -> Result<Self, Error> {
        let carrier = Carrier::from_frequency_mega_hz(frequency_mega_hz)?;

        match constellation {
            Constellation::GPS => match carrier {
                Carrier::L1 => return Ok(Self::from_str("L1C").unwrap()),
                Carrier::L2 => return Ok(Self::from_str("L2C").unwrap()),
                Carrier::L5 => return Ok(Self::from_str("L5X").unwrap()),
                _ => return Err(Error::UnknownGPSFrequency),
            },
            Constellation::Galileo => match carrier {
                Carrier::L1 => return Ok(Self::from_str("L1C").unwrap()),
                Carrier::L5 => return Ok(Self::from_str("L5X").unwrap()),
                Carrier::E6 => return Ok(Self::from_str("L6X").unwrap()),
                Carrier::E5b => return Ok(Self::from_str("L7X").unwrap()),
                Carrier::E5a5b => return Ok(Self::from_str("L8X").unwrap()),
                _ => return Err(Error::UnknownGalileoFrequency),
            },
            Constellation::QZSS => match carrier {
                Carrier::L1 => return Ok(Self::from_str("L1C").unwrap()),
                Carrier::L2 => return Ok(Self::from_str("L2X").unwrap()),
                Carrier::L5 => return Ok(Self::from_str("L5X").unwrap()),
                Carrier::E6 => return Ok(Self::from_str("L6X").unwrap()),
                _ => return Err(Error::UnknownQzssFrequency),
            },
            Constellation::BeiDou => match carrier {
                Carrier::B1 => return Ok(Self::from_str("L2X").unwrap()),
                Carrier::L1 => return Ok(Self::from_str("L1X").unwrap()),
                Carrier::L5 => return Ok(Self::from_str("L5X").unwrap()),
                Carrier::E5b => return Ok(Self::from_str("L7X").unwrap()),
                Carrier::E5a5b => return Ok(Self::from_str("L8X").unwrap()),
                Carrier::B3 => return Ok(Self::from_str("L6X").unwrap()),
                _ => return Err(Error::UnknownBDSFrequency),
            },
            Constellation::Glonass => match carrier {
                Carrier::G1(_) => return Ok(Self::from_str("L2C").unwrap()),
                Carrier::G2(_) => return Ok(Self::from_str("L2X").unwrap()),
                Carrier::G3 => return Ok(Self::from_str("L3X").unwrap()),
                Carrier::G1a => return Ok(Self::from_str("L4X").unwrap()),
                Carrier::G2a => return Ok(Self::from_str("L6X").unwrap()),
                _ => return Err(Error::UnknownGlonassFrequency),
            },
            Constellation::IRNSS => match carrier {
                Carrier::L5 => return Ok(Self::from_str("L5X").unwrap()),
                Carrier::S => return Ok(Self::from_str("L9X").unwrap()),
                _ => return Err(Error::UnknownIRNSSFrequency),
            },
            _ => {},
        }

        if constellation.is_sbas() {
            match carrier {
                Carrier::L1 => Ok(Self::from_str("L1C").unwrap()),
                Carrier::L5 => Ok(Self::from_str("L5X").unwrap()),
                _ => Err(Error::UnknownSBASFrequency),
            }
        } else {
            Err(Error::UnknownFrequency)
        }
    }

    /// Returns the period of provided code length (repetition period), expressed in seconds,
    /// of self: a valid Pseudo Range observable. This is not intended to be used
    /// on phase observables, although they are also determined from PRN codes.
    /// This is mostly used in fractional pseudo range determination.
    pub fn code_length(&self, c: Constellation) -> Option<f64> {
        match c {
            Constellation::GPS => {
                match self {
                    Self::PseudoRange(code) => {
                        match code.as_ref() {
                            "C1" => Some(20.0E-3_f64),
                            "C1C" => Some(1.0_f64), // TODO
                            "C1L" => Some(1.0_f64), // TODO
                            "C1X" => Some(1.0_f64), // TODO
                            "C1P" => Some(1.0_f64), // TODO,
                            "C1W" => Some(1.0_f64), // TODO
                            "C1Y" => Some(1.0_f64), // TODO
                            "C1M" => Some(1.0_f64), // TODO
                            "C2" => Some(1.0_f64),  //TODO
                            "C2D" => Some(1.0_f64), //TODO
                            "C2S" => Some(1.0_f64), //TODO
                            "C2L" => Some(1.0_f64), //TODO
                            "C2X" => Some(1.0_f64), //TODO
                            "C2P" => Some(1.0_f64), //TODO
                            "C2W" => Some(1.0_f64), //TODO
                            "C2Y" => Some(1.0_f64), //TODO
                            "C2M" => Some(1.0_f64), //TODO
                            _ => None,              // does not apply
                        }
                    },
                    _ => None, // invalid: not a pseudo range
                }
            },
            Constellation::QZSS => {
                match self {
                    Self::PseudoRange(code) => {
                        match code.as_ref() {
                            "C1" => Some(20.0E-3_f64),
                            "C1C" => Some(1.0_f64), // TODO
                            "C1L" => Some(1.0_f64), // TODO
                            "C1X" => Some(1.0_f64), // TODO
                            "C1P" => Some(1.0_f64), // TODO,
                            "C1W" => Some(1.0_f64), // TODO
                            "C1Y" => Some(1.0_f64), // TODO
                            "C1M" => Some(1.0_f64), // TODO
                            "C2" => Some(1.0_f64),  //TODO
                            "C2S" => Some(1.0_f64), //TODO
                            "C2L" => Some(1.0_f64), //TODO
                            "C2X" => Some(1.0_f64), //TODO
                            "C5" => Some(1.0_f64),  //TODO
                            "C5I" => Some(1.0_f64), //TODO
                            "C5P" => Some(1.0_f64), //TODO
                            "C5Q" => Some(1.0_f64), //TODO
                            "C5X" => Some(1.0_f64), //TODO
                            "C5Z" => Some(1.0_f64), //TODO
                            "C6" => Some(1.0_f64),  //TODO
                            "C6L" => Some(1.0_f64), //TODO
                            "C6X" => Some(1.0_f64), //TODO
                            "C6E" => Some(1.0_f64), //TODO
                            "C6S" => Some(1.0_f64), //TODO
                            _ => None,              // does not apply
                        }
                    },
                    _ => None, // invalid: not a pseudo range
                }
            },
            Constellation::BeiDou => {
                match self {
                    Self::PseudoRange(code) => {
                        match code.as_ref() {
                            "C1" => Some(1.0_f64),
                            "C2I" => Some(1.0_f64),
                            "C2X" => Some(1.0_f64),
                            "C1D" => Some(1.0_f64),
                            "C1P" => Some(1.0_f64),
                            "C1X" => Some(1.0_f64),
                            "C1S" => Some(1.0_f64),
                            "C1L" => Some(1.0_f64),
                            "C1Z" => Some(1.0_f64),
                            "C5D" => Some(1.0_f64),
                            "C5P" => Some(1.0_f64),
                            "C5X" => Some(1.0_f64),
                            "C2" => Some(1.0_f64),
                            "C7I" => Some(1.0_f64),
                            "C7Q" => Some(1.0_f64),
                            "C7X" => Some(1.0_f64),
                            "C7D" => Some(1.0_f64),
                            "C7P" => Some(1.0_f64),
                            "C7Z" => Some(1.0_f64),
                            "C8D" => Some(1.0_f64),
                            "C8P" => Some(1.0_f64),
                            "C8X" => Some(1.0_f64),
                            "C6I" => Some(1.0_f64),
                            "C6Q" => Some(1.0_f64),
                            "C6X" => Some(1.0_f64),
                            "C6D" => Some(1.0_f64),
                            "C6P" => Some(1.0_f64),
                            "C6Z" => Some(1.0_f64),
                            _ => None, // does not apply
                        }
                    },
                    _ => None, // invalid : not a pseudo range
                }
            },
            Constellation::Galileo => {
                match self {
                    Self::PseudoRange(code) => {
                        match code.as_ref() {
                            "C1" => Some(1.0_f64),  // TODO
                            "C1A" => Some(1.0_f64), // TODO
                            "C1B" => Some(1.0_f64), // TODO
                            "C1C" => Some(1.0_f64), // TODO
                            "C1X" => Some(1.0_f64), // TODO
                            "C1Z" => Some(1.0_f64), // TODO
                            "C5I" => Some(1.0_f64), // TODO
                            "C5Q" => Some(1.0_f64), // TODO
                            "C5X" => Some(1.0_f64), // TODO
                            "C7I" => Some(1.0_f64), // TODO
                            "C7Q" => Some(1.0_f64), // TODO
                            "C7X" => Some(1.0_f64), // TODO
                            "C5" => Some(1.0_f64),  // TODO
                            "C8I" => Some(1.0_f64), // TODO
                            "C8Q" => Some(1.0_f64), // TODO
                            "C8X" => Some(1.0_f64), // TODO
                            "C6" => Some(1.0_f64),  // TODO
                            "C6A" => Some(1.0_f64), // TODO
                            "C6B" => Some(1.0_f64), // TODO
                            "C6C" => Some(1.0_f64), // TODO
                            "C6X" => Some(1.0_f64), // TODO
                            "C6Z" => Some(1.0_f64), // TODO
                            _ => None,
                        }
                    },
                    _ => None, // invalid: not a pseudo range
                }
            },
            Constellation::SBAS => {
                match self {
                    Self::PseudoRange(code) => {
                        match code.as_ref() {
                            "C1" => Some(1.0_f64),  // TODO
                            "C1C" => Some(1.0_f64), // TODO
                            "C5" => Some(1.0_f64),  // TODO
                            "C5I" => Some(1.0_f64), // TODO
                            "C5Q" => Some(1.0_f64), // TODO
                            "C5X" => Some(1.0_f64), // TODO
                            _ => None,
                        }
                    },
                    _ => None, // invalid: not a pseudo range
                }
            },
            Constellation::Glonass => {
                match self {
                    Self::PseudoRange(code) => {
                        match code.as_ref() {
                            "C1" => Some(1.0_f64),  // TODO
                            "C1C" => Some(1.0_f64), // TODO
                            "C1P" => Some(1.0_f64), // TODO
                            "C4A" => Some(1.0_f64), // TODO
                            "C4C" => Some(1.0_f64), // TODO
                            "C5I" => Some(1.0_f64), // TODO
                            "C5Q" => Some(1.0_f64), // TODO
                            "C5X" => Some(1.0_f64), // TODO
                            _ => None,
                        }
                    },
                    _ => None, // invalid: not a pseudo range
                }
            },
            Constellation::IRNSS => {
                match self {
                    Self::PseudoRange(code) => {
                        match code.as_ref() {
                            "S" => Some(1.0_f64), //TODO
                            _ => None,            // invalid
                        }
                    },
                    _ => None, // invalid : not a pseudo range
                }
            },
            _ => None,
        }
    }
}

impl std::fmt::UpperHex for Observable {
    /// Formats this [Observable] in standardized 2 or 3 letter code,
    /// as found in RINEX records
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::PseudoRange(c) => write!(f, "{}", c),
            Self::PhaseRange(c) => write!(f, "{}", c),
            Self::Doppler(c) => write!(f, "{}", c),
            Self::SSI(c) => write!(f, "{}", c),
        }
    }
}

impl std::str::FromStr for Observable {
    type Err = ObsRINEXParsingError;

    /// Parses an [Observable] from a 2 (RINEX v2) or 3 (RINEX v3) character
    /// description, that must fit the standard specifications,
    /// including case sensitivity.
    ///
    /// For example:
    /// - "L1" may represent a valid sample of the code or phase
    /// of the L1 signal, in a RINEX v2 file.
    /// - "C1C" represents a valid decoded range using the L1 signal,
    /// in a RINEX v3 file.
    /// - "L1C" represents a valid sample of the phase
    /// of the L1 signal, in a RINEX v3 file.
    ///
    /// This methods identifieds both V2 and V3 observables correctly.
    fn from_str(content: &str) -> Result<Self, Self::Err> {
        let len = content.len();
        if len < 1 || len > 3 {
            return ObsRINEXParsingError::InvalidObservable;
        }

        if content.starts_with('L') {
            // phase-range
            Ok(Self::PhaseRange(
                ArrayString::<3>::from_str(content)
                    .map_err(|_| ObsRINEXParsingError::ObservableSizeError),
            ))
        } else if content.starts_with('C') {
            // pseudo-range
            Ok(Self::PseudoRange(
                ArrayString::<3>::from_str(content)
                    .map_err(|_| ObsRINEXParsingError::ObservableSizeError),
            ))
        } else if content.starts_with('P') {
            // legacy glonass pseudo-range
            Ok(Self::PseudoRange(
                ArrayString::<3>::from_str(content)
                    .map_err(|_| ObsRINEXParsingError::ObservableSizeError),
            ))
        } else if content.starts_with('S') {
            // SSI
            Ok(Self::SSI(
                ArrayString::<3>::from_str(content)
                    .map_err(|_| ObsRINEXParsingError::ObservableSizeError),
            ))
        } else {
            Err(ObsRINEXParsingError::IncorrectObservable)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;
    #[test]
    fn test_default_observable() {
        let default = Observable::default();
        assert_eq!(default, Observable::from_str("L1C").unwrap());
        assert_eq!(default, Observable::PhaseRange(String::from("L1C")));
        assert!(default.is_phase_range_observable());
    }
    #[test]
    fn test_physics() {
        assert!(Observable::from_str("L1")
            .unwrap()
            .is_phase_range_observable());
        assert!(Observable::from_str("L2")
            .unwrap()
            .is_phase_range_observable());
        assert!(Observable::from_str("L6X")
            .unwrap()
            .is_phase_range_observable());
        assert!(Observable::from_str("C1")
            .unwrap()
            .is_pseudo_range_observable());
        assert!(Observable::from_str("C2")
            .unwrap()
            .is_pseudo_range_observable());
        assert!(Observable::from_str("C6X")
            .unwrap()
            .is_pseudo_range_observable());
        assert!(Observable::from_str("D1").unwrap().is_doppler_observable());
        assert!(Observable::from_str("D2").unwrap().is_doppler_observable());
        assert!(Observable::from_str("D6X").unwrap().is_doppler_observable());
        assert!(Observable::from_str("S1").unwrap().is_ssi_observable());
        assert!(Observable::from_str("S2").unwrap().is_ssi_observable());
        assert!(Observable::from_str("S1P").unwrap().is_ssi_observable());
        assert!(Observable::from_str("S1W").unwrap().is_ssi_observable());
    }
    #[test]
    fn test_observable() {
        assert_eq!(Observable::from_str("PR").unwrap(), Observable::Pressure);
        assert_eq!(Observable::from_str("pr").unwrap(), Observable::Pressure);
        assert_eq!(Observable::from_str("PR").unwrap().to_string(), "PR");

        assert_eq!(Observable::from_str("WS").unwrap(), Observable::WindSpeed);
        assert_eq!(Observable::from_str("ws").unwrap(), Observable::WindSpeed);
        assert_eq!(Observable::from_str("WS").unwrap().to_string(), "WS");

        assert!(Observable::from_str("Err").is_err());
        assert!(Observable::from_str("TODO").is_err());

        assert_eq!(
            Observable::from_str("L1").unwrap(),
            Observable::PhaseRange(String::from("L1"))
        );

        assert!(Observable::from_str("L1").unwrap().code().is_none());

        assert_eq!(
            Observable::from_str("L2").unwrap(),
            Observable::PhaseRange(String::from("L2"))
        );

        assert_eq!(
            Observable::from_str("L5").unwrap(),
            Observable::PhaseRange(String::from("L5"))
        );
        assert_eq!(
            Observable::from_str("L6Q").unwrap(),
            Observable::PhaseRange(String::from("L6Q"))
        );
        assert_eq!(
            Observable::from_str("L6Q").unwrap().code(),
            Some(String::from("6Q")),
        );

        assert_eq!(
            Observable::from_str("L1C").unwrap(),
            Observable::PhaseRange(String::from("L1C"))
        );
        assert_eq!(
            Observable::from_str("L1P").unwrap(),
            Observable::PhaseRange(String::from("L1P"))
        );
        assert_eq!(
            Observable::from_str("L8X").unwrap(),
            Observable::PhaseRange(String::from("L8X"))
        );

        assert_eq!(
            Observable::from_str("L1P").unwrap(),
            Observable::PhaseRange(String::from("L1P"))
        );

        assert_eq!(
            Observable::from_str("L8X").unwrap(),
            Observable::PhaseRange(String::from("L8X"))
        );

        assert_eq!(
            Observable::from_str("S7Q").unwrap(),
            Observable::SSI(String::from("S7Q")),
        );

        assert_eq!(
            Observable::PseudoRange("S7Q".to_string()).to_string(),
            "S7Q",
        );

        assert_eq!(Observable::Doppler("D7Q".to_string()).to_string(), "D7Q",);

        assert_eq!(Observable::Doppler("C7X".to_string()).to_string(), "C7X",);
    }

    #[test]
    fn test_same_physics() {
        assert!(Observable::Temperature.same_physics(&Observable::Temperature));
        assert!(!Observable::Pressure.same_physics(&Observable::Temperature));

        let dop_l1 = Observable::Doppler("L1".to_string());
        let dop_l1c = Observable::Doppler("L1C".to_string());
        let dop_l2 = Observable::Doppler("L2".to_string());
        let dop_l2w = Observable::Doppler("L2W".to_string());

        let pr_l1 = Observable::PseudoRange("L1".to_string());
        let pr_l1c = Observable::PseudoRange("L1C".to_string());
        let pr_l2 = Observable::PseudoRange("L2".to_string());
        let pr_l2w = Observable::PseudoRange("L2W".to_string());

        assert!(dop_l1.same_physics(&dop_l1));
        assert!(dop_l1c.same_physics(&dop_l1));
        assert!(dop_l1c.same_physics(&dop_l2));
        assert!(dop_l1c.same_physics(&dop_l2w));
        assert!(!dop_l1.same_physics(&pr_l1));
        assert!(!dop_l1.same_physics(&pr_l1c));
        assert!(!dop_l1.same_physics(&pr_l2));
        assert!(!dop_l1.same_physics(&pr_l2w));

        assert!(pr_l1.same_physics(&pr_l1));
        assert!(pr_l1.same_physics(&pr_l1c));
        assert!(pr_l1.same_physics(&pr_l2));
        assert!(pr_l1.same_physics(&pr_l2w));
    }
}
