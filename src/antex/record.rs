use std::{collections::HashMap, str::FromStr};

use crate::{
    antex::{Antenna, AntennaSpecific, Calibration, CalibrationMethod, RxAntenna, SvAntenna},
    linspace::Linspace,
    parse_f64,
    prelude::{Carrier, Epoch, ParsingError, COSPAR, SV},
};

#[cfg(feature = "serde")]
use serde::Serialize;

/// Returns true if this line matches
/// the beginning of a `epoch` for ATX file (special files),
/// this is not really an epoch but rather a group of dataset
/// for this given antenna, there is no sampling data attached to it.
pub(crate) fn is_new_epoch(content: &str) -> bool {
    content.contains("START OF ANTENNA")
}

/// Phase pattern description.
/// We currently do not support azimuth dependent phase patterns.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum AntennaPhasePattern {
    /// Azimuth Independent Phase pattern
    AzimuthIndependentPattern(Vec<f64>),
}

impl Default for AntennaPhasePattern {
    fn default() -> Self {
        Self::AzimuthIndependentPattern(Vec::<f64>::new())
    }
}

#[derive(Debug, Default, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct FrequencyDependentData {
    /// Eccentricities of the mean APC as NEU coordinates in millimeters.
    /// The offset position is either relative to
    /// Antenna Reference point (ARP), if this is an [`RxAntenna`],
    /// or the Spacecraft Mass Center, if this is an [`SvAntenna`].
    pub apc_eccentricity: (f64, f64, f64),
    /// Antenna Phase Pattern.
    /// We currently do not support Azimuth Dependent phase patterns.
    pub phase_pattern: AntennaPhasePattern,
}

/// ANTEX RINEX record content.
/// Data is a list of Antenna containing several [Frequency] items.
/// We do not parse RMS frequencies at the moment, but it will
/// easily be unlocked in near future.
/*TODO
/// Record browsing example:
/// ```
/// // grab ATX RINEX
///
/// // grab record
/// let record = record.as_antex()
///    .unwrap();
/// // browse antennas
/// for (antenna, frequencies) in record.iter() {
///   let calibration = antenna.calibration;
///   // several calibration methods exist
///   if calibration.method == CalibrationMethod::Chamber {
///     // calibration is certified
///     // from `calibration.valid_from` (chrono::NaiveDateTime)
///     // until `calibration.valid_until` (chrono::NaiveDateTime)
///   }
///   // calibration process informations
///   assert_eq!(calibration.agency, "Some agency");
///   assert_eq!(calibration.date, "DateTime description");
///   // antenna information
///   assert_eq!(antenna.sn, "Serial Number");
///   assert_eq!(antenna.dazi, 1.0);
///   for frequency in frequencies.iter() {
///     for pattern in frequency.patterns {
///         assert_eq!(pattern.is_azimuth_dependent(), true);
///         let Some((azimuth, phase_pattern)) = pattern.azimuth_dependent() {
///             for raw_phase in phase_patter.iter() {
///                 // raw phase pattern data
///             }
///         }
///     }
///   }
/// }
/// ```
*/
pub type Record = Vec<(Antenna, HashMap<Carrier, FrequencyDependentData>)>;

fn parse_datetime(content: &str) -> Result<Epoch, ParsingError> {
    let mut parser = content.split('-');

    let year = parser.next().ok_or(ParsingError::DatetimeFormat)?;

    let year = year
        .parse::<i32>()
        .map_err(|_| ParsingError::DatetimeParsing)?;

    let month = parser.next().ok_or(ParsingError::DatetimeFormat)?;

    let month = match month {
        "JAN" | "Jan" => 1,
        "FEB" | "Feb" => 2,
        "MAR" | "Mar" => 3,
        "APR" | "Apr" => 4,
        "MAY" | "May" => 5,
        "JUN" | "Jun" => 6,
        "JUL" | "Jul" => 7,
        "AUG" | "Aug" => 8,
        "SEP" | "Sep" => 9,
        "OCT" | "Oct" => 10,
        "NOV" | "Nov" => 11,
        "DEC" | "Dec" => 12,
        _ => {
            return Err(ParsingError::DatetimeParsing);
        },
    };

    let day = parser.next().ok_or(ParsingError::DatetimeFormat)?;
    let day = day
        .parse::<u8>()
        .map_err(|_| ParsingError::DatetimeParsing)?;

    Ok(Epoch::from_gregorian_utc_at_midnight(
        2000 + year,
        month,
        day,
    ))
}

