//! Meteo sensor observables
use crate::prelude::{Error, ParsingError};

/// The observables describe all possible types of measurements
/// by meteo sensors, as found in Meteo RINEX files.
/// Observation (signal measurements) RINEX files have their own set
/// of observables.
#[derive(Debug, Default, Copy, Clone, PartialEq, PartialOrd, Hash, Ord, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MeteoObservable {
    /// Dry temperature estimate, in Celcius degrees.
    #[default]
    Temperature,

    /// Pressure estimate, in hPa.
    Pressure,

    /// Relative humidity rate estimate, in percent.
    HumidityRate,

    /// Wet Zenith Path delay in millimeters of signal
    /// propagation delay (frequency dependent).
    ZenithWetDelay,

    /// Zenith path delay, dry component in millimeters of signal
    /// propagation delay (frequency dependent).
    ZenithDryDelay,

    /// Total zenith path delay (dry + wet) in millimeters of
    /// signal propagation delay (frequency dependent).
    ZenithTotalDelay,

    /// Wind direction azimuth estimate, in degrees.
    WindDirection,

    /// Wind speed measurement, in m.s⁻¹.
    WindSpeed,

    /// Rain Increment: rain accumulation since previous
    /// observation, in 10th of millimeters.
    RainIncrement,

    /// Hail Indicator: a boolean number asserted when hail is detected
    /// by the sensor.
    HailIndicator,
}

impl MeteoObservable {
    /// Returns true if Self and rhs describe the same physical observation.
    /// For example, both are phase observations.
    pub fn same_physics(&self, rhs: &MeteoObservable) -> bool {
        match self {
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
}

impl std::fmt::Display for MeteoObservable {
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

impl std::str::FromStr for MeteoObservable {
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
                        Err(ParsingError::UnknownMeteoObservable)
                    }
                } else {
                    Err(ParsingError::BadMeteoObservable)
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
        let default = MeteoObservable::default();
        assert_eq!(default, MeteoObservable::from_str("L1C").unwrap());
        assert_eq!(default, MeteoObservable::PhaseRange(String::from("L1C")));
        assert!(default.is_phase_range_observable());
    }
    #[test]
    fn test_physics() {
        assert!(MeteoObservable::from_str("L1")
            .unwrap()
            .is_phase_range_observable());
        assert!(MeteoObservable::from_str("L2")
            .unwrap()
            .is_phase_range_observable());
        assert!(MeteoObservable::from_str("L6X")
            .unwrap()
            .is_phase_range_observable());
        assert!(MeteoObservable::from_str("C1")
            .unwrap()
            .is_pseudo_range_observable());
        assert!(MeteoObservable::from_str("C2")
            .unwrap()
            .is_pseudo_range_observable());
        assert!(MeteoObservable::from_str("C6X")
            .unwrap()
            .is_pseudo_range_observable());
        assert!(MeteoObservable::from_str("D1")
            .unwrap()
            .is_doppler_observable());
        assert!(MeteoObservable::from_str("D2")
            .unwrap()
            .is_doppler_observable());
        assert!(MeteoObservable::from_str("D6X")
            .unwrap()
            .is_doppler_observable());
        assert!(MeteoObservable::from_str("S1").unwrap().is_ssi_observable());
        assert!(MeteoObservable::from_str("S2").unwrap().is_ssi_observable());
        assert!(MeteoObservable::from_str("S1P")
            .unwrap()
            .is_ssi_observable());
        assert!(MeteoObservable::from_str("S1W")
            .unwrap()
            .is_ssi_observable());
    }
    #[test]
    fn test_observable() {
        assert_eq!(
            MeteoObservable::from_str("PR").unwrap(),
            MeteoObservable::Pressure
        );
        assert_eq!(
            MeteoObservable::from_str("pr").unwrap(),
            MeteoObservable::Pressure
        );
        assert_eq!(MeteoObservable::from_str("PR").unwrap().to_string(), "PR");

        assert_eq!(
            MeteoObservable::from_str("WS").unwrap(),
            MeteoObservable::WindSpeed
        );
        assert_eq!(
            MeteoObservable::from_str("ws").unwrap(),
            MeteoObservable::WindSpeed
        );
        assert_eq!(MeteoObservable::from_str("WS").unwrap().to_string(), "WS");

        assert!(MeteoObservable::from_str("Err").is_err());
        assert!(MeteoObservable::from_str("TODO").is_err());

        assert_eq!(
            MeteoObservable::from_str("L1").unwrap(),
            MeteoObservable::PhaseRange(String::from("L1"))
        );

        assert!(MeteoObservable::from_str("L1").unwrap().code().is_none());

        assert_eq!(
            MeteoObservable::from_str("L2").unwrap(),
            MeteoObservable::PhaseRange(String::from("L2"))
        );

        assert_eq!(
            MeteoObservable::from_str("L5").unwrap(),
            MeteoObservable::PhaseRange(String::from("L5"))
        );
        assert_eq!(
            MeteoObservable::from_str("L6Q").unwrap(),
            MeteoObservable::PhaseRange(String::from("L6Q"))
        );
        assert_eq!(
            MeteoObservable::from_str("L6Q").unwrap().code(),
            Some(String::from("6Q")),
        );

        assert_eq!(
            MeteoObservable::from_str("L1C").unwrap(),
            MeteoObservable::PhaseRange(String::from("L1C"))
        );
        assert_eq!(
            MeteoObservable::from_str("L1P").unwrap(),
            MeteoObservable::PhaseRange(String::from("L1P"))
        );
        assert_eq!(
            MeteoObservable::from_str("L8X").unwrap(),
            MeteoObservable::PhaseRange(String::from("L8X"))
        );

        assert_eq!(
            MeteoObservable::from_str("L1P").unwrap(),
            MeteoObservable::PhaseRange(String::from("L1P"))
        );

        assert_eq!(
            MeteoObservable::from_str("L8X").unwrap(),
            MeteoObservable::PhaseRange(String::from("L8X"))
        );

        assert_eq!(
            MeteoObservable::from_str("S7Q").unwrap(),
            MeteoObservable::SSI(String::from("S7Q")),
        );

        assert_eq!(
            MeteoObservable::PseudoRange("S7Q".to_string()).to_string(),
            "S7Q",
        );

        assert_eq!(
            MeteoObservable::Doppler("D7Q".to_string()).to_string(),
            "D7Q",
        );

        assert_eq!(
            MeteoObservable::Doppler("C7X".to_string()).to_string(),
            "C7X",
        );
    }

    #[test]
    fn test_same_physics() {
        assert!(MeteoObservable::Temperature.same_physics(&MeteoObservable::Temperature));
        assert!(!MeteoObservable::Pressure.same_physics(&MeteoObservable::Temperature));

        let dop_l1 = MeteoObservable::Doppler("L1".to_string());
        let dop_l1c = MeteoObservable::Doppler("L1C".to_string());
        let dop_l2 = MeteoObservable::Doppler("L2".to_string());
        let dop_l2w = MeteoObservable::Doppler("L2W".to_string());

        let pr_l1 = MeteoObservable::PseudoRange("L1".to_string());
        let pr_l1c = MeteoObservable::PseudoRange("L1C".to_string());
        let pr_l2 = MeteoObservable::PseudoRange("L2".to_string());
        let pr_l2w = MeteoObservable::PseudoRange("L2W".to_string());

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
