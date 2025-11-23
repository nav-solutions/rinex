//! RINEX header formatting

use crate::{
    fmt_comment, fmt_rinex,
    header::Header,
    prelude::{Constellation, FormattingError},
    types::Type,
};

use std::io::{BufWriter, Write};

impl Header {
    /// Formats [Header] into [Write]able interface, using efficient buffering.
    pub fn format<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        const NUM_GLO_CHANNELS_PER_LINE: usize = 8;

        let major = self.version.major;

        if let Some(obs) = &self.obs {
            if let Some(crinex) = &obs.crinex {
                crinex.format(w)?;
            }
        }

        self.format_rinex_version(w)?;

        if self.program.is_some() || self.run_by.is_some() || self.date.is_some() {
            self.format_prog_runby(w)?;
        }

        if self.observer.is_some() || self.agency.is_some() {
            self.format_observer_agency(w)?;
        }

        self.format_comments(w)?;
        self.format_rinex_dependent(w)?;

        if let Some(rcvr) = &self.rcvr {
            rcvr.format(w)?;
        }

        if let Some(ant) = &self.rcvr_antenna {
            ant.format(w)?;
        }

        if let Some(marker) = &self.geodetic_marker {
            marker.format(w)?;
        }

        if let Some((x_ecef_m, y_ecef_m, z_ecef_m)) = self.rx_position {
            writeln!(
                w,
                "{}",
                fmt_rinex(
                    &format!("{:14.4}{:14.4}{:14.4}", x_ecef_m, y_ecef_m, z_ecef_m),
                    "APPROX POSITION XYZ"
                )
            )?;
        }

        self.format_sampling_interval(w)?;

        if let Some(leap) = self.leap {
            leap.format(w)?;
        }

        let num_channels = self.glo_channels.len();

        if num_channels > 0 {
            write!(w, "{:3} ", num_channels,)?;
        }

        let mut modulo = 0;
        for (nth, (sv, channel)) in self.glo_channels.iter().enumerate() {
            write!(w, "{:x} {:2} ", sv, channel,)?;

            if nth == NUM_GLO_CHANNELS_PER_LINE - 1 {
                write!(w, "GLONASS SLOT / FRQ #\n    ")?;
            } else {
                if (nth % NUM_GLO_CHANNELS_PER_LINE) == NUM_GLO_CHANNELS_PER_LINE - 1 {
                    write!(w, "GLONASS SLOT / FRQ #\n    ")?;
                }
            }

            modulo = nth % NUM_GLO_CHANNELS_PER_LINE;
        }

        if modulo > 0 && modulo != NUM_GLO_CHANNELS_PER_LINE - 1 {
            writeln!(
                w,
                "{:>width$}",
                "GLONASS SLOT / FRQ #",
                width = 79 - 9 - (modulo + 1) * 6
            )?;
        }

        // KB model
        for (index, (constellation, model)) in self.ionod_corrections.iter().enumerate() {
            if let Some(kb) = model.as_klobuchar() {
                if major == 2 && index == 0 {
                    kb.format_v2_header(w)?;
                } else if major == 3 {
                    kb.format_v3_header(w, *constellation)?;
                }
            } else if let Some(ng) = model.as_nequick_g() {
                if major == 3 {
                    ng.format_header(w, constellation)?;
                }
            }
        }

        //TODO
        // things that could be nice to squeeze in:
        // [+] SBAS detail (detailed vehicle identity)
        // [+] RINEX 3 -> 2 observables conversion (see OBS/V2/rovn as an example)

