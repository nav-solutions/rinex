use crate::prelude::{Carrier, Constellation, Error, ParsingError};

/// Observable describes all possible observations,
/// forming Observation and Meteo RINEX epoch content.
#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Ord, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Observable {
    /// Carrier phase range converted to [m] (not cycles!)
    PhaseRange(String),
    /// Doppler shift observation
    Doppler(String),
    /// SSI: Receiver signal strength observation [dB]
    SSI(String),
    /// Received Power [dBm]
    Power(String),
    /// Decoded Pseudo range converted to [m]
    PseudoRange(String),
    /// Channel number Pseudo Observable.
    /// Attached to Phase or PseudoRange observable to accurately
    /// described how they were sampled.
    ChannelNumber(String),
    /// Pressure observation in hPa
    Pressure,
    /// Dry temperature measurement in Celcius degrees
    Temperature,
    /// Relative humidity measurement in %
    HumidityRate,
    /// Wet Zenith Path delay in mm
    ZenithWetDelay,
    /// Zenith path delay, dry component in mm
    ZenithDryDelay,
    /// Total zenith path delay (dry + wet) in mm
    ZenithTotalDelay,
    /// Wind direction azimuth in degrees
    WindDirection,
    /// Wind speed in m.s⁻¹
    WindSpeed,
    /// Rain Increment: rain accumulation
    /// since previous measurement, in 10th of mm
    RainIncrement,
    /// Hail Indicator
    HailIndicator,
    /// Frequency Ratio (dimensionless)
    FrequencyRatio,
}

impl Default for Observable {
    fn default() -> Self {
        Self::PhaseRange("L1C".to_string())
    }
}

