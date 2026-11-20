//! Leap second counter
use crate::{
    prelude::{ParsingError, TimeScale},
    FormattingError,
};

use std::io::{BufWriter, Write};

/// `Leap` to describe leap seconds.
/// GLO = UTC = GPS - ΔtLS   
/// GPS = UTC + ΔtLS   
#[derive(Default, Copy, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Leap {
    /// Counter at the time of formatting
    pub leap: u32,
    /// ΔtLS : "future or past leap second(s)",
    /// actual number of leap seconds between GPS/GAL and GLO,
    /// or BDS and UTC.
    pub delta_tls: Option<u32>,
    /// Week counter
    pub week: Option<u32>,
    /// Days counter
    pub day: Option<u32>,
    /// Timescale definition
    pub timescale: Option<TimeScale>,
}

impl Leap {
    // Format [Leap] into [BufWriter]
    pub fn format<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        if let Some(delta) = &self.delta_tls {
            write!(
                w,
                "{:6}{:6}{:6}{:6} ",
                self.leap,
                delta,
                self.week.unwrap_or(0),
                self.day.unwrap_or(0),
            )?;
        } else {
            write!(w, "{:6}                   ", self.leap,)?;
        }

        if let Some(ts) = self.timescale {
            write!(w, "{:x}", ts)?;
        } else {
            write!(w, "   ")?;
        }

        writeln!(w, "                                LEAP SECONDS")?;

        Ok(())
    }

    // Copy and assign Delta [TLS]
    pub fn with_delta_tls(&self, delta_tls: u32) -> Self {
        let mut s = self.clone();
        s.delta_tls = Some(delta_tls);
        s
    }
    // Copy and assign Week counter
    pub fn with_week(&self, week: u32) -> Self {
        let mut s = self.clone();
        s.week = Some(week);
        s
    }
    // Copy and assign Day counter
    pub fn with_day(&self, day: u32) -> Self {
        let mut s = self.clone();
        s.day = Some(day);
        s
    }
    // Copy and assign Timescale
    pub fn with_timescale(&self, ts: TimeScale) -> Self {
        let mut s = self.clone();
        s.timescale = Some(ts);
        s
    }
}

impl std::str::FromStr for Leap {
    type Err = ParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // RINEX 3/4 uses four I6 fields and a time-system identifier.
        // Optional blank fields stay unknown; byte positions must not be trimmed.
        if !s.is_ascii() {
            return Err(ParsingError::LeapParsing);
        }
        if s.len() >= 24 {
            let parse = |field: &str| -> Result<Option<u32>, ParsingError> {
                let field = field.trim();
                if field.is_empty() {
                    Ok(None)
                } else {
                    field
                        .parse()
                        .map(Some)
                        .map_err(|_| ParsingError::LeapParsing)
                }
            };
            let system = s[24..].trim();
            return Ok(Leap {
                leap: parse(&s[..6])?.ok_or(ParsingError::LeapParsing)?,
                delta_tls: parse(&s[6..12])?,
                week: parse(&s[12..18])?,
                day: parse(&s[18..24])?,
                timescale: if system.is_empty() {
                    None
                } else if system == "BDS" {
                    Some(TimeScale::BDT)
                } else {
                    Some(TimeScale::from_str(system)?)
                },
            });
        }

        // Keep upstream's compact inputs, including a system touching the last
        // number ("7GPS"). Without I6 columns, incomplete future fields cannot be
        // assigned reliably, so retain only the current leap count.
        let s = s.trim();
        let numbers = s.trim_end_matches(|c: char| c.is_ascii_alphabetic());
        let system = s[numbers.len()..].trim();
        let items = numbers
            .split_ascii_whitespace()
            .map(|item| item.parse::<u32>().map_err(|_| ParsingError::LeapParsing))
            .collect::<Result<Vec<_>, _>>()?;
        let mut ls = Leap {
            leap: *items.first().ok_or(ParsingError::LeapParsing)?,
            ..Leap::default()
        };
        if items.len() > 4 {
            return Err(ParsingError::LeapParsing);
        }
        if items.len() == 4 {
            ls.delta_tls = Some(items[1]);
            ls.week = Some(items[2]);
            ls.day = Some(items[3]);
        }
        if !system.is_empty() {
            ls.timescale = Some(TimeScale::from_str(system)?);
        }
        Ok(ls)
    }
}

#[cfg(test)]
mod test {

    use crate::{
        prelude::{Header, Leap, TimeScale},
        tests::formatting::Utf8Buffer,
    };

    use std::{
        io::{BufReader, BufWriter},
        str::FromStr,
    };

    #[test]
    fn fixed_width_partial_fields_keep_raw_values_and_unknown_day() {
        // GRN0 2024-05-10 real header content (first40 bytes). Physical meaning
        // of its unusual future fields is not certified by syntax parsing.
        let leap = Leap::from_str("    18  2313     5                       ").unwrap();
        assert_eq!(leap.leap, 18);
        assert_eq!(leap.delta_tls, Some(2313));
        assert_eq!(leap.week, Some(5));
        assert_eq!(leap.day, None);
        assert_eq!(leap.timescale, None);
    }