/*
 * Parses the calibration validity FROM/UNTIL field
 */
fn parse_validity_epoch(content: &str) -> Result<Epoch, ParsingError> {
    let mut items = content.split_ascii_whitespace();

    let year = items.next().ok_or(ParsingError::DatetimeFormat)?;

    let year = year
        .parse::<i32>()
        .map_err(|_| ParsingError::DatetimeParsing)?;

    let month = items.next().ok_or(ParsingError::DatetimeFormat)?;

    let month = month
        .parse::<u8>()
        .map_err(|_| ParsingError::DatetimeParsing)?;

    let day = items.next().ok_or(ParsingError::DatetimeFormat)?;
    let day = day
        .parse::<u8>()
        .map_err(|_| ParsingError::DatetimeParsing)?;

    let hh = items.next().ok_or(ParsingError::DatetimeFormat)?;
    let hh = hh
        .parse::<u8>()
        .map_err(|_| ParsingError::DatetimeParsing)?;

    let mm = items.next().ok_or(ParsingError::DatetimeFormat)?;
    let mm = mm.parse::<u8>().map_err(|_| ParsingError::DatetimeFormat)?;

    let ss = items.next().ok_or(ParsingError::DatetimeParsing)?;

    let secs: u8;
    let mut nanos = 0_u32;

    if let Some(dot) = ss.find('.') {
        secs = ss[..dot]
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DatetimeParsing)?;

        nanos = ss[dot + 1..]
            .trim()
            .parse::<u32>()
            .map_err(|_| ParsingError::DatetimeParsing)?;
    } else {
        secs = ss
            .parse::<u8>()
            .map_err(|_| ParsingError::DatetimeParsing)?;
    }

    Ok(Epoch::from_gregorian_utc(
        year, month, day, hh, mm, secs, nanos,
    ))
}