        // Conclusion
        writeln!(w, "{}", fmt_rinex("", "END OF HEADER"))?;
        Ok(())
    }

    /// Formats "RINEX VERSION / TYPE"
    fn format_rinex_version<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        let (major, minor) = (self.version.major, self.version.minor);

        match self.rinex_type {
            Type::NavigationData => match self.constellation {
                Some(Constellation::Glonass) => {
                    writeln!(
                        w,
                        "{}",
                        fmt_rinex(
                            &format!("{:6}.{:02}           G: GLONASS NAV DATA", major, minor),
                            "RINEX VERSION / TYPE"
                        )
                    )?;
                },
                Some(Constellation::Mixed) => {
                    writeln!(
                        w,
                        "{}",
                        fmt_rinex(
                            &format!(
                                "{:6}.{:02}           NAVIGATION DATA     MIXED",
                                major, minor
                            ),
                            "RINEX VERSION / TYPE"
                        )
                    )?;
                },
                Some(c) => {
                    writeln!(
                        w,
                        "{}",
                        fmt_rinex(
                            &format!(
                                "{:6}.{:02}           NAVIGATION DATA     {:<20E}",
                                major, minor, c
                            ),
                            "RINEX VERSION / TYPE"
                        )
                    )?;
                },
                _ => {
                    return Err(FormattingError::UndefinedConstellation);
                },
            },
            Type::ObservationData => match self.constellation {
                Some(Constellation::Mixed) => {
                    writeln!(
                        w,
                        "{}",
                        fmt_rinex(
                            &format!(
                                "{:6}.{:02}           OBSERVATION DATA    M (MIXED)",
                                major, minor,
                            ),
                            "RINEX VERSION / TYPE"
                        )
                    )?;
                },
                Some(c) => {
                    writeln!(
                        w,
                        "{}",
                        fmt_rinex(
                            &format!(
                                "{:6}.{:02}           OBSERVATION DATA    {:<20E}",
                                major, minor, c
                            ),
                            "RINEX VERSION / TYPE"
                        )
                    )?;
                },
                _ => {
                    return Err(FormattingError::UndefinedConstellation);
                },
            },
            Type::MeteoData => {
                writeln!(
                    w,
                    "{}",
                    fmt_rinex(
                        &format!("{:6}.{:02}           METEOROLOGICAL DATA", major, minor),
                        "RINEX VERSION / TYPE"
                    )
                )?;
            },
            Type::ClockData => {
                writeln!(
                    w,
                    "{}",
                    fmt_rinex(
                        &format!("{:6}.{:02}           CLOCK DATA", major, minor),
                        "RINEX VERSION / TYPE"
                    )
                )?;
            },
            Type::AntennaData => {}, // TODO (ANTEX)
        }

        Ok(())
    }

    /// Formats RINEX type dependent [Header] fields
    fn format_rinex_dependent<W: Write>(
        &self,
        w: &mut BufWriter<W>,
    ) -> Result<(), FormattingError> {
        if let Some(obs) = &self.obs {
            obs.format(w, self.version.major)
        } else if let Some(nav) = &self.nav {
            nav.format(w, self.version.major)
        } else if let Some(meteo) = &self.meteo {
            meteo.format(w)
        } else if let Some(clock) = &self.clock {
            clock.format(w)
        } else if let Some(antex) = &self.antex {
            antex.format(w)
        } else {
            Ok(()) // should not happen
        }
    }

    /// Formats "PGM / RUN BY / DATE"
    fn format_prog_runby<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        let mut string = if let Some(program) = &self.program {
            format!("{:<20}", program)
        } else {
            "                    ".to_string()
        };

        if let Some(runby) = &self.run_by {
            let formatted = format!("{:<20}", runby);
            string.push_str(&formatted);
        } else {
            string.push_str("                    ");
        };

        if let Some(date) = &self.date {
            string.push_str(date);
        } else {
            string.push_str("                    ");
        };

        // PGM / RUN BY / DATE
        writeln!(w, "{}", fmt_rinex(&string, "PGM / RUN BY / DATE"),)?;

        Ok(())
    }

    /// Formats "OBSERVER / AGENCY"
    fn format_observer_agency<W: Write>(
        &self,
        w: &mut BufWriter<W>,
    ) -> Result<(), FormattingError> {
        let mut string = if let Some(observer) = &self.observer {
            format!("{:<20}", observer)
        } else {
            "                    ".to_string()
        };

        if let Some(agency) = &self.agency {
            string.push_str(agency);
        } else {
            string.push_str("                    ");
        };

        writeln!(w, "{}", fmt_rinex(&string, "OBSERVER / AGENCY"),)?;

        Ok(())
    }

    /// Formats "INTERVAL"
    fn format_sampling_interval<W: Write>(
        &self,
        w: &mut BufWriter<W>,
    ) -> Result<(), FormattingError> {
        if let Some(interval) = &self.sampling_interval {
            writeln!(
                w,
                "{}",
                fmt_rinex(&format!("{:6}", interval.to_seconds()), "INTERVAL")
            )?;
        }
        Ok(())
    }

    /// Formats all comments
    fn format_comments<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        for comment in self.comments.iter() {
            writeln!(w, "{}", fmt_comment(comment))?;
        }
        Ok(())
    }
}
