use crate::{
    epoch::parse_in_timescale as parse_epoch_in_timescale,
    error::ParsingError,
    navigation::time::TimeOffset,
    prelude::{Epoch, TimeScale},
};

impl TimeOffset {
    /// Parse left hand side and right hand side [TimeScale]s
    fn parse_lhs_rhs_timescales(content: &str) -> Result<(TimeScale, TimeScale), ParsingError> {
        match content {
            // gps
            "GPGA" => Ok((TimeScale::GPST, TimeScale::GST)),
            "GPUT" => Ok((TimeScale::GPST, TimeScale::UTC)),
            // qzss
            "QZGP" => Ok((TimeScale::QZSST, TimeScale::GPST)),
            "QZUT" => Ok((TimeScale::QZSST, TimeScale::UTC)),
            // gal
            "GAGP" => Ok((TimeScale::GST, TimeScale::GPST)),
            "GAUT" => Ok((TimeScale::GST, TimeScale::UTC)),
            // beidou
            "BDUT" => Ok((TimeScale::BDT, TimeScale::UTC)),
            "BDGA" => Ok((TimeScale::BDT, TimeScale::GST)),
            "BDGP" => Ok((TimeScale::BDT, TimeScale::GPST)),
            // "SBAS"
            "SBUT" => Ok((TimeScale::GPST, TimeScale::UTC)),
            _ => Err(ParsingError::NavInvalidTimescale),
        }
    }

    /// Parse [TimeOffset] from RINEXv2 standard
    pub fn parse_v2_delta_utc(line: &str) -> Result<Self, ParsingError> {
        let (a0, rem) = line.split_at(22);
        let (a1, rem) = rem.split_at(19);
        let (seconds, rem) = rem.split_at(9);
        let (week, _) = rem.split_at(9);

        let week = week
            .trim()
            .parse::<u32>()
            .map_err(|_| ParsingError::NavEpochWeekCounter)?;

        let seconds = seconds
            .trim()
            .parse::<u64>()
            .map_err(|_| ParsingError::NavEpochWeekCounter)?;

        let a0 = a0
            .trim()
            .replace('D', "e")
            .parse::<f64>()
            .map_err(|_| ParsingError::NavTimeOffsetParinsg)?;

        let a1 = a1
            .trim()
            .replace('D', "e")
            .parse::<f64>()
            .map_err(|_| ParsingError::NavTimeOffsetParinsg)?;

        Ok(Self::from_time_of_week(
            week,
            seconds * 1_000_000_000,
            TimeScale::GPST,
            TimeScale::UTC,
            (a0, a1, 0.0),
        ))
    }

    /// Parse [TimeOffset] from RINEXv2 standard
    pub fn parse_v2_corr_to_system_time(line: &str) -> Result<Self, ParsingError> {
        let (year, rem) = line.split_at(6);
        let (month, rem) = rem.split_at(6);
        let (day, rem) = rem.split_at(6);
        let (tau, _) = rem.split_at(22);

        let year = year
            .trim()
            .parse::<i32>()
            .map_err(|_| ParsingError::NavTimeOffsetParinsg)?;

        let month = month
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::NavTimeOffsetParinsg)?;

