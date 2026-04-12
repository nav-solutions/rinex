//! RINEX Clock files parser & analysis
pub mod record;

pub use record::{ClockKey, ClockProfile, ClockProfileType, ClockType, Record};

use std::{
    io::{BufWriter, Write},
    str::FromStr,
};

use crate::{
    fmt_rinex, parse_f64,
    prelude::{DOMES, FormattingError, TimeScale, Version},
};

/// Clocks `RINEX` specific header fields
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HeaderFields {
    /// Site name
    pub site: Option<String>,
    /// Site DOMES ID#
    pub domes: Option<DOMES>,
    /// IGS code
    pub igs: Option<String>,
    /// Full name
    pub full_name: Option<String>,
    /// Station reference clock
    pub ref_clock: Option<String>,
    /// Timescale is either a GNSS timescale or UTC / TAI.
    /// Timescale is omitted in SBAS or COMPASS files.
    pub timescale: Option<TimeScale>,
    /// Reference clocks used in measurement / analysis process
    pub work_clock: Vec<WorkClock>,
    /// Types of clock profiles encountered in this file
    pub codes: Vec<ClockProfileType>,
}

/// Clock used in the analysis and evaluation of this file
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WorkClock {
    /// Name of this local clock
    pub name: String,
    /// Clock site DOMES ID#
    pub domes: Option<DOMES>,
    /// Possible clock constraint [s]
    pub constraint: Option<f64>,
}

impl WorkClock {
    pub(crate) fn parse(version: Version, content: &str) -> Self {
        const LIMIT: Version = Version { major: 3, minor: 4 };
        if version < LIMIT {
            let (name, rem) = content.split_at(4);
            let (domes, rem) = rem.split_at(36);
            let constraint = rem.split_at(20).0;
            Self {
                name: name.trim().to_string(),
                domes: if let Ok(domes) = DOMES::from_str(domes.trim()) {
                    Some(domes)
                } else {
                    None
                },
                constraint: if let Ok(value) = parse_f64(constraint.trim()) {
                    Some(value)
                } else {
                    None
                },
            }
        } else {
            let (name, rem) = content.split_at(10);
            let (domes, rem) = rem.split_at(10);
            let constraint = rem.split_at(40).0;
            Self {
                name: name.trim().to_string(),
                domes: if let Ok(domes) = DOMES::from_str(domes.trim()) {
                    Some(domes)
                } else {
                    None
                },
                constraint: if let Ok(value) = parse_f64(constraint.trim()) {
                    Some(value)
                } else {
                    None
                },
            }
        }
    }
}

impl HeaderFields {
    /// Formats [HeaderFields] into [BufWriter].
    pub(crate) fn format<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        write!(w, "{:6}", self.codes.len())?;

        for (nth, code) in self.codes.iter().enumerate() {
            write!(w, "{:6}", code,)?;

            if nth % 9 == 8 {
                writeln!(w, "# / TYPES OF DATA\n      ")?;
            }
        }

        // possible timescale
        if let Some(ts) = self.timescale {
            writeln!(
                w,
                "{}",
                fmt_rinex(&format!("   {:x}", ts), "TIME SYSTEM ID")
            )?;
        }

        Ok(())
    }

    /// Defines a new [WorkClock]
    pub(crate) fn work_clock(&self, clk: WorkClock) -> Self {
        let mut s = self.clone();
        s.work_clock.push(clk);
        s
    }

    /// Defines [TimeScale]
    pub(crate) fn timescale(&self, ts: TimeScale) -> Self {
        let mut s = self.clone();
        s.timescale = Some(ts);
        s
    }

    /// Defines clock site
    pub(crate) fn site(&self, site: &str) -> Self {
        let mut s = self.clone();
        s.site = Some(site.to_string());
        s
    }

    /// Defines [DOMES] site number
    pub(crate) fn domes(&self, domes: DOMES) -> Self {
        let mut s = self.clone();
        s.domes = Some(domes);
        s
    }

    /// Defines IGS special code
    pub(crate) fn igs(&self, igs: &str) -> Self {
        let mut s = self.clone();
        s.igs = Some(igs.to_string());
        s
    }

    /// Defines clock site full name
    pub(crate) fn full_name(&self, name: &str) -> Self {
        let mut s = self.clone();
        s.full_name = Some(name.to_string());
        s
    }

    /// Defines site reference clock
    pub(crate) fn refclock(&self, clk: &str) -> Self {
        let mut s = self.clone();
        s.ref_clock = Some(clk.to_string());
        s
    }
}
