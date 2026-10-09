//! Earth Orientation NAV frames

use crate::{
    epoch::parse_in_timescale as parse_epoch_in_timescale,
    error::FormattingError,
    navigation::formatting::{format_epoch_v4_fields, NavFormatter},
    parse_f64,
    prelude::{Epoch, ParsingError, TimeScale},
};

use std::io::{BufWriter, Write};

#[cfg(feature = "serde")]
use serde::Serialize;

/// Earth Orientation Message
#[derive(Debug, Clone, Default, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct EarthOrientation {
    /// ((arc-sec), (arc-sec.day⁻¹), (arc-sec.day⁻²))
    pub x: (f64, f64, f64),
    /// ((arc-sec), (arc-sec.day⁻¹), (arc-sec.day⁻²))
    pub y: (f64, f64, f64),
    /// Signed transmission seconds of the reference GNSS week
    pub t_tm: f64,
    /// Delta UT1 ((sec), (sec.day⁻¹), (sec.day⁻²))
    pub delta_ut1: (f64, f64, f64),
}

impl EarthOrientation {
    pub(crate) fn parse(
        line_1: &str,
        line_2: &str,
        line_3: &str,
        ts: TimeScale,
    ) -> Result<(Epoch, Self), ParsingError> {
        let bad = || ParsingError::NavEarthOrientationParsing;
        if [line_1, line_2, line_3]
            .iter()
            .any(|line| !line.is_ascii() || line.len() < 80 || !line[80..].trim().is_empty())
        {
            return Err(bad());
        }
        let scalar = |line: &str, start: usize| -> Result<f64, ParsingError> {
            let value = parse_f64(line[start..start + 19].trim()).map_err(|_| bad())?;
            if !value.is_finite() {
                return Err(bad());
            }
            Ok(value)
        };
        let epoch = parse_epoch_in_timescale(line_1[..23].trim(), ts)?;
        let x = (
            scalar(line_1, 23)?,
            scalar(line_1, 42)?,
            scalar(line_1, 61)?,
        );
        let y = (
            scalar(line_2, 23)?,
            scalar(line_2, 42)?,
            scalar(line_2, 61)?,
        );
        let t_tm = scalar(line_3, 4)?;
        let delta_ut1 = (
            scalar(line_3, 23)?,
            scalar(line_3, 42)?,
            scalar(line_3, 61)?,
        );
        Ok((
            epoch,
            Self {
                x,
                y,
                t_tm,
                delta_ut1,
            },
        ))
    }

    /// Formats the RINEX 4 EOP record (Table A34), following the
    /// "> EOP" record header.
    pub(crate) fn format_v4<W: Write>(
        &self,
        w: &mut BufWriter<W>,
        epoch: Epoch,
    ) -> Result<(), FormattingError> {
        writeln!(
            w,
            "    {}{}{}{}",
            format_epoch_v4_fields(epoch),
            NavFormatter::new(self.x.0),
            NavFormatter::new(self.x.1),
            NavFormatter::new(self.x.2),
        )?;

        writeln!(
            w,
            "{:23}{}{}{}",
            "",
            NavFormatter::new(self.y.0),
            NavFormatter::new(self.y.1),
            NavFormatter::new(self.y.2),
        )?;

        writeln!(
            w,
            "    {}{}{}{}",
            NavFormatter::new(self.t_tm),
            NavFormatter::new(self.delta_ut1.0),
            NavFormatter::new(self.delta_ut1.1),
            NavFormatter::new(self.delta_ut1.2),
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::EarthOrientation;
    use crate::prelude::{Epoch, TimeScale};
    use std::str::FromStr;
    #[test]
    fn earth_orientations_parsing() {
        for (
            line_1,
            line_2,
            line_3,
            ts,
            test_epoch,
            x_0,
            x_1,
            x_2,
            y_0,
            y_1,
            y_2,
            t_tm,
            dut1,
            ddut1,
            dddut1,
        ) in [(
            "    2023 03 14 16 51 12-4.024982452393e-02 3.957748413086e-05 0.000000000000e+00",
            "                        3.562908172607e-01 2.602100372314e-03 0.000000000000e+00",
            "     4.392000000000e+03-1.940387487411e-02-1.411736011505e-04 0.000000000000e+00",
            TimeScale::UTC,
            "2023-03-14T16:51:12 UTC",
            -4.024982452393e-02,
            3.957748413086e-05,
            0.0,
            3.562908172607e-01,
            2.602100372314e-03,
            0.0,
            4392.0,
            -1.940387487411e-02,
            -1.411736011505e-04,
            0.000000000000e+00,
        )] {
            let (t, eop) = EarthOrientation::parse(line_1, line_2, line_3, ts).unwrap();

            let test_epoch = Epoch::from_str(test_epoch).unwrap();

            assert_eq!(t, test_epoch);

            assert_eq!(eop.x.0, x_0);
            assert_eq!(eop.x.1, x_1);
            assert_eq!(eop.x.2, x_2);

            assert_eq!(eop.y.0, y_0);
            assert_eq!(eop.y.1, y_1);
            assert_eq!(eop.y.2, y_2);

            assert_eq!(eop.t_tm, t_tm);

            assert_eq!(eop.delta_ut1.0, dut1);
            assert_eq!(eop.delta_ut1.1, ddut1);
            assert_eq!(eop.delta_ut1.2, dddut1);
        }
    }
}
