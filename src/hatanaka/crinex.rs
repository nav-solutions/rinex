//! CRINEX Header definitions
use crate::{
    epoch::now as epoch_now_utc,
    epoch::parse_formatted_month,
    prelude::{Epoch, FormattingError, ParsingError, Version},
};

use std::io::{BufWriter, Write};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// CRINEX specifications
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CRINEX {
    /// Compression program version
    pub version: Version,

    /// Compression program name
    pub prog: String,

    /// Date of compression
    pub date: Epoch,
}

impl CRINEX {
    /// Formats [CRINEX] into [BufWriter]
    pub fn format<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        writeln!(
            w,
            "{}                 COMPACT RINEX FORMAT                    CRINEX VERS   / TYPE",
            self.version,
        )?;

        let (y, m, d, hh, mm, _, _) = self.date.to_gregorian_utc();

        let formatted_month = match m {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            _ => "Dec",
        };

        write!(w, "{}", self.prog)?;
        write!(w, "{:<width$}", "", width = 40 - self.prog.len())?;

        writeln!(
            w,
            "{:02}-{}-{} {:02}:{:02}     CRINEX PROG / DATE",
            d,
            formatted_month,
            y - 2000,
            hh,
            mm,
        )?;

        Ok(())
    }

    /// Defines compression algorithm revision
    pub fn with_version(&self, version: Version) -> Self {
        let mut s = self.clone();
        s.version = version;
        s
    }

    /// Defines compression program name
    pub fn with_prog(&self, prog: &str) -> Self {
        let mut s = self.clone();
        s.prog = prog.to_string();
        s
    }

    /// Defines date of compression
    pub fn with_date(&self, e: Epoch) -> Self {
        let mut s = self.clone();
        s.date = e;
        s
    }

    /// Parse and append prog+date fields
    pub(crate) fn with_prog_date(&self, prog_date: &str) -> Result<Self, ParsingError> {
        if prog_date.len() < 60 {
            return Err(ParsingError::HeaderLineTooShort);
        }

        let (prog, rem) = prog_date.split_at(20);
        let (_, rem) = rem.split_at(20);
        let datetime_str = rem.split_at(20).0.trim();

        let mut i = 0;
        let mut year = 2000_i32; // CRINEX: Y2000
        let mut month = 0_u8;
        let mut day = 0_u8;
        let mut hour = 0_u8;
        let mut mins = 0_u8;

        for (index, datetime) in datetime_str.split_ascii_whitespace().enumerate() {
            match index {
                0 => {
                    for (index, parsed) in datetime.split('-').enumerate() {
                        let parsed = parsed.trim();
                        match index {
                            0 => {
                                day = parsed
                                    .parse::<u8>()
                                    .or(Err(ParsingError::DatetimeParsing))?;
                            },
                            1 => {
                                month = parse_formatted_month(parsed)?;
                            },
                            2 => {
                                year += parsed
                                    .parse::<i32>()
                                    .or(Err(ParsingError::DatetimeParsing))?;
                            },
                            _ => {},
                        }
                    }
                },
                1 => {
                    for (index, parsed) in datetime.split(':').enumerate() {
                        let parsed = parsed.trim();
                        match index {
                            0 => {
                                hour = parsed
                                    .parse::<u8>()
                                    .or(Err(ParsingError::DatetimeParsing))?;
                            },
                            1 => {
                                mins = parsed
                                    .parse::<u8>()
                                    .or(Err(ParsingError::DatetimeParsing))?;
                            },
                            _ => {},
                        }
                    }
                },
                _ => {},
            }

            i += 1;
        }

        if i != 2 {
            return Err(ParsingError::DatetimeFormat);
        }

        let epoch = Epoch::from_gregorian_utc(year, month, day, hour, mins, 0, 0);
        let s = self.with_prog(prog.trim());
        let s = s.with_date(epoch);
        Ok(s)
    }
}

impl Default for CRINEX {
    fn default() -> Self {
        Self {
            version: Version { major: 3, minor: 0 },
            prog: format!("nav-solutions v{}", env!("CARGO_PKG_VERSION")),
            date: epoch_now_utc(),
        }
    }
}

impl std::str::FromStr for CRINEX {
    type Err = ParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut crinex = Self::default();

        // We expect one line separator.
        // Content should follow standard odering (specs)
        for (index, line) in s.lines().enumerate() {
            if index == 0 {
                if line.len() < 60 {
                    return Err(ParsingError::HeaderLineTooShort);
                }
                let vers = line.split_at(20).0.trim();
                let version = Version::from_str(vers)?;
                crinex = crinex.with_version(version);
            } else {
                if line.len() < 60 {
                    return Err(ParsingError::HeaderLineTooShort);
                }
                crinex = crinex.with_prog_date(line)?;
            }
        }
        Ok(crinex)
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::{Epoch, Version, CRINEX};
    use std::str::FromStr;

    #[test]
    fn test_crinex_1() {
        let crinex = CRINEX {
            version: Version::new(3, 0),
            prog: "RNX2CRX ver.4.0.7".to_string(),
            date: Epoch::from_str("2021-01-02T00:01:00 UTC").unwrap(),
        };

        let content =
            "3.0                 COMPACT RINEX FORMAT                    CRINEX VERS   / TYPE
RNX2CRX ver.4.0.7                       02-Jan-21 00:01     CRINEX PROG / DATE";

        let decoded = CRINEX::from_str(&content).unwrap();
        assert_eq!(decoded, crinex);
    }

    #[test]
    fn test_crinex_2() {
        let crinex = CRINEX {
            version: Version::new(1, 0),
            prog: "test".to_string(),
            date: Epoch::from_str("2015-10-20T09:08:00 UTC").unwrap(),
        };

        let content =
            "1.0                 COMPACT RINEX FORMAT                    CRINEX VERS   / TYPE
test                                    20-Oct-15 09:08     CRINEX PROG / DATE";

        let decoded = CRINEX::from_str(&content).unwrap();
        assert_eq!(decoded, crinex);
    }

    #[test]
    fn test_with_prog_date() {
        let crinex = CRINEX::default();

        let crinex = crinex
            .with_prog_date(
                "RNX2CRX ver.4.0.7                       28-Dec-21 00:17     CRINEX PROG / DATE",
            )
            .unwrap();

        let _ = crinex
            .with_prog_date("RNX2CRX ver.4.0.7                       28-Dec-21 00:17     ")
            .unwrap();
    }
}
