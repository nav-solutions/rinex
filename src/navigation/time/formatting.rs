use crate::{
    epoch::epoch_decompose,
    errors::FormattingError,
    fmt_rinex,
    navigation::{formatting::NavFormatter, time::TimeOffset},
    prelude::{Epoch, TimeScale},
};

use std::io::{BufWriter, Write};

impl TimeOffset {
    /// Parse left hand side and right hand side [TimeScale]s
    fn to_lhs_rhs_timescales(&self) -> &str {
        match (self.lhs, self.rhs) {
            (TimeScale::GPST, TimeScale::UTC) => "GPUT",
            (TimeScale::GPST, TimeScale::GST) => "GPGA",
            (TimeScale::GPST, TimeScale::BDT) => "GPBD",
            (TimeScale::QZSST, TimeScale::UTC) => "QZUT",
            (TimeScale::QZSST, TimeScale::GST) => "QZGA",
            (TimeScale::QZSST, TimeScale::BDT) => "QZBD",
            (TimeScale::GST, TimeScale::UTC) => "GAUT",
            (TimeScale::GST, TimeScale::GPST) => "GAGP",
            (TimeScale::GST, TimeScale::BDT) => "GABD",
            (TimeScale::BDT, TimeScale::UTC) => "BDUT",
            (TimeScale::BDT, TimeScale::GST) => "BDGA",
            (TimeScale::BDT, TimeScale::GPST) => "BDGP",
            _ => "GPUT",
        }
    }

    /// Format [TimeOffset] according to RINEXv2 standard
    pub(crate) fn format_v2_delta_utc<W: Write>(
        &self,
        w: &mut BufWriter<W>,
    ) -> Result<(), FormattingError> {
        writeln!(
            w,
            "{}",
            fmt_rinex(
                &format!(
                    "   {}{} {:8} {:8}",
                    NavFormatter::new_time_system_correction_v2(self.polynomial.0),
                    NavFormatter::new_time_system_correction_v2(self.polynomial.1),
                    self.t_ref.1 / 1_000_000_000,
                    self.t_ref.0,
                ),
                "DELTA-UTC: A0,A1,T,W",
            ),
        )?;

        Ok(())
    }

    /// Format [TimeOffset] according to RINEXv2 format
    pub(crate) fn format_v2_corr_to_system_time<W: Write>(
        &self,
        w: &mut BufWriter<W>,
    ) -> Result<(), FormattingError> {
        let t = Epoch::from_time_of_week(self.t_ref.0, self.t_ref.1, self.lhs);

        let (y, m, d, _, _, _, _) = epoch_decompose(t);

        writeln!(
            w,
            "{}",
            fmt_rinex(
                &format!(
                    "{:6}{:6}{:6}   {}",
                    y,
                    m,
                    d,
                    NavFormatter::new_time_system_correction_v2(self.polynomial.0)
                ),
                "CORR TO SYSTEM TIME",
            ),
        )?;

        Ok(())
    }

    /// Format [TimeOffset] according to RINEXv3 standard
    pub(crate) fn format_v3<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        writeln!(
            w,
            "{}",
            fmt_rinex(
                &format!(
                    "{} {}{} {:6}{:5}",
                    self.to_lhs_rhs_timescales(),
                    NavFormatter::new_time_system_correction_v3_offset(self.polynomial.0),
                    NavFormatter::new_time_system_correction_v3_drift(self.polynomial.1),
                    self.t_ref.1 / 1_000_000_000,
                    self.t_ref.0
                ),
                "TIME SYSTEM CORR"
            ),
        )?;

        Ok(())
    }

    pub(crate) fn format_v4<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        let t = Epoch::from_time_of_week(self.t_ref.0, self.t_ref.1, self.lhs);
        let (y, m, d, hh, mm, ss, _) = epoch_decompose(t);

        writeln!(
            w,
            "    {:04} {:02} {:02} {:02} {:02} {:02} {}",
            y,
            m,
            d,
            hh,
            mm,
            ss,
            self.to_lhs_rhs_timescales(),
        )?;

        writeln!(
            w,
            "    {}{}{}{}",
            NavFormatter::new((self.t_ref.1 / 1_000_000_000) as f64),
            NavFormatter::new(self.polynomial.0),
            NavFormatter::new(self.polynomial.1),
            NavFormatter::new(self.polynomial.2),
        )?;

        Ok(())
    }
}
