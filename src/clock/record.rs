use thiserror::Error;

use std::{collections::BTreeMap, str::FromStr};

use crate::{
    epoch::parse_in_timescale as parse_epoch_in_timescale,
    parse_f64,
    prelude::{Epoch, ParsingError, TimeScale, Version, SV},
};

#[cfg(feature = "processing")]
use qc_traits::{DecimationFilter, DecimationFilterType, FilterItem, MaskFilter, MaskOperand};

/// [`ClockKey`] describes each [`ClockProfile`] at a specific [Epoch].
#[derive(Error, PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ClockKey {
    /// Type of Clock
    pub clock_type: ClockType,
    /// Type of attached measurement
    pub profile_type: ClockProfileType,
}

impl std::fmt::Display for ClockKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "clock-type: {}", self.clock_type)?;
        write!(f, "profile-type: {}", self.profile_type)
    }
}

/// Type of clock we're dealing with.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ClockType {
    /// SV Clock (on board)
    SV(SV),
    /// Ground station Clock
    Station(String),
}

impl Default for ClockType {
    fn default() -> Self {
        Self::Station(String::from("Unknown"))
    }
}

impl ClockType {
    /// Unwraps self as a `satellite vehicle`
    pub fn as_sv(&self) -> Option<SV> {
        match self {
            Self::SV(s) => Some(*s),
            _ => None,
        }
    }
    /// Unwraps self as a `station` identification code
    pub fn as_station(&self) -> Option<String> {
        match self {
            Self::Station(s) => Some(s.clone()),
            _ => None,
        }
    }
}

impl std::fmt::Display for ClockType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(sv) = self.as_sv() {
            f.write_str(&sv.to_string())?
        } else if let Some(station) = self.as_station() {
            f.write_str(&station)?
        }
        Ok(())
    }
}

/// Clock Profile is the actual measurement or estimate
/// at a specified Epoch.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct ClockProfile {
    /// Clock bias [s]
    pub bias: f64,
    /// Clock bias deviation
    pub bias_dev: Option<f64>,
    /// Clock drift [s/s]
    pub drift: Option<f64>,
    /// Clock drift deviation
    pub drift_dev: Option<f64>,
    /// Clock drift change [s/s^2]
    pub drift_change: Option<f64>,
    /// Clock drift change deviation
    pub drift_change_dev: Option<f64>,
}

/// Clock data observables
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ClockProfileType {
    /// Data analysis results for receiver clocks
    /// derived from a set of network receivers and satellites
    AR,
    /// Data analysis results for satellites clocks
    /// derived from a set of network receivers and satellites
    AS,
    /// Calibration measurements for a single GNSS receiver
    CR,
    /// Discontinuity measurements for a single GNSS receiver
    DR,
    /// Broadcast SV clocks monitor measurements
    MS,
}

impl std::str::FromStr for ClockProfileType {
    type Err = ParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "ar" => Ok(Self::AR),
            "as" => Ok(Self::AS),
            "cr" => Ok(Self::CR),
            "dr" => Ok(Self::DR),
            "ms" => Ok(Self::MS),
            _ => Err(ParsingError::ClockProfileType),
        }
    }
}

impl std::fmt::Display for ClockProfileType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::AR => f.write_str("AR"),
            Self::AS => f.write_str("AS"),
            Self::CR => f.write_str("CR"),
            Self::DR => f.write_str("DR"),
            Self::MS => f.write_str("MS"),
        }
    }
}

/// Clock RINEX record content.
pub type Record = BTreeMap<Epoch, BTreeMap<ClockKey, ClockProfile>>;

pub(crate) fn is_new_epoch(line: &str) -> bool {
    // first 2 bytes match a ClockProfileType code
    if line.len() < 3 {
        false
    } else {
        let content = &line[..2];
        ClockProfileType::from_str(content).is_ok()
    }
}

