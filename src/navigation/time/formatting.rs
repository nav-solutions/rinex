use crate::{
    epoch::epoch_decompose,
    error::FormattingError,
    fmt_rinex,
    navigation::{
        formatting::{format_epoch_v4_fields, NavFormatter},
        time::TimeOffset,
    },
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
            (TimeScale::QZSST, TimeScale::GPST) => "QZGP",
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
                    self.time_system
                        .as_deref()
                        .unwrap_or_else(|| self.to_lhs_rhs_timescales()),
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

    /// Format STO preserving the distinct reference epoch, transmission time,
    /// time-system pair and SBAS/UTC identifiers (RINEX Table A30).
    pub(crate) fn format_v4<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        let transmit = self
            .transmission_time
            .ok_or(FormattingError::NavMissingTransmissionTime)?;
        let t = Epoch::from_time_of_week(self.t_ref.0, self.t_ref.1, self.lhs);
        writeln!(
            w,
            "    {} {:<18} {:<18} {:<18}",
            format_epoch_v4_fields(t),
            self.time_system
                .as_deref()
                .unwrap_or_else(|| self.to_lhs_rhs_timescales()),
            self.sbas.as_deref().unwrap_or(""),
            self.utc.as_deref().unwrap_or("")
        )?;
        writeln!(
            w,
            "    {}{}{}{}",
            NavFormatter::new(transmit),
            NavFormatter::new(self.polynomial.0),
            NavFormatter::new(self.polynomial.1),
            NavFormatter::new(self.polynomial.2)
        )?;
        Ok(())
    }
}
