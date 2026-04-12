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
        // two formats exist
        let mut ls = Leap::default();
        let items = s.split_ascii_whitespace().collect::<Vec<_>>();
        let count = items.len();

        if count < 3 {
            // Simple / basic format
            ls.leap = items[0].parse::<u32>().or(Err(ParsingError::LeapParsing))?;
        } else {
            // Complex / complete format
            let (leap, rem) = s.split_at(5);
            let (tls, rem) = rem.split_at(5);
            let (week, rem) = rem.split_at(5);
            let (day, rem) = rem.split_at(5);
            let system = rem.trim();

            ls.leap = leap
                .trim()
                .parse::<u32>()
                .or(Err(ParsingError::LeapParsing))?;

            let tls = tls
                .trim()
                .parse::<u32>()
                .or(Err(ParsingError::LeapParsing))?;

            ls.delta_tls = Some(tls);

            let week = week
                .trim()
                .parse::<u32>()
                .or(Err(ParsingError::LeapParsing))?;

            ls.week = Some(week);

            let day = day
                .trim()
                .parse::<u32>()
                .or(Err(ParsingError::LeapParsing))?;

            ls.day = Some(day);

            if system.eq("") {
                ls.timescale = None;
            } else {
                let ts = TimeScale::from_str(system)?;
                ls.timescale = Some(ts);
            }
        }

        Ok(ls)
    }
}

#[cfg(test)]
mod test {

    use crate::{
        prelude::{Leap, TimeScale},
        tests::formatting::Utf8Buffer,
    };

    use std::{io::BufWriter, str::FromStr};

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