/// Builds `RINEX` record entry for `Clocks` data files.   
/// Returns identified `epoch` to sort data efficiently.  
/// Returns 2D data as described in `record` definition
pub(crate) fn parse_epoch(
    version: Version,
    content: &str,
    ts: TimeScale,
) -> Result<(Epoch, ClockKey, ClockProfile), ParsingError> {
    let mut lines = content.lines();
    let line = lines.next().ok_or(ParsingError::EmptyEpoch)?;

    const LIMIT: Version = Version { major: 3, minor: 4 };
    let (dtype, mut rem) = line.split_at(3);
    let profile_type = ClockProfileType::from_str(dtype.trim())?;

    let clock_type = match version < LIMIT {
        true => {
            // old revision
            let (system_str, r) = rem.split_at(5);
            rem = r;
            if let Ok(svnn) = SV::from_str(system_str.trim()) {
                ClockType::SV(svnn)
            } else {
                ClockType::Station(system_str.trim().to_string())
            }
        },
        false => {
            // modern revision
            let (system_str, r) = rem.split_at(4);
            if let Ok(svnn) = SV::from_str(system_str.trim()) {
                let (_, r) = r.split_at(6);
                rem = r;
                ClockType::SV(svnn)
            } else {
                let mut content = system_str.to_owned();
                let (remainder, r) = r.split_at(6);
                rem = r;
                content.push_str(remainder);
                ClockType::Station(content.trim().to_string())
            }
        },
    };

    // Epoch: Y on 4 digits, even on RINEX2
    const OFFSET: usize = "yyyy mm dd hh mm sssssssssss".len();

    let (epoch, rem) = rem.split_at(OFFSET);
    let epoch = parse_epoch_in_timescale(epoch.trim(), ts)?;

    // nb of data fields
    let (_n, rem) = rem.split_at(4);

    // data fields
    let mut profile = ClockProfile::default();

    for (index, item) in rem.split_ascii_whitespace().enumerate() {
        match index {
            0 => {
                profile.bias = parse_f64(item.trim())
                    .map_err(|_| ParsingError::ClockProfile)?;
            },
            1 => {
                profile.bias_dev = Some(
                    parse_f64(item.trim())
                        .map_err(|_| ParsingError::ClockProfile)?,
                );
            },
            _ => {},
        }
    }
    for line in lines {
        for (index, item) in line.split_ascii_whitespace().enumerate() {
            match index {
                0 => {
                    profile.drift = Some(
                        parse_f64(item.trim())
                            .map_err(|_| ParsingError::ClockProfile)?,
                    );
                },
                1 => {
                    profile.drift_dev = Some(
                        parse_f64(item.trim())
                            .map_err(|_| ParsingError::ClockProfile)?,
                    );
                },
                2 => {
                    profile.drift_change = Some(
                        parse_f64(item.trim())
                            .map_err(|_| ParsingError::ClockProfile)?,
                    );
                },
                3 => {
                    profile.drift_change_dev = Some(
                        parse_f64(item.trim())
                            .map_err(|_| ParsingError::ClockProfile)?,
                    );
                },
                _ => {},
            }
        }
    }
    Ok((
        epoch,
        ClockKey {
            clock_type,
            profile_type,
        },
        profile,
    ))
}

/// Writes epoch into stream
pub(crate) fn fmt_epoch(epoch: &Epoch, key: &ClockKey, prof: &ClockProfile) -> String {
    let mut lines = String::with_capacity(60);
    let (y, m, d, hh, mm, ss, _) = epoch.to_gregorian_utc();

    let mut n = 1;
    if prof.drift.is_some() {
        n += 1;
    }
    if prof.drift_dev.is_some() {
        n += 1;
    }
    if prof.drift_change.is_some() {
        n += 1;
    }
    if prof.drift_change_dev.is_some() {
        n += 1;
    }

    lines.push_str(&format!(
        "{} {}  {} {:02} {:02} {:02} {:02} {:02}.000000  {}   {:.12E}",
        key.profile_type, key.clock_type, y, m, d, hh, mm, ss, n, prof.bias
    ));

    if let Some(sigma) = prof.bias_dev {
        lines.push_str(&format!("{:.13E} ", sigma));
    }
    lines.push('\n');
    if let Some(drift) = prof.drift {
        lines.push_str(&format!("   {:.13E} ", drift));
        if let Some(sigma) = prof.drift_dev {
            lines.push_str(&format!("{:.13E} ", sigma));
        }
        if let Some(drift_change) = prof.drift_change {
            lines.push_str(&format!("{:.13E} ", drift_change));
        }
        if let Some(sigma) = prof.drift_change_dev {
            lines.push_str(&format!("{:.13E} ", sigma));
        }
        lines.push('\n');
    }
    lines
}

#[cfg(feature = "processing")]
pub(crate) fn clock_mask_mut(rec: &mut Record, mask: &MaskFilter) {
    match mask.operand {
        MaskOperand::Equals => match &mask.item {
            FilterItem::EpochItem(epoch) => rec.retain(|e, _| *e == *epoch),
            FilterItem::ConstellationItem(mask) => {
                rec.retain(|_, data| {
                    data.retain(|sysclk, _| {
                        if let Some(sv) = sysclk.clock_type.as_sv() {
                            mask.contains(&sv.constellation)
                        } else {
                            false
                        }
                    });
                    !data.is_empty()
                });
            },
            _ => {}, // FilterItem::
        },
        MaskOperand::NotEquals => match &mask.item {
            FilterItem::EpochItem(epoch) => rec.retain(|e, _| *e != *epoch),
            _ => {}, // FilterItem::
        },
        MaskOperand::GreaterEquals => match &mask.item {
            FilterItem::EpochItem(epoch) => rec.retain(|e, _| *e >= *epoch),
            _ => {}, // FilterItem::
        },
        MaskOperand::GreaterThan => match &mask.item {
            FilterItem::EpochItem(epoch) => rec.retain(|e, _| *e > *epoch),
            _ => {}, // FilterItem::
        },
        MaskOperand::LowerEquals => match &mask.item {
            FilterItem::EpochItem(epoch) => rec.retain(|e, _| *e <= *epoch),
            _ => {}, // FilterItem::
        },
        MaskOperand::LowerThan => match &mask.item {
            FilterItem::EpochItem(epoch) => rec.retain(|e, _| *e < *epoch),
            _ => {}, // FilterItem::
        },
    }
}