/// Parses entire Antenna block
/// and all inner frequency entries
pub(crate) fn parse_antenna(
    content: &str,
) -> Result<(Antenna, HashMap<Carrier, FrequencyDependentData>), ParsingError> {
    let lines = content.lines();
    let mut antenna = Antenna::default();
    let mut inner = HashMap::<Carrier, FrequencyDependentData>::new();
    let mut frequency = Carrier::default();
    let mut freq_data = FrequencyDependentData::default();
    let mut valid_from = Epoch::default();

    for line in lines {
        let (content, marker) = line.split_at(60);
        if marker.contains("TYPE / SERIAL NO") {
            let (ant_igs, rem) = content.split_at(16); // IGS V.1.4 does not follow the specs ?
            let (block1, rem) = rem.split_at(20 + 4);
            let (block2, rem) = rem.split_at(10);
            let (block3, _rem) = rem.split_at(10);

            let (block1, block2, block3) = (block1.trim(), block2.trim(), block3.trim());
            /*
             * SV/RX antenna determination
             */
            let specificities = match block2.is_empty() && block3.is_empty() {
                false => AntennaSpecific::SvAntenna(SvAntenna {
                    igs_type: ant_igs.trim().to_string(),
                    sv: SV::from_str(block1)?,
                    cospar: COSPAR::from_str(block3)?,
                }),
                true => AntennaSpecific::RxAntenna(RxAntenna {
                    igs_type: ant_igs.trim().to_string(),
                    serial_number: {
                        if !block1.is_empty() && !block1.eq("NONE") {
                            Some(block1.to_string())
                        } else {
                            None
                        }
                    },
                }),
            };
            antenna = antenna.with_specificities(specificities);
        } else if marker.contains("METH / BY / # / DATE") {
            let (method, rem) = content.split_at(20);
            let (agency, rem) = rem.split_at(20);
            let (number, rem) = rem.split_at(10); // N#
            let (date, _) = rem.split_at(10);

            let cal = Calibration {
                method: CalibrationMethod::from_str(method.trim()).unwrap(),
                number: number
                    .trim()
                    .parse::<u16>()
                    .map_err(|_| ParsingError::AntexAntennaCalibrationNumber)?,
                agency: agency.trim().to_string(),
                date: parse_datetime(date.trim())?,
                validity_period: None,
            };

            antenna.calibration = cal.clone();
        } else if marker.contains("VALID FROM") {
            valid_from = parse_validity_epoch(content.trim())?;
        } else if marker.contains("VALID UNTIL") {
            let valid_until = parse_validity_epoch(content.trim())?;

            antenna = antenna.with_validity_period(valid_from, valid_until);
        } else if marker.contains("SINEX CODE") {
            let sinex = content.split_at(20).0;

            antenna.sinex_code = sinex.trim().to_string();
        } else if marker.contains("DAZI") {
            //let dazi = content.split_at(20).0.trim();
            //if let Ok(dazi) = f64::from_str(dazi) {
            //    antenna = antenna.with_dazi(dazi)
            //}
        } else if marker.contains("# OF FREQUENCIES") {
            /*
             * we actually do not care about this field
             * it is easy to determine it from the current infrastructure
             */
        } else if marker.contains("START OF FREQUENCY") {
            let svnn = content.split_at(10).0;
            let sv = SV::from_str(svnn.trim())?;
            frequency = Carrier::from_sv(sv)?;
        } else if marker.contains("NORTH / EAST / UP") {
            let (north, rem) = content.split_at(10);
            let (east, rem) = rem.split_at(10);
            let (up, _) = rem.split_at(10);

            let north = parse_f64(north.trim()).map_err(|_| ParsingError::AntexAPCCoordinates)?;

            let east = parse_f64(east.trim()).map_err(|_| ParsingError::AntexAPCCoordinates)?;

            let up = parse_f64(up.trim()).map_err(|_| ParsingError::AntexAPCCoordinates)?;

            freq_data.apc_eccentricity = (north, east, up);
        } else if marker.contains("ZEN1 / ZEN2 / DZEN") {
            let (start, rem) = content.split_at(8);
            let (end, rem) = rem.split_at(6);
            let (spacing, _) = rem.split_at(6);

            let start = parse_f64(start.trim()).map_err(|_| ParsingError::AntexZenithGrid)?;

            let end = parse_f64(end.trim()).map_err(|_| ParsingError::AntexZenithGrid)?;

            let spacing = parse_f64(spacing.trim()).map_err(|_| ParsingError::AntexZenithGrid)?;

            antenna.zenith_grid = Linspace {
                start,
                end,
                spacing,
            };
        } else if marker.contains("END OF FREQUENCY") {
            inner.insert(frequency, freq_data.clone());
        } else if marker.contains("END OF ANTENNA") {
            break; // end of this block, considered as an `epoch`
                   // if we make a parallel with other types of RINEX
        } else {
            // inside phase pattern
        }
        //    } else if marker.contains("SINEX CODE") {
        //        let sinex = content.split_at(10).0;
        //        antenna = antenna.with_sinex_code(sinex.trim())
        //        if line.contains("NOAZI") {
        //            frequency = frequency.add_pattern(Pattern::NonAzimuthDependent(values.clone()))
        //        } else {
        //            let angle = f64::from_str(content.trim()).unwrap();
        //            frequency =
        //                frequency.add_pattern(Pattern::AzimuthDependent((angle, values.clone())))
        //        }
        //    }
    }

    Ok((antenna, inner))
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_new_epoch() {
        let content = "                                                           START OF ANTENNA";
        assert!(is_new_epoch(content));
        let content =
            "TROSAR25.R4      LEIT727259                                 TYPE / SERIAL NO";
        assert!(!is_new_epoch(content));
        let content =
            "    26                                                      # OF FREQUENCIES";
        assert!(!is_new_epoch(content));
        let content =
            "   G01                                                      START OF FREQUENCY";
        assert!(!is_new_epoch(content));
    }
}
