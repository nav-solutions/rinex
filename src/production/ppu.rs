use super::Error;
use hifitime::{Duration, Unit, DAYS_PER_YEAR};

#[cfg(feature = "serde")]
use serde::Serialize;

/// PPU Gives information on file periodicity.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum PPU {
    /// A Daily file is the standard and contains 24h of data
    #[default]
    Daily,
    /// Contains 15' of data
    QuarterHour,
    /// Contains 1h of data
    Hourly,
    /// Contains 1 year of data
    Yearly,
    /// Unspecified
    Unspecified,
}

impl From<Duration> for PPU {
    fn from(dt: Duration) -> Self {
        if dt >= DAYS_PER_YEAR * Unit::Day {
            Self::Yearly
        } else if dt > 1.0 * Unit::Hour && dt <= 24.0 * Unit::Hour {
            Self::Daily
        } else if dt > 15.0 * Unit::Minute && dt <= 1.0 * Unit::Hour {
            Self::Hourly
        } else if dt > 5.0 * Unit::Minute && dt <= 15.0 * Unit::Minute {
            Self::QuarterHour
        } else {
            Self::Unspecified
        }
    }
}

impl PPU {
    /// Returns this file periodicity as a [Duration]
    pub fn duration(&self) -> Option<Duration> {
        match self {
            Self::QuarterHour => Some(15 * Unit::Minute),
            Self::Hourly => Some(1 * Unit::Hour),
            Self::Daily => Some(1 * Unit::Day),
            Self::Yearly => Some(DAYS_PER_YEAR * Unit::Day),
            _ => None,
        }
    }
}

impl std::fmt::Display for PPU {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::QuarterHour => write!(f, "15M"),
            Self::Hourly => write!(f, "01H"),
            Self::Daily => write!(f, "01D"),
            Self::Yearly => write!(f, "O1Y"),
            Self::Unspecified => write!(f, "00U"),
        }
    }
}

impl std::str::FromStr for PPU {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "15M" => Ok(Self::QuarterHour),
            "01H" => Ok(Self::Hourly),
            "01D" => Ok(Self::Daily),
            "01Y" => Ok(Self::Yearly),
            _ => Ok(Self::Unspecified),
        }
    }
}

#[cfg(test)]
mod test {
    use super::PPU;
    use hifitime::{Unit, DAYS_PER_YEAR};
    use std::str::FromStr;
    #[test]
    fn ppu_parsing() {
        for (c, expected, dur) in [
            ("15M", PPU::QuarterHour, Some(15 * Unit::Minute)),
            ("01H", PPU::Hourly, Some(1 * Unit::Hour)),
            ("01D", PPU::Daily, Some(1 * Unit::Day)),
            ("01Y", PPU::Yearly, Some(DAYS_PER_YEAR * Unit::Day)),
            ("XX", PPU::Unspecified, None),
            ("01U", PPU::Unspecified, None),
        ] {
            let ppu = PPU::from_str(c).unwrap();
            assert_eq!(ppu, expected);
            assert_eq!(ppu.duration(), dur, "test failed for {}", c);
        }
    }
    #[test]
    fn ppu_cast() {
        for (duration, expected) in [
            (15.0 * Unit::Minute, PPU::QuarterHour),
            (0.9 * Unit::Hour, PPU::Hourly),
            (1.0 * Unit::Hour, PPU::Hourly),
            (0.9 * Unit::Day, PPU::Daily),
            (1.0 * Unit::Day, PPU::Daily),
        ] {
            assert_eq!(PPU::from(duration), expected);
        }
    }
}