#[cfg(feature = "processing")]
pub(crate) fn clock_decim_mut(rec: &mut Record, f: &DecimationFilter) {
    if f.item.is_some() {
        todo!("targetted decimation not supported yet");
    }
    match f.filter {
        DecimationFilterType::Modulo(r) => {
            let mut i = 0;
            rec.retain(|_, _| {
                let retained = (i % r) == 0;
                i += 1;
                retained
            });
        },
        DecimationFilterType::Duration(interval) => {
            let mut last_retained = Option::<Epoch>::None;
            rec.retain(|e, _| {
                if let Some(last) = last_retained {
                    let dt = *e - last;
                    if dt >= interval {
                        last_retained = Some(*e);
                        true
                    } else {
                        false
                    }
                } else {
                    last_retained = Some(*e);
                    true // always retain 1st epoch
                }
            });
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::prelude::SV;
    use crate::version::Version;
    use std::str::FromStr;
    #[test]
    fn test_is_new_epoch() {
        let c = "AR AREQ 1994 07 14 20 59  0.000000  6   -0.123456789012E+00 -0.123456789012E+01";
        assert!(is_new_epoch(c));
        let c = "RA AREQ 1994 07 14 20 59  0.000000  6   -0.123456789012E+00 -0.123456789012E+01";
        assert!(!is_new_epoch(c));
        let c = "DR AREQ 1994 07 14 20 59  0.000000  6   -0.123456789012E+00 -0.123456789012E+01";
        assert!(is_new_epoch(c));
        let c = "CR AREQ 1994 07 14 20 59  0.000000  6   -0.123456789012E+00 -0.123456789012E+01";
        assert!(is_new_epoch(c));
        let c = "AS AREQ 1994 07 14 20 59  0.000000  6   -0.123456789012E+00 -0.123456789012E+01";
        assert!(is_new_epoch(c));
        let c = "CR USNO 1995 07 14 20 59 50.000000  2    0.123456789012E+00  -0.123456789012E-01";
        assert!(is_new_epoch(c));
        let c = "AS G16  1994 07 14 20 59  0.000000  2   -0.123456789012E+00 -0.123456789012E+01";
        assert!(is_new_epoch(c));
        let c = "A  G16  1994 07 14 20 59  0.000000  2   -0.123456789012E+00 -0.123456789012E+01";
        assert!(!is_new_epoch(c));
        let c = "z";
        assert!(!is_new_epoch(c));
    }
    #[test]
    fn parse_clk_v2_epoch() {
        for (descriptor, epoch, key, profile) in [
            (
                "AS R20  2019 01 08 00 03 30.000000  1   -0.364887538519E-03",
                Epoch::from_str("2019-01-08T00:03:30 GPST").unwrap(),
                ClockKey {
                    clock_type: ClockType::SV(SV::from_str("R20").unwrap()),
                    profile_type: ClockProfileType::AS,
                },
                ClockProfile {
                    bias: -0.364887538519E-03,
                    bias_dev: None,
                    drift: None,
                    drift_change: None,
                    drift_dev: None,
                    drift_change_dev: None,
                },
            ),
            (
                "AS R18  2019 01 08 10 00  0.000000  2    0.294804625338E-04  0.835484069663E-11",
                Epoch::from_str("2019-01-08T10:00:00 GPST").unwrap(),
                ClockKey {
                    clock_type: ClockType::SV(SV::from_str("R18").unwrap()),
                    profile_type: ClockProfileType::AS,
                },
                ClockProfile {
                    bias: 0.294804625338E-04,
                    bias_dev: Some(0.835484069663E-11),
                    drift: None,
                    drift_dev: None,
                    drift_change: None,
                    drift_change_dev: None,
                },
            ),
            (
                "AR PIE1 2019 01 08 00 04  0.000000  1   -0.434275035628E-03",
                Epoch::from_str("2019-01-08T00:04:00 GPST").unwrap(),
                ClockKey {
                    clock_type: ClockType::Station("PIE1".to_string()),
                    profile_type: ClockProfileType::AR,
                },
                ClockProfile {
                    bias: -0.434275035628E-03,
                    bias_dev: None,
                    drift: None,
                    drift_dev: None,
                    drift_change: None,
                    drift_change_dev: None,
                },
            ),
            (
                "AR IMPZ 2019 01 08 00 00  0.000000  2   -0.331415119107E-07  0.350626190546E-10",
                Epoch::from_str("2019-01-08T00:00:00 GPST").unwrap(),
                ClockKey {
                    clock_type: ClockType::Station("IMPZ".to_string()),
                    profile_type: ClockProfileType::AR,
                },
                ClockProfile {
                    bias: -0.331415119107E-07,
                    bias_dev: Some(0.350626190546E-10),
                    drift: None,
                    drift_dev: None,
                    drift_change: None,
                    drift_change_dev: None,
                },
            ),
        ] {
            let (parsed_e, parsed_k, parsed_prof) =
                parse_epoch(Version { minor: 0, major: 2 }, descriptor, TimeScale::GPST)
                    .unwrap_or_else(|_| panic!("failed to parse \"{}\"", descriptor));

            assert_eq!(parsed_e, epoch, "parsed wrong epoch");
            assert_eq!(parsed_k, key, "parsed wrong clock id");
            assert_eq!(parsed_prof, profile, "parsed wrong clock data");
        }
    }
    #[test]
    fn parse_clk_v3_epoch() {
        for (descriptor, epoch, key, profile) in [
            (
                "AR AREQ 1994 07 14 20 59  0.000000  6   -0.123456789012E+00 -0.123456789012E+01
    -0.123456789012E+02 -0.123456789012E+03 -0.123456789012E+04 -0.123456789012E+05",
                Epoch::from_str("1994-07-14T20:59:00 GPST").unwrap(),
                ClockKey {
                    clock_type: ClockType::Station("AREQ".to_string()),
                    profile_type: ClockProfileType::AR,
                },
                ClockProfile {
                    bias: -0.123456789012E+00,
                    bias_dev: Some(-0.123456789012E+01),
                    drift: Some(-0.123456789012E+02),
                    drift_dev: Some(-0.123456789012E+03),
                    drift_change: Some(-0.123456789012E+04),
                    drift_change_dev: Some(-0.123456789012E+05),
                },
            ),
            (
                "AS G16  1994 07 14 20 59  0.000000  2   -0.123456789012E+00 -0.123456789012E+01",
                Epoch::from_str("1994-07-14T20:59:00 GPST").unwrap(),
                ClockKey {
                    clock_type: ClockType::SV(SV::from_str("G16").unwrap()),
                    profile_type: ClockProfileType::AS,
                },
                ClockProfile {
                    bias: -0.123456789012E+00,
                    bias_dev: Some(-0.123456789012E+01),
                    drift: None,
                    drift_dev: None,
                    drift_change: None,
                    drift_change_dev: None,
                },
            ),
            (
                "CR USNO 1994 07 14 20 59  0.000000  2   -0.123456789012E+00 -0.123456789012E+01",
                Epoch::from_str("1994-07-14T20:59:00 GPST").unwrap(),
                ClockKey {
                    clock_type: ClockType::Station("USNO".to_string()),
                    profile_type: ClockProfileType::CR,
                },
                ClockProfile {
                    bias: -0.123456789012E+00,
                    bias_dev: Some(-0.123456789012E+01),
                    drift: None,
                    drift_dev: None,
                    drift_change: None,
                    drift_change_dev: None,
                },
            ),
            (
                "DR USNO 1994 07 14 20 59  0.000000  2   -0.123456789012E+00 -0.123456789012E+01
    -0.123456789012E-03 -0.123456789012E-04",
                Epoch::from_str("1994-07-14T20:59:00 GPST").unwrap(),
                ClockKey {
                    clock_type: ClockType::Station("USNO".to_string()),
                    profile_type: ClockProfileType::DR,
                },
                ClockProfile {
                    bias: -0.123456789012E+00,
                    bias_dev: Some(-0.123456789012E+01),
                    drift: Some(-0.123456789012E-03),
                    drift_dev: Some(-0.123456789012E-04),
                    drift_change: None,
                    drift_change_dev: None,
                },
            ),
        ] {
            let (parsed_e, parsed_k, parsed_prof) =
                parse_epoch(Version { minor: 0, major: 2 }, descriptor, TimeScale::GPST)
                    .unwrap_or_else(|_| panic!("failed to parse \"{}\"", descriptor));

            assert_eq!(parsed_e, epoch, "parsed wrong epoch");
            assert_eq!(parsed_k, key, "parsed wrong clock id");
            assert_eq!(parsed_prof, profile, "parsed wrong clock data");
        }
    }
}
