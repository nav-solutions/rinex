//! Epoch parsing helper

use crate::{
    prelude::{Epoch, ParsingError, TimeScale},
    types::Type,
};

/// Infaillible `Epoch::now()` call.
pub(crate) fn now() -> Epoch {
    Epoch::now().unwrap_or(Epoch::from_gregorian_utc_at_midnight(2000, 1, 1))
}

/// Parse "Jan" like month string
pub fn parse_formatted_month(content: &str) -> Result<u8, ParsingError> {
    match content {
        "Jan" => Ok(1),
        "Feb" => Ok(2),
        "Mar" => Ok(3),
        "Apr" => Ok(4),
        "May" => Ok(5),
        "Jun" => Ok(6),
        "Jul" => Ok(7),
        "Aug" => Ok(8),
        "Sep" => Ok(9),
        "Oct" => Ok(10),
        "Nov" => Ok(11),
        "Dec" => Ok(12),
        _ => Err(ParsingError::DatetimeParsing),
    }
}

/// Formats given epoch to string, matching standard specifications
pub(crate) fn format(epoch: Epoch, t: Type, revision: u8) -> String {
    let (y, m, d, hh, mm, ss, nanos) = epoch_decompose(epoch);

    match t {
        Type::ObservationData => {
            if revision < 3 {
                // old RINEX wants 2 digit YY field
                let mut y = y - 2000;
                if y < 0 {
                    // fix: files recorded prior 21st century
                    y += 100;
                }
                format!(
                    "{:02} {:>2} {:>2} {:>2} {:>2} {:>2}.{:07}",
                    y,
                    m,
                    d,
                    hh,
                    mm,
                    ss,
                    nanos / 100,
                )
            } else {
                format!(
                    "{:04} {:02} {:02} {:02} {:02} {:>2}.{:07}",
                    y,
                    m,
                    d,
                    hh,
                    mm,
                    ss,
                    nanos / 100,
                )
            }
        },
        Type::NavigationData => {
            if revision < 3 {
                // old RINEX wants 2 digit YY field
                let mut y = y - 2000;
                if y < 0 {
                    // fix: files recorded prior 21st century
                    y += 100;
                }
                format!(
                    "{:02} {:>2} {:>2} {:>2} {:>2} {:>2}.{:1}",
                    y,
                    m,
                    d,
                    hh,
                    mm,
                    ss,
                    nanos / 100_000_000
                )
            } else {
                format!("{:04} {:02} {:02} {:02} {:02} {:02}", y, m, d, hh, mm, ss)
            }
        },
        _ => {
            if revision < 3 {
                // old RINEX wants 2 digit YY field
                let mut y = y - 2000;
                if y < 0 {
                    // fix: files recorded prior 21st century
                    y += 100;
                }
                format!("{:02} {:>2} {:>2} {:>2} {:>2} {:>2}", y, m, d, hh, mm, ss)
            } else {
                format!("{:04} {:>2} {:>2} {:>2} {:>2} {:>2}", y, m, d, hh, mm, ss)
            }
        },
    }
}