        let day = day
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::NavTimeOffsetParinsg)?;

        let t_ref = Epoch::from_gregorian_utc_at_midnight(year, month, day);

        let a0 = tau
            .trim()
            .replace('D', "e")
            .parse::<f64>()
            .map_err(|_| ParsingError::NavTimeOffsetParinsg)?;

        Ok(Self::from_epoch(
            t_ref,
            TimeScale::GPST, //TODO GlonassT
            TimeScale::UTC,
            (a0, 0.0, 0.0),
        ))
    }

    /// Parse [TimeOffset] from RINEXv3 standard
    pub fn parse_v3(line: &str) -> Result<Self, ParsingError> {
        let (timescales, rem) = line.split_at(4);
        let (a0, rem) = rem.split_at(18);
        let (a1, rem) = rem.split_at(16);
        let (seconds, rem) = rem.split_at(7);
        let (week, _) = rem.split_at(5);

        let (lhs, rhs) = Self::parse_lhs_rhs_timescales(timescales)?;

        let week = week
            .trim()
            .parse::<u32>()
            .map_err(|_| ParsingError::NavEpochWeekCounter)?;

        let seconds = seconds
            .trim()
            .parse::<u64>()
            .map_err(|_| ParsingError::NavEpochWeekCounter)?;

        let a0 = a0
            .trim()
            .replace('D', "e")
            .parse::<f64>()
            .map_err(|_| ParsingError::NavTimeOffsetParinsg)?;

        let a1 = a1
            .trim()
            .replace('D', "e")
            .parse::<f64>()
            .map_err(|_| ParsingError::NavTimeOffsetParinsg)?;

        Ok(Self::from_time_of_week(
            week,
            seconds * 1_000_000_000,
            lhs,
            rhs,
            (a0, a1, 0.0),
        ))
    }

    /// Parse [TimeOffset] from RINEXv4 standard
    pub fn parse_v4(line_1: &str, line_2: &str) -> Result<Self, ParsingError> {
        let (epoch, rem) = line_1.split_at(24);
        let (timescales, _) = rem.split_at(4);

        let (lhs, rhs) = Self::parse_lhs_rhs_timescales(timescales)?;

        // let utc = rem.trim().to_string();
        let t_ref = parse_epoch_in_timescale(epoch.trim(), lhs)?;
        let (t_week, t_nanos) = t_ref.to_time_of_week();

        let (_, rem) = line_2.split_at(23);
        let (a0, rem) = rem.split_at(19);
        let (a1, rem) = rem.split_at(19);
        let (a2, _) = rem.split_at(19);

        // let t_tm = t_tm
        //     .trim()
        //     .replace('D', "e")
        //     .parse::<f64>()
        //     .map_err(|_| ParsingError::NavTimeOffsetParinsg)?;

        let (a0, a1, a2) = (
            a0.trim()
                .replace('D', "e")
                .parse::<f64>()
                .map_err(|_| ParsingError::NavTimeOffsetParinsg)?,
            a1.trim()
                .replace('D', "e")
                .parse::<f64>()
                .map_err(|_| ParsingError::NavTimeOffsetParinsg)?,
            a2.trim()
                .replace('D', "e")
                .parse::<f64>()
                .map_err(|_| ParsingError::NavTimeOffsetParinsg)?,
        );

        let time_offset = Self::from_time_of_week(t_week, t_nanos, lhs, rhs, (a0, a1, a2));

        Ok(time_offset)
    }
}

#[cfg(test)]
mod test {
    use std::io::BufWriter;
    use std::str::FromStr;

    use super::TimeOffset;
    use crate::prelude::{Epoch, TimeScale};
    use crate::tests::formatting::Utf8Buffer;

    #[test]
    fn parsing_delta_utc_v2() {
        for (content, a0, a1, sec, week) in [(
            "     .133179128170D-06  .107469588780D-12   552960     1025 ",
            0.133179128170E-06,
            0.107469588780E-12,
            552960,
            1025,
        )] {
            let expected = TimeOffset::from_time_of_week(
                week,
                sec * 1_000_000_000,
                TimeScale::GPST,
                TimeScale::UTC,
                (a0, a1, 0.0),
            );

            let parsed = TimeOffset::parse_v2_delta_utc(content).unwrap();
            assert_eq!(parsed, expected);

            // test reciprocal
            let mut buf = BufWriter::new(Utf8Buffer::new(1024));
            parsed.format_v2_delta_utc(&mut buf).unwrap();

            let formatted = buf.into_inner().unwrap().to_ascii_utf8();

            let reparsed = TimeOffset::parse_v2_delta_utc(&formatted).unwrap();
            assert_eq!(parsed, reparsed);
        }
    }

    #[test]
    fn parsing_correction_to_system_time_v2() {
        for (content, y, m, d, a0) in [(
            "  2021     1     1   -1.862645149231D-09                    ",
            2021,
            1,
            1,
            -1.862645149231E-09,
        )] {
            let t_ref = Epoch::from_gregorian_utc_at_midnight(y, m, d);

            let expected =
                TimeOffset::from_epoch(t_ref, TimeScale::GPST, TimeScale::UTC, (a0, 0.0, 0.0));

            let parsed = TimeOffset::parse_v2_corr_to_system_time(content).unwrap();
            assert_eq!(parsed, expected);

            // test reciprocal
            let mut buf = BufWriter::new(Utf8Buffer::new(1024));
            parsed.format_v2_corr_to_system_time(&mut buf).unwrap();

            let formatted = buf.into_inner().unwrap().to_ascii_utf8();

            let reparsed = TimeOffset::parse_v2_corr_to_system_time(&formatted).unwrap();
            assert_eq!(parsed, reparsed);
        }
    }