impl Observable {
    /// Returns true if Self and rhs describe the same physical observation.
    /// For example, both are phase observations.
    pub fn same_physics(&self, rhs: &Observable) -> bool {
        match self {
            Self::SSI(_) => matches!(rhs, Self::SSI(_)),
            Self::PhaseRange(_) => matches!(rhs, Self::PhaseRange(_)),
            Self::Power(_) => matches!(rhs, Self::Power(_)),
            Self::Doppler(_) => matches!(rhs, Self::Doppler(_)),
            Self::PseudoRange(_) => matches!(rhs, Self::PseudoRange(_)),
            Self::ChannelNumber(_) => matches!(rhs, Self::ChannelNumber(_)),
            Self::Pressure => matches!(rhs, Self::Pressure),
            Self::Temperature => matches!(rhs, Self::Temperature),
            Self::HumidityRate => matches!(rhs, Self::HumidityRate),
            Self::ZenithWetDelay => matches!(rhs, Self::ZenithWetDelay),
            Self::ZenithDryDelay => matches!(rhs, Self::ZenithDryDelay),
            Self::ZenithTotalDelay => matches!(rhs, Self::ZenithTotalDelay),
            Self::WindSpeed => matches!(rhs, Self::WindSpeed),
            Self::WindDirection => matches!(rhs, Self::WindDirection),
            Self::RainIncrement => matches!(rhs, Self::RainIncrement),
            Self::HailIndicator => matches!(rhs, Self::RainIncrement),
            Self::FrequencyRatio => matches!(rhs, Self::FrequencyRatio),
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

    pub fn is_power_observable(&self) -> bool {
        matches!(self, Self::Power(_))
    }

    /// Returns true if this [Observable] is a channel number (usually for Glonass FDMA)
    pub fn is_channel_number(&self) -> bool {
        matches!(self, Self::ChannelNumber(_))
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
        freq_mhz: f64,
    ) -> Result<Self, Error> {
        let carrier = Carrier::from_frequency_mega_hz(freq_mhz)?;

        match constellation {
            Constellation::GPS => match carrier {
                Carrier::L1 => return Ok(Self::PseudoRange("C1C".to_string())),
                Carrier::L2 => return Ok(Self::PseudoRange("C2C".to_string())),
                Carrier::L5 => return Ok(Self::PseudoRange("C5X".to_string())),
                _ => return Err(Error::UnknownGPSFrequency),
            },
            Constellation::Galileo => match carrier {
                Carrier::L1 => return Ok(Self::PseudoRange("C1C".to_string())),
                Carrier::L5 => return Ok(Self::PseudoRange("C5X".to_string())),
                Carrier::E6 => return Ok(Self::PseudoRange("C6X".to_string())),
                Carrier::E5b => return Ok(Self::PseudoRange("C7X".to_string())),
                Carrier::E5a5b => return Ok(Self::PseudoRange("C8X".to_string())),
                _ => return Err(Error::UnknownGalileoFrequency),
            },
            Constellation::QZSS => match carrier {
                Carrier::L1 => return Ok(Self::PseudoRange("C1C".to_string())),
                Carrier::L2 => return Ok(Self::PseudoRange("C2X".to_string())),
                Carrier::L5 => return Ok(Self::PseudoRange("C5X".to_string())),
                Carrier::E6 => return Ok(Self::PseudoRange("C6X".to_string())),
                _ => return Err(Error::UnknownQzssFrequency),
            },
            Constellation::BeiDou => match carrier {
                Carrier::B1 => return Ok(Self::PseudoRange("C2X".to_string())),
                Carrier::L1 => return Ok(Self::PseudoRange("C1X".to_string())),
                Carrier::L5 => return Ok(Self::PseudoRange("C5X".to_string())),
                Carrier::E5b => return Ok(Self::PseudoRange("C7X".to_string())),
                Carrier::E5a5b => return Ok(Self::PseudoRange("C8X".to_string())),
                Carrier::B3 => return Ok(Self::PseudoRange("C6X".to_string())),
                _ => return Err(Error::UnknownBDSFrequency),
            },
            Constellation::Glonass => match carrier {
                Carrier::G1(_) => return Ok(Self::PseudoRange("C1C".to_string())),
                Carrier::G2(_) => return Ok(Self::PseudoRange("C2X".to_string())),
                Carrier::G3 => return Ok(Self::PseudoRange("C3X".to_string())),
                Carrier::G1a => return Ok(Self::PseudoRange("C4X".to_string())),
                Carrier::G2a => return Ok(Self::PseudoRange("C6X".to_string())),
                _ => return Err(Error::UnknownGlonassFrequency),
            },
            Constellation::IRNSS => match carrier {
                Carrier::L5 => return Ok(Self::PseudoRange("C5X".to_string())),
                Carrier::S => return Ok(Self::PseudoRange("C9X".to_string())),
                _ => return Err(Error::UnknownIRNSSFrequency),
            },
            _ => {},
        }

        if constellation.is_sbas() {
            match carrier {
                Carrier::L1 => Ok(Self::PseudoRange("C1C".to_string())),
                Carrier::L5 => Ok(Self::PseudoRange("C5X".to_string())),
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
        freq_mhz: f64,
    ) -> Result<Self, Error> {
        let carrier = Carrier::from_frequency_mega_hz(freq_mhz)?;

        match constellation {
            Constellation::GPS => match carrier {
                Carrier::L1 => return Ok(Self::PhaseRange("L1C".to_string())),
                Carrier::L2 => return Ok(Self::PhaseRange("L2C".to_string())),
                Carrier::L5 => return Ok(Self::PhaseRange("L5X".to_string())),
                _ => return Err(Error::UnknownGPSFrequency),
            },
            Constellation::Galileo => match carrier {
                Carrier::L1 => return Ok(Self::PhaseRange("L1C".to_string())),
                Carrier::L5 => return Ok(Self::PhaseRange("L5X".to_string())),
                Carrier::E6 => return Ok(Self::PhaseRange("L6X".to_string())),
                Carrier::E5b => return Ok(Self::PhaseRange("L7X".to_string())),
                Carrier::E5a5b => return Ok(Self::PhaseRange("L8X".to_string())),
                _ => return Err(Error::UnknownGalileoFrequency),
            },
            Constellation::QZSS => match carrier {
                Carrier::L1 => return Ok(Self::PhaseRange("L1C".to_string())),
                Carrier::L2 => return Ok(Self::PhaseRange("L2X".to_string())),
                Carrier::L5 => return Ok(Self::PhaseRange("L5X".to_string())),
                Carrier::E6 => return Ok(Self::PhaseRange("L6X".to_string())),
                _ => return Err(Error::UnknownQzssFrequency),
            },
            Constellation::BeiDou => match carrier {
                Carrier::B1 => return Ok(Self::PhaseRange("L2X".to_string())),
                Carrier::L1 => return Ok(Self::PhaseRange("L1X".to_string())),
                Carrier::L5 => return Ok(Self::PhaseRange("L5X".to_string())),
                Carrier::E5b => return Ok(Self::PhaseRange("L7X".to_string())),
                Carrier::E5a5b => return Ok(Self::PhaseRange("L8X".to_string())),
                Carrier::B3 => return Ok(Self::PhaseRange("L6X".to_string())),
                _ => return Err(Error::UnknownBDSFrequency),
            },
            Constellation::Glonass => match carrier {
                Carrier::G1(_) => return Ok(Self::PhaseRange("L1C".to_string())),
                Carrier::G2(_) => return Ok(Self::PhaseRange("L2X".to_string())),
                Carrier::G3 => return Ok(Self::PhaseRange("L3X".to_string())),
                Carrier::G1a => return Ok(Self::PhaseRange("L4X".to_string())),
                Carrier::G2a => return Ok(Self::PhaseRange("L6X".to_string())),
                _ => return Err(Error::UnknownGlonassFrequency),
            },
            Constellation::IRNSS => match carrier {
                Carrier::L5 => return Ok(Self::PhaseRange("L5X".to_string())),
                Carrier::S => return Ok(Self::PhaseRange("L9X".to_string())),
                _ => return Err(Error::UnknownIRNSSFrequency),
            },
            _ => {},
        }

        if constellation.is_sbas() {
            match carrier {
                Carrier::L1 => Ok(Self::PhaseRange("L1C".to_string())),
                Carrier::L5 => Ok(Self::PhaseRange("L5X".to_string())),
                _ => Err(Error::UnknownSBASFrequency),
            }
        } else {
            Err(Error::UnknownFrequency)
        }
    }

    /// Returns the code length (repetition period), expressed in seconds,
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

    /// Returns true if this is the L1 pivot Ph or Code signal observation,
    /// used in signal combinations
    pub(crate) fn is_l1_pivot(&self, constellation: Constellation) -> bool {
        if self.is_phase_range_observable() || self.is_pseudo_range_observable() {
            if let Ok(carrier) = self.to_carrier(constellation) {
                matches!(
                    carrier,
                    Carrier::L1
                        | Carrier::E1
                        | Carrier::G1(_)
                        | Carrier::S1
                        | Carrier::B1c
                        | Carrier::B1a
                )
            } else {
                false
            }
        } else {
            false
        }
    }
}

impl std::fmt::Display for Observable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Pressure => write!(f, "PR"),
            Self::Temperature => write!(f, "TD"),
            Self::HumidityRate => write!(f, "HR"),
            Self::ZenithWetDelay => write!(f, "ZW"),
            Self::ZenithDryDelay => write!(f, "ZD"),
            Self::ZenithTotalDelay => write!(f, "ZT"),
            Self::WindDirection => write!(f, "WD"),
            Self::WindSpeed => write!(f, "WS"),
            Self::RainIncrement => write!(f, "RI"),
            Self::HailIndicator => write!(f, "HI"),
            Self::FrequencyRatio => write!(f, "F"),
            Self::PseudoRange(c)
            | Self::PhaseRange(c)
            | Self::Doppler(c)
            | Self::SSI(c)
            | Self::Power(c)
            | Self::ChannelNumber(c) => write!(f, "{}", c),
        }
    }
}

impl std::str::FromStr for Observable {
    type Err = ParsingError;
    fn from_str(content: &str) -> Result<Self, Self::Err> {
        let content = content.to_uppercase();
        let content = content.trim();
        match content {
            "P" | "PR" => Ok(Self::Pressure),
            "T" | "TD" => Ok(Self::Temperature),
            "H" | "HR" => Ok(Self::HumidityRate),
            "F" => Ok(Self::FrequencyRatio),
            "ZW" => Ok(Self::ZenithWetDelay),
            "ZD" => Ok(Self::ZenithDryDelay),
            "ZT" => Ok(Self::ZenithTotalDelay),
            "WD" => Ok(Self::WindDirection),
            "WS" => Ok(Self::WindSpeed),
            "RI" => Ok(Self::RainIncrement),
            "HI" => Ok(Self::HailIndicator),
            _ => {
                let len = content.len();
                if len > 1 && len < 4 {
                    if content.starts_with('L') {
                        Ok(Self::PhaseRange(content.to_string()))
                    } else if content.starts_with('C') || content.starts_with('P') {
                        Ok(Self::PseudoRange(content.to_string()))
                    } else if content.starts_with('S') {
                        Ok(Self::SSI(content.to_string()))
                    } else if content.starts_with('W') {
                        Ok(Self::Power(content.to_string()))
                    } else if content.starts_with('D') {
                        Ok(Self::Doppler(content.to_string()))
                    } else {
                        Err(ParsingError::UnknownObservable)
                    }
                } else {
                    Err(ParsingError::BadObservable)
                }
            },
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
