use super::Error;
use hifitime::{Duration, Unit, SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MINUTE};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FFU {
    /// Sample rate
    pub val: u32,
    /// Period unit
    pub unit: Unit,
}

impl From<Duration> for FFU {
    fn from(dt: Duration) -> Self {
        let total_seconds = dt.to_seconds();
        if dt < SECONDS_PER_MINUTE * Unit::Second {
            Self {
                val: total_seconds.round() as u32,
                unit: Unit::Second,
            }
        } else if dt < 1.0 * Unit::Hour {
            Self {
                val: (total_seconds / SECONDS_PER_MINUTE).round() as u32,
                unit: Unit::Minute,
            }
        } else if dt < 1 * Unit::Day {
            Self {
                val: (total_seconds / SECONDS_PER_HOUR).round() as u32,
                unit: Unit::Hour,
            }
        } else {
            Self {
                val: (total_seconds / SECONDS_PER_DAY).round() as u32,
                unit: Unit::Day,
            }
        }
    }
}

impl Default for FFU {
    fn default() -> Self {
        Self {
            val: 30,
            unit: Unit::Second,
        }
    }
}

impl std::fmt::Display for FFU {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.unit {
            Unit::Minute => write!(f, "{:02}M", self.val),
            Unit::Hour => write!(f, "{:02}H", self.val),
            Unit::Day => write!(f, "{:02}D", self.val),
            Unit::Second | _ => write!(f, "{:02}S", self.val),
        }
    }
}

impl std::str::FromStr for FFU {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 3 {
            return Err(Error::InvalidFFU);
        }
        let val = s[..2].parse::<u32>().map_err(|_| Error::InvalidFFU)?;
        let unit = match s.chars().nth(2) {
            Some('S') => Unit::Second,
            Some('M') => Unit::Minute,
            Some('H') => Unit::Hour,
            Some('D') => Unit::Day,
            _ => return Err(Error::InvalidFFU),
        };
        Ok(Self { val, unit })
    }
}

#[cfg(test)]
mod test {
    use super::FFU;
    use hifitime::{Duration, Unit, SECONDS_PER_DAY};
    use std::str::FromStr;
    #[test]
    fn ffu_parsing() {
        for (desc, expected) in [
            (
                "30S",
                FFU {
                    val: 30,
                    unit: Unit::Second,
                },
            ),
            (
                "01M",
                FFU {
                    val: 1,
                    unit: Unit::Minute,
                },
            ),
            (
                "15M",
                FFU {
                    val: 15,
                    unit: Unit::Minute,
                },
            ),
            (
                "30M",
                FFU {
                    val: 30,
                    unit: Unit::Minute,
                },
            ),
            (
                "01H",
                FFU {
                    val: 1,
                    unit: Unit::Hour,
                },
            ),
            (
                "04H",
                FFU {
                    val: 4,
                    unit: Unit::Hour,
                },
            ),
            (
                "08H",
                FFU {
                    val: 8,
                    unit: Unit::Hour,
                },
            ),
            (
                "01D",
                FFU {
                    val: 1,
                    unit: Unit::Day,
                },
            ),
            (
                "07D",
                FFU {
                    val: 7,
                    unit: Unit::Day,
                },
            ),
        ] {
            let ffu = FFU::from_str(desc).unwrap();
            assert_eq!(ffu, expected);
        }
    }
    #[test]
    fn ffu_cast() {
        for (duration, expected) in [
            (
                Duration::from_seconds(30.0),
                FFU {
                    val: 30,
                    unit: Unit::Second,
                },
            ),
            (
                Duration::from_seconds(60.0),
                FFU {
                    val: 1,
                    unit: Unit::Minute,
                },
            ),
            (
                Duration::from_seconds(3600.0),
                FFU {
                    val: 1,
                    unit: Unit::Hour,
                },
            ),
            (
                Duration::from_seconds(SECONDS_PER_DAY),
                FFU {
                    val: 1,
                    unit: Unit::Day,
                },
            ),
        ] {
            assert_eq!(FFU::from(duration), expected);
        }
    }
}