    #[test]
    fn parsing_v3() {
        // TODO
        // GLUT -1.8626451492e-09 0.000000000e+00      0    0          TIME SYSTEM CORR
        // GLGP -2.1420419216e-08 0.000000000e+00 518400 2138          TIME SYSTEM CORR

        // TODO
        // IRUT -9.6333678812e-09 1.776356839e-15 345888 1114          TIME SYSTEM CORR
        // IRGP -4.9476511776e-10-2.664535259e-15 432288 2138          TIME SYSTEM CORR

        let content = "XXXX  1.7840648070e-08 5.773159728e-14 432288 2138          ";
        assert!(TimeOffset::parse_v3(content).is_err());

        for (content, a0, a1, week, sec, lhs, rhs) in [
            (
                "GAUT  1.8626451492E-09-8.881784197E-16 432000 2138          TIME SYSTEM CORR\n",
                1.8626451492e-09,
                -8.881784197e-16,
                2138,
                432000,
                TimeScale::GST,
                TimeScale::UTC,
            ),
            (
                "GPUT -3.7252902985E-09-1.065814104E-14  61440 2139          TIME SYSTEM CORR\n",
                -3.7252902985e-09,
                -1.065814104e-14,
                2139,
                61440,
                TimeScale::GPST,
                TimeScale::UTC,
            ),
            (
                "GAGP  2.1536834538E-09-9.769962617E-15 432000 2138          TIME SYSTEM CORR\n",
                2.1536834538e-09,
                -9.769962617e-15,
                2138,
                432000,
                TimeScale::GST,
                TimeScale::GPST,
            ),
            (
                "BDUT  0.0000000000E+00-4.085620730E-14     14  782          TIME SYSTEM CORR\n",
                0.0,
                -4.085620730e-14,
                782,
                14,
                TimeScale::BDT,
                TimeScale::UTC,
            ),
            (
                "QZUT  5.5879354477E-09 0.000000000E+00  94208 2139          TIME SYSTEM CORR\n",
                5.5879354477e-09,
                0.000000000e+00,
                2139,
                94208,
                TimeScale::QZSST,
                TimeScale::UTC,
            ),
        ] {
            let expected =
                TimeOffset::from_time_of_week(week, sec * 1_000_000_000, lhs, rhs, (a0, a1, 0.0));

            let parsed = TimeOffset::parse_v3(content).unwrap();

            assert_eq!(parsed, expected);

            // test reciprocal
            let mut buf = BufWriter::new(Utf8Buffer::new(1024));
            parsed.format_v3(&mut buf).unwrap();

            let formatted = buf.into_inner().unwrap().to_ascii_utf8();
            assert_eq!(formatted, content);

            let reparsed = TimeOffset::parse_v3(&formatted)
                .unwrap_or_else(|e| panic!("Parse back failed for \"{}\" - {}", formatted, e));

            assert_eq!(parsed, reparsed);
        }
    }

    #[test]
    fn parsing_v4() {
        for (line_1, line_2, lhs, rhs, t_ref, t_sec, a0, a1, a2) in [
            (
                "    2022 06 08 00 00 00 GAUT                                  UTCGAL",
                "     2.952070000000E+05-1.862645149231E-09 8.881784197001E-16 0.000000000000E+00",
                TimeScale::GST,
                TimeScale::UTC,
                "2022-06-08T00:00:00 GST",
                295207,
                -1.862645149231E-09,
                8.881784197001E-16,
                0.0,
            ),
            (
                "    2022 06 10 19 56 48 GPUT                                  UTC(USNO)",
                "     2.952840000000E+05 9.313225746155E-10 2.664535259100E-15 0.000000000000E+00",
                TimeScale::GPST,
                TimeScale::UTC,
                "2022-06-10T19:56:48 GPST",
                295284,
                9.313225746155E-10,
                2.664535259100E-15,
                0.0,
            ),
        ] {
            let t_ref = Epoch::from_str(t_ref).unwrap();
            let (t_ref_week, _) = t_ref.to_time_of_week();

            let time_offset = TimeOffset::parse_v4(line_1, line_2).unwrap();

            assert_eq!(time_offset.lhs, lhs);
            assert_eq!(time_offset.rhs, rhs);
            assert_eq!(time_offset.t_ref.0, t_ref_week);
            //assert_eq!(time_offset.t_ref.1, t_sec * 1_000_000_000);

            assert_eq!(time_offset.polynomial, (a0, a1, a2),);

            // test reciprocal
            let mut buf = BufWriter::new(Utf8Buffer::new(1024));
            time_offset.format_v4(&mut buf).unwrap();

            let formatted = buf.into_inner().unwrap().to_ascii_utf8();

            for (index, line) in formatted.split('\n').enumerate() {
                if index == 0 {
                    // assert_eq!(line, line_1);
                } else if index == 1 {
                    // assert_eq!(line, line_2);
                } else if index == 3 {
                    panic!("two lines expected (only)!");
                }
            }
        }
    }
}