/// Parses [Epoch] from string, interprated in [TimeScale]
pub(crate) fn parse_in_timescale(content: &str, ts: TimeScale) -> Result<Epoch, ParsingError> {
    let mut y = 0_i32;
    let mut m = 0_u8;
    let mut d = 0_u8;
    let mut hh = 0_u8;
    let mut mm = 0_u8;
    let mut ss = 0_u8;
    let mut ns = 0_u64;

    if content.split_ascii_whitespace().count() < 6 {
        return Err(ParsingError::EpochFormat);
    }

    for (field_index, item) in content.split_ascii_whitespace().enumerate() {
        match field_index {
            0 => {
                y = item
                    .parse::<i32>()
                    .map_err(|_| ParsingError::EpochParsing)?;

                /* old RINEX problem: YY sometimes encoded on two digits */
                if y > 79 && y <= 99 {
                    y += 1900;
                } else if y < 79 {
                    y += 2000;
                }
            },
            1 => {
                m = item.parse::<u8>().map_err(|_| ParsingError::EpochParsing)?;
            },
            2 => {
                d = item.parse::<u8>().map_err(|_| ParsingError::EpochParsing)?;
            },
            3 => {
                hh = item.parse::<u8>().map_err(|_| ParsingError::EpochParsing)?;
            },
            4 => {
                mm = item.parse::<u8>().map_err(|_| ParsingError::EpochParsing)?;
            },
            5 => {
                if let Some(dot) = item.find('.') {
                    let is_nav = item.trim().len() < 7;

                    ss = item[..dot]
                        .trim()
                        .parse::<u8>()
                        .map_err(|_| ParsingError::EpochParsing)?;

                    let nanos = item[dot + 1..].trim();

                    ns = nanos
                        .parse::<u64>()
                        .map_err(|_| ParsingError::EpochParsing)?;

                    if is_nav {
                        // NAV RINEX : 100ms precision
                        ns *= 100_000_000;
                    } else if nanos.len() != 9 {
                        // OBS RINEX : 100ns precision
                        ns *= 100;
                    }
                } else {
                    ss = item
                        .trim()
                        .parse::<u8>()
                        .map_err(|_| ParsingError::EpochParsing)?;
                }
            },
            _ => {},
        }
    }

    //println!("content \"{}\"", content); // DEBUG
    //println!("Y {} M {} D {} HH {} MM {} SS {} NS {}", y, m, d, hh, mm, ss, ns); // DEBUG
    match ts {
        TimeScale::UTC => {
            // Catch possible Hifitime panic on bad string content
            if y == 0 {
                return Err(ParsingError::EpochFormat);
            }
            let epoch = Epoch::from_gregorian_utc(y, m, d, hh, mm, ss, ns as u32);
            Ok(epoch)
        },
        TimeScale::TAI => {
            // Catch possible Hifitime panic on bad string content
            if y == 0 {
                return Err(ParsingError::EpochFormat);
            }
            let epoch = Epoch::from_gregorian_tai(y, m, d, hh, mm, ss, ns as u32);
            Ok(epoch)
        },
        ts => {
            // Catch possible Hifitime panic on bad string content
            if y == 0 {
                return Err(ParsingError::EpochFormat);
            }
            let epoch = Epoch::from_gregorian_str(&format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06} {}",
                y, m, d, hh, mm, ss, ns, ts
            ))?;
            Ok(epoch)
        },
    }
}

pub(crate) fn parse_utc(s: &str) -> Result<Epoch, ParsingError> {
    parse_in_timescale(s, TimeScale::UTC)
}

pub(crate) fn epoch_decompose(epoch: Epoch) -> (i32, u8, u8, u8, u8, u8, u32) {
    epoch.to_gregorian(epoch.time_scale)
}

#[cfg(test)]
mod test {
    use super::*;
    use hifitime::Epoch;
    use hifitime::TimeScale;
    use std::str::FromStr;

    #[test]
    fn epoch_parse_nav_v2() {
        let e = parse_utc("20 12 31 23 45  0.0");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2020);
        assert_eq!(m, 12);
        assert_eq!(d, 31);
        assert_eq!(hh, 23);
        assert_eq!(mm, 45);
        assert_eq!(ss, 0);
        assert_eq!(ns, 0);
        assert_eq!(e.time_scale, TimeScale::UTC);
        assert_eq!(format(e, Type::NavigationData, 2), "20 12 31 23 45  0.0");

