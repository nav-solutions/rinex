//! `Navigation` new EOP Earth Orientation messages
use crate::epoch;
use crate::parse_f64;
use crate::prelude::*;
use thiserror::Error;

/// EopMessage Parsing error
#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to parse epoch")]
    EpochParsingError(#[from] epoch::ParsingError),
    #[error("eop message missing 1st line")]
    EopMissing1stLine,
    #[error("eop message missing 2nd line")]
    EopMissing2ndLine,
    #[error("eop message missing 3rd line")]
    EopMissing3rdLine,
}

/// Earth Orientation Message
#[derive(Debug, Clone, Default, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct EopMessage {
    /// ((arc-sec), (arc-sec.day⁻¹), (arc-sec.day⁻²))
    pub x: (f64, f64, f64),
    /// ((arc-sec), (arc-sec.day⁻¹), (arc-sec.day⁻²))
    pub y: (f64, f64, f64),
    /// Message transmmission time in seconds of GNSS week
    pub t_tm: u32,
    /// Delta UT1 ((sec), (sec.day⁻¹), (sec.day⁻²))
    pub delta_ut1: (f64, f64, f64),
}

impl EopMessage {
    pub(crate) fn parse(
        mut lines: std::str::Lines<'_>,
        ts: TimeScale,
    ) -> Result<(Epoch, Self), Error> {
        let line = match lines.next() {
            Some(l) => l,
            _ => return Err(Error::EopMissing1stLine),
        };
        let (epoch, rem) = line.split_at(23);
        let (xp, rem) = rem.split_at(19);
        let (dxp, ddxp) = rem.split_at(19);

        let line = match lines.next() {
            Some(l) => l,
            _ => return Err(Error::EopMissing2ndLine),
        };
        let (_, rem) = line.split_at(23);
        let (yp, rem) = rem.split_at(19);
        let (dyp, ddyp) = rem.split_at(19);

        let line = match lines.next() {
            Some(l) => l,
            _ => return Err(Error::EopMissing3rdLine),
        };
        let (t_tm, rem) = line.split_at(23);
        let (dut, rem) = rem.split_at(19);
        let (ddut, dddut) = rem.split_at(19);

        let epoch = epoch::parse_in_timescale(epoch.trim(), ts)?;
        let x = (
            parse_f64(xp.trim()).unwrap_or(0.0_f64),
            parse_f64(dxp.trim()).unwrap_or(0.0_f64),
            parse_f64(ddxp.trim()).unwrap_or(0.0_f64),
        );
        let y = (
            parse_f64(yp.trim()).unwrap_or(0.0_f64),
            parse_f64(dyp.trim()).unwrap_or(0.0_f64),
            parse_f64(ddyp.trim()).unwrap_or(0.0_f64),
        );
        let t_tm = parse_f64(t_tm.trim()).unwrap_or(0.0_f64);
        let delta_ut1 = (
            parse_f64(dut.trim()).unwrap_or(0.0_f64),
            parse_f64(ddut.trim()).unwrap_or(0.0_f64),
            parse_f64(dddut.trim()).unwrap_or(0.0_f64),
        );

        Ok((
            epoch,
            Self {
                x,
                y,
                t_tm: t_tm as u32,
                delta_ut1,
            },
        ))
    }
}