    #[test]
    fn fixed_width_blanks_and_explicit_time_system_are_not_shifted() {
        let leap = Leap::from_str("    18                  GPS             ").unwrap();
        assert_eq!(leap.leap, 18);
        assert_eq!(leap.delta_tls, None);
        assert_eq!(leap.week, None);
        assert_eq!(leap.day, None);
        assert_eq!(leap.timescale, Some(TimeScale::GPST));
        for (system, timescale) in [
            ("", None),
            ("GPS", Some(TimeScale::GPST)),
            ("BDS", Some(TimeScale::BDT)),
        ] {
            let complete =
                Leap::from_str(&format!("{:6}{:6}{:6}{:6}{system}", 18, 18, 2185, 7)).unwrap();
            assert_eq!(complete.delta_tls, Some(18));
            assert_eq!(complete.day, Some(7));
            assert_eq!(complete.week, Some(2185));
            assert_eq!(complete.timescale, timescale);
        }
    }

    #[test]
    fn malformed_or_empty_fields_return_error_without_panicking() {
        for input in [
            "",
            " ",
            "18 x",
            "18 18 2185 7 99",
            "                        ",
            "    18    xx                            ",
            "    18                  é",
            "é",
            "    xx                                  ",
        ] {
            assert!(Leap::from_str(input).is_err(), "{input:?}");
        }
    }

    #[test]
    fn leap_second_basic() {
        assert_eq!(
            Leap::from_str("18").unwrap(),
            Leap {
                leap: 18,
                week: None,
                day: None,
                timescale: None,
                delta_tls: None,
            }
        );
    }
    #[test]
    fn leap_second_standard() {
        assert_eq!(
            Leap::from_str("18    18  2185     7").unwrap(),
            Leap {
                leap: 18,
                week: Some(2185),
                day: Some(7),
                timescale: None,
                delta_tls: Some(18),
            }
        );
    }

    #[test]
    fn leap_second_with_timescale() {
        assert_eq!(
            Leap::from_str("18    18  2185     7GPS").unwrap(),
            Leap {
                leap: 18,
                week: Some(2185),
                day: Some(7),
                delta_tls: Some(18),
                timescale: Some(TimeScale::GPST),
            }
        );
    }

    #[test]
    fn leap_second_missing_future_field() {
        // REDZ00POL_S_20242240000_01D_30S_MO.rnx header: "    18  2326     7"
        assert_eq!(
            Leap::from_str("18  2326     7").unwrap(),
            Leap {
                leap: 18,
                week: None,
                day: None,
                timescale: None,
                delta_tls: None,
            }
        );
    }

    #[test]
    fn leap_second_basic_with_timescale() {
        assert_eq!(
            Leap::from_str("18                  GPS").unwrap(),
            Leap {
                leap: 18,
                week: None,
                day: None,
                timescale: Some(TimeScale::GPST),
                delta_tls: None,
            }
        );
    }

    #[test]
    fn leap_second_single_digit() {
        assert_eq!(
            Leap::from_str("5     5   500     7").unwrap(),
            Leap {
                leap: 5,
                week: Some(500),
                day: Some(7),
                timescale: None,
                delta_tls: Some(5),
            }
        );
    }

    #[test]
    fn leap_second_malformed_is_an_error() {
        for s in ["", "   ", "GPS", "18 x 2185 7", "18    18  2185     7XYZ"] {
            assert!(Leap::from_str(s).is_err(), "{:?}", s);
        }
    }

    #[test]
    fn fixed_width_missing_delta_keeps_later_fields() {
        // Synthetic I6 boundary: a blank delta must not shift week or day.
        let input = format!("{:6}{:6}{:6}{:6}GPS", 18, "", 2185, 7);
        let leap = Leap::from_str(&input).unwrap();
        assert_eq!(leap.leap, 18);
        assert_eq!(leap.delta_tls, None);
        assert_eq!(leap.week, Some(2185));
        assert_eq!(leap.day, Some(7));
        assert_eq!(leap.timescale, Some(TimeScale::GPST));
    }

    #[test]
    fn header_parser_preserves_leap_columns() {
        // Real GRN0 2024-05-10 leap content in a minimal synthetic OBS header.
        // Also check the time-system position with all future fields blank.
        for (content, expected) in [
            (
                "    18  2313     5                       ",
                Leap {
                    leap: 18,
                    delta_tls: Some(2313),
                    week: Some(5),
                    ..Leap::default()
                },
            ),
            (
                "    18                  GPS             ",
                Leap {
                    leap: 18,
                    timescale: Some(TimeScale::GPST),
                    ..Leap::default()
                },
            ),
        ] {
            let text = format!(
                "     3.04           OBSERVATION DATA    G                   RINEX VERSION / TYPE\n\
                 {content:<60}LEAP SECONDS\n\
                 {:60}END OF HEADER\n",
                ""
            );
            let header = Header::parse(&mut BufReader::new(text.as_bytes())).unwrap();
            assert_eq!(header.leap, Some(expected), "{content:?}");
        }
    }

    #[test]
    fn leap_formatting() {
        let leap = Leap::from_str("18").unwrap();
        let mut buf = BufWriter::new(Utf8Buffer::new(256));
        leap.format(&mut buf).unwrap();

        let ascii = buf.into_inner().unwrap().to_ascii_utf8();

        assert_eq!(
            &ascii,
            "    18                                                      LEAP SECONDS\n"
        );

        let leap = Leap::from_str("18    18  2185     7").unwrap();
        let mut buf = BufWriter::new(Utf8Buffer::new(256));
        leap.format(&mut buf).unwrap();

        let ascii = buf.into_inner().unwrap().to_ascii_utf8();

        assert_eq!(
            &ascii,
            "    18    18  2185     7                                    LEAP SECONDS\n"
        );
    }
}