        let e = parse_utc("21  1  1 16 15  0.0");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2021);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
        assert_eq!(hh, 16);
        assert_eq!(mm, 15);
        assert_eq!(ss, 0);
        assert_eq!(ns, 0);
        assert_eq!(e.time_scale, TimeScale::UTC);
        assert_eq!(format(e, Type::NavigationData, 2), "21  1  1 16 15  0.0");
    }

    #[test]
    fn epoch_parse_nav_v2_nanos() {
        let e = parse_utc("20 12 31 23 45  0.1");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (_, _, _, _, _, ss, ns) = e.to_gregorian_utc();
        assert_eq!(ss, 0);
        assert_eq!(ns, 100_000_000);
        assert_eq!(format(e, Type::NavigationData, 2), "20 12 31 23 45  0.1");
    }

    #[test]
    fn epoch_parse_nav_v3() {
        let e = parse_utc("2021 01 01 00 00 00 ");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2021);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
        assert_eq!(hh, 00);
        assert_eq!(mm, 00);
        assert_eq!(ss, 0);
        assert_eq!(ns, 0);
        assert_eq!(e.time_scale, TimeScale::UTC);
        assert_eq!(format(e, Type::NavigationData, 3), "2021 01 01 00 00 00");

        let e = parse_utc("2021 01 01 09 45 00 ");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2021);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
        assert_eq!(hh, 09);
        assert_eq!(mm, 45);
        assert_eq!(ss, 0);
        assert_eq!(ns, 0);
        assert_eq!(format(e, Type::NavigationData, 3), "2021 01 01 09 45 00");

        let e = parse_utc("2020 06 25 00 00 00");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2020);
        assert_eq!(m, 6);
        assert_eq!(d, 25);
        assert_eq!(hh, 00);
        assert_eq!(mm, 00);
        assert_eq!(ss, 0);
        assert_eq!(ns, 0);
        assert_eq!(format(e, Type::NavigationData, 3), "2020 06 25 00 00 00");

        let e = parse_utc("2020 06 25 09 49 04");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2020);
        assert_eq!(m, 6);
        assert_eq!(d, 25);
        assert_eq!(hh, 09);
        assert_eq!(mm, 49);
        assert_eq!(ss, 04);
        assert_eq!(ns, 0);
        assert_eq!(format(e, Type::NavigationData, 3), "2020 06 25 09 49 04");
    }

    #[test]
    fn epoch_parse_obs_v2() {
        let e = parse_utc(" 21 12 21  0  0  0.0000000");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2021);
        assert_eq!(m, 12);
        assert_eq!(d, 21);
        assert_eq!(hh, 00);
        assert_eq!(mm, 00);
        assert_eq!(ss, 0);
        assert_eq!(ns, 0);
        assert_eq!(e.time_scale, TimeScale::UTC);
        assert_eq!(
            format(e, Type::ObservationData, 2),
            "21 12 21  0  0  0.0000000"
        );

        let e = parse_utc(" 21 12 21  0  0 30.0000000");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2021);
        assert_eq!(m, 12);
        assert_eq!(d, 21);
        assert_eq!(hh, 00);
        assert_eq!(mm, 00);
        assert_eq!(ss, 30);
        assert_eq!(ns, 0);
        assert_eq!(
            format(e, Type::ObservationData, 2),
            "21 12 21  0  0 30.0000000"
        );

        let e = parse_utc(" 21  1  1  0  0  0.0000000");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2021);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
        assert_eq!(hh, 00);
        assert_eq!(mm, 00);
        assert_eq!(ss, 0);
        assert_eq!(ns, 0);
        assert_eq!(
            format(e, Type::ObservationData, 2),
            "21  1  1  0  0  0.0000000"
        );

        let e = parse_utc(" 21  1  1  0  7 30.0000000");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2021);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
        assert_eq!(hh, 00);
        assert_eq!(mm, 7);
        assert_eq!(ss, 30);
        assert_eq!(ns, 0);
        assert_eq!(
            format(e, Type::ObservationData, 2),
            "21  1  1  0  7 30.0000000"
        );
    }

    #[test]
    fn epoch_parse_obs_v3() {
        let e = parse_utc(" 2022 01 09 00 00  0.0000000");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2022);
        assert_eq!(m, 1);
        assert_eq!(d, 9);
        assert_eq!(hh, 00);
        assert_eq!(mm, 0);
        assert_eq!(ss, 00);
        assert_eq!(ns, 0);
        assert_eq!(
            format(e, Type::ObservationData, 3),
            "2022 01 09 00 00  0.0000000"
        );

        let e = parse_utc(" 2022 01 09 00 13 30.0000000");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2022);
        assert_eq!(m, 1);
        assert_eq!(d, 9);
        assert_eq!(hh, 00);
        assert_eq!(mm, 13);
        assert_eq!(ss, 30);
        assert_eq!(ns, 0);
        assert_eq!(
            format(e, Type::ObservationData, 3),
            "2022 01 09 00 13 30.0000000"
        );

        let e = parse_utc(" 2022 03 04 00 52 30.0000000");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2022);
        assert_eq!(m, 3);
        assert_eq!(d, 4);
        assert_eq!(hh, 00);
        assert_eq!(mm, 52);
        assert_eq!(ss, 30);
        assert_eq!(ns, 0);
        assert_eq!(
            format(e, Type::ObservationData, 3),
            "2022 03 04 00 52 30.0000000"
        );

        let e = parse_utc(" 2022 03 04 00 02 30.0000000");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2022);
        assert_eq!(m, 3);
        assert_eq!(d, 4);
        assert_eq!(hh, 00);
        assert_eq!(mm, 02);
        assert_eq!(ss, 30);
        assert_eq!(ns, 0);
        assert_eq!(
            format(e, Type::ObservationData, 3),
            "2022 03 04 00 02 30.0000000"
        );
    }

    #[test]
    fn epoch_parse_obs_v2_nanos() {
        let e = parse_utc(" 21  1  1  0  7 39.1234567");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (_, _, _, _, _, ss, ns) = e.to_gregorian_utc();
        assert_eq!(ss, 39);
        assert_eq!(ns, 123_456_700);
        assert_eq!(
            format(e, Type::ObservationData, 2),
            "21  1  1  0  7 39.1234567"
        );
    }

    #[test]
    fn epoch_parse_obs_v3_nanos() {
        let e = parse_utc("2022 01 09 00 00  0.1000000");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (_, _, _, _, _, ss, ns) = e.to_gregorian_utc();
        assert_eq!(ss, 0);
        assert_eq!(ns, 100_000_000);
        assert_eq!(
            format(e, Type::ObservationData, 3),
            "2022 01 09 00 00  0.1000000"
        );

        let e = parse_utc(" 2022 01 09 00 00  0.1234000");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (_, _, _, _, _, ss, ns) = e.to_gregorian_utc();
        assert_eq!(ss, 0);
        assert_eq!(ns, 123_400_000);
        assert_eq!(
            format(e, Type::ObservationData, 3),
            "2022 01 09 00 00  0.1234000"
        );

        let e = parse_utc(" 2022 01 09 00 00  8.7654321");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (_, _, _, _, _, ss, ns) = e.to_gregorian_utc();
        assert_eq!(ss, 8);
        assert_eq!(ns, 765_432_100);
        assert_eq!(
            format(e, Type::ObservationData, 3),
            "2022 01 09 00 00  8.7654321"
        );
    }

    #[test]
    fn epoch_parse_meteo_v2() {
        let e = parse_utc(" 22  1  4  0  0  0  ");
        assert!(e.is_ok());
        let e = e.unwrap();
        let (y, m, d, hh, mm, ss, ns) = e.to_gregorian_utc();
        assert_eq!(y, 2022);
        assert_eq!(m, 1);
        assert_eq!(d, 4);
        assert_eq!(hh, 00);
        assert_eq!(mm, 00);
        assert_eq!(ss, 00);
        assert_eq!(ns, 0);
        assert_eq!(format(e, Type::MeteoData, 2), "22  1  4  0  0  0");
    }

    #[test]
    fn epoch_decomposition() {
        for (epoch, y, m, d, hh, mm, ss, ns) in [
            ("2021-01-01T00:00:00 GPST", 2021, 1, 1, 0, 0, 0, 0),
            ("2021-01-01T00:00:01 GPST", 2021, 1, 1, 0, 0, 1, 0),
            ("2021-01-01T23:59:58 GPST", 2021, 1, 1, 23, 59, 58, 0),
            ("2021-01-01T23:59:59 GPST", 2021, 1, 1, 23, 59, 59, 0),
            ("2021-01-01T00:00:00 GST", 2021, 1, 1, 0, 0, 0, 0),
            ("2021-01-01T00:00:01 GST", 2021, 1, 1, 0, 0, 1, 0),
            ("2021-01-01T23:59:58 GST", 2021, 1, 1, 23, 59, 58, 0),
            ("2021-01-01T23:59:59 GST", 2021, 1, 1, 23, 59, 59, 0),
        ] {
            let e = Epoch::from_str(epoch).unwrap();
            assert_eq!(
                epoch_decompose(e),
                (y, m, d, hh, mm, ss, ns),
                "failed for {}",
                epoch
            );
        }
    }

    #[test]
    fn test_formatted_month() {
        assert_eq!(parse_formatted_month("Jan").unwrap(), 1);
        assert_eq!(parse_formatted_month("Feb").unwrap(), 2);
        assert_eq!(parse_formatted_month("Mar").unwrap(), 3);
        assert_eq!(parse_formatted_month("Aug").unwrap(), 8);
        assert_eq!(parse_formatted_month("Dec").unwrap(), 12);

        assert!(parse_formatted_month("Ced").is_err());
    }
}
