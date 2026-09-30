use crate::{
    antex::{
        record::{is_new_epoch as is_new_antex_epoch, parse_antenna as parse_antex_antenna},
        Record as AntexRecord,
    },
    clock::{
        record::{is_new_epoch as is_new_clock_epoch, parse_epoch as parse_clock_epoch},
        ClockKey, ClockProfile, Record as ClockRecord,
    },
    epoch::parse_in_timescale,
    hatanaka::DecompressorExpert,
    is_rinex_comment,
    meteo::{
        is_new_epoch as is_new_meteo_epoch, parse_epoch as parse_meteo_epoch, Record as MeteoRecord,
    },
    navigation::{
        is_new_epoch as is_new_nav_epoch, parse_epoch as parse_nav_epoch,
        timescale as nav_timescale, NavDiagnosticKind, NavMessageType, NavParseDiagnostic,
        NavParseReport, Record as NavRecord,
    },
    observation::Observations,
    observation::{
        is_new_epoch as is_new_observation_epoch, parse_epoch as parse_observation_epoch,
        Record as ObservationRecord,
    },
    prelude::{Epoch, Header, ParsingError, TimeScale},
    record::{Comments, Record},
    types::Type,
};

use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read},
    str::from_utf8,
};

#[cfg(feature = "log")]
use log::error;

impl Record {
    /// Parses [Record] section by consuming [Read]er entirely.
    /// This requires reference to [Header] that was just parsed by consuming [Read]er until this point.
    /// Direct record parsing fails on rejected NAV records. Use [crate::Rinex::parse]
    /// to continue with an attached diagnostic report.
    pub fn parse<R: Read>(
        header: &mut Header,
        reader: &mut BufReader<R>,
    ) -> Result<(Self, Comments), ParsingError> {
        let (record, comments, _) = Self::parse_with_report(header, reader, 0, true)?;
        Ok((record, comments))
    }

    pub(crate) fn parse_with_report<R: Read>(
        header: &mut Header,
        reader: &mut BufReader<R>,
        header_lines: usize,
        strict_nav_fields: bool,
    ) -> Result<(Self, Comments, NavParseReport), ParsingError> {
        let mut report = NavParseReport::default();
        let mut file_line = header_lines;
        let mut record_line = header_lines + 1;
        // eos reached: process pending buffer & exit
        let mut eos = false;
        let mut crinex_error = false;

        // current line storage
        let mut line_buf = String::with_capacity(128);

        // epoch storage
        let mut epoch_buf = String::with_capacity(1024);

        // comments management
        let mut comments: Comments = Comments::new();
        let mut comment_ts = Epoch::default();
        let mut comment_content = Vec::<String>::with_capacity(4);

        // ANTEX
        let mut atx_rec = AntexRecord::new();

        // NAV
        let mut nav_rec = NavRecord::new();

        // OBS
        let mut obs_rec = ObservationRecord::new();
        let mut observations = Observations::default();

        // CRINEX case
        const CRINEX_BUF_SIZE: usize = 1024;
        let mut buf = [0; CRINEX_BUF_SIZE];

        let mut is_crinex = false;
        let mut crinex_v3 = false;
        let mut gnss_observables = Default::default();

        if let Some(obs) = &header.obs {
            if let Some(crinex) = &obs.crinex {
                is_crinex = true;
                crinex_v3 = crinex.version.major > 2;
            }
            gnss_observables = obs.codes.clone();
        }

        // Build a decompressor, that we deployed if needed.
        // These parameters are compatible with historical RNX2CRX tool.
        let mut decompressor = DecompressorExpert::<5>::new(
            crinex_v3,
            header.constellation.unwrap_or_default(),
            gnss_observables,
        );

        // MET
        let mut met_rec = MeteoRecord::new();

        // CLK
        let mut clk_rec = ClockRecord::new();

        // OBSERVATION case: timescale is either defined by
        // [+] TIME OF FIRST header field
        // [+] TIME OF LAST header field (flexibility, actually invalid according to specs)
        let mut obs_ts = TimeScale::default();

        if let Some(obs) = &header.obs {
            if let Some(t) = obs.timeof_first_obs {
                obs_ts = t.time_scale;
            } else {
                let t = obs
                    .timeof_last_obs
                    .ok_or(ParsingError::BadObsBadTimescaleDefinition)?;
                obs_ts = t.time_scale;
            }
        }

        // Clock RINEX TimeScale definition.
        // Modern revisions define it in header directly.
        // Old revisions are once again badly defined and most likely not thought out.
        //  + We default to GPST to "match" the case where this file is multi constellation
        //   and it seems that clocks steered to GPST is the most common case.
        //   For example NASA/CDDIS.com
        //  + In mono constellation, we adapt to that timescale.
        let mut clk_ts = TimeScale::GPST;
        if let Some(clk) = &header.clock {
            if let Some(ts) = clk.timescale {
                clk_ts = ts;
            } else {
                if let Some(constellation) = &header.constellation {
                    if let Some(ts) = constellation.timescale() {
                        clk_ts = ts;
                    }
                }
            }
        }

        // Iterate and consume, one line at a time
        loop {
            let size = reader.read_line(&mut line_buf)?;
            if size == 0 {
                // reached EOS
                // we might still have something to process prior exiting
                eos |= true;
            } else {
                file_line += 1;
            }

            if matches!(header.rinex_type, Type::NavigationData)
                && line_buf.starts_with('>')
                && !is_new_nav_epoch(&line_buf, header.version)
            {
                return Err(ParsingError::NavMsgType);
            }

            // (special case) COMMENTS: store as is
            if is_rinex_comment(&line_buf) {
                let comment = line_buf.split_at(60).0.trim_end();
                comment_content.push(comment.to_string());

                // skip parsing
                line_buf.clear();
                continue;
            }

            // (special case) COMMENTS: store as is
            if line_buf.contains("COMMENT") {
                let content = line_buf.split_at(60).0.trim();
                if let Some(comments) = comments.get_mut(&comment_ts) {
                    comments.push(content.to_string());
                } else {
                    comments.insert(comment_ts, vec![content.to_string()]);
                }
            }

            // CRINEX special case:
            // - apply decompression algorithm prior moving forward
            // - decompress new pending line, which may recover several lines (in old V1 format)
            if is_crinex {
                let line_len = line_buf.len();

                // catch errors nicely, simply log them
                // it is normal to abort on final line for example
                match decompressor.decompress(&line_buf, line_len, &mut buf, CRINEX_BUF_SIZE) {
                    Ok(size) => {
                        if size > 0 {
                            // clear and overwrite pending content with recovered content
                            // we should have valid ASCII UTF-8 at all times, at this point
                            let recovered =
                                from_utf8(&buf[..size]).map_err(|_| ParsingError::BadUtf8Crinex)?;

                            line_buf.clear();
                            line_buf = recovered.to_string();
                            line_buf.push('\n');
                        } else if !eos {
                            // line consumed by the decompressor (epoch or clock
                            // description) without anything to publish yet:
                            // the compressed text must not reach the parser.
                            line_buf.clear();
                            continue;
                        }
                    },
                    Err(_) => {
                        // We wind up here on the final line which is empty
                        // and the decompressor reports that the next epoch is too short,
                        // which is normal. But will not help create a resynchronizable decompressor.
                        //
                        // Hatanaka decompression cannot recover from corrupt content
                        // in either revisions. It would be possible to recover if we
                        // modified the Epoch synchronization __but__ develop a very
                        // smart epoch detector, which does not seem easy to do.
                        crinex_error = true;
                    },
                }
            }

            let mut new_epoch = false;

            // we're trying to stack a complete epoch
            // that we process once a new one appears
            if epoch_buf.len() > 0 {
                new_epoch = Self::is_new_epoch(&line_buf, &header);

                // trick to force attempt on last iteration
                new_epoch |= eos;

                if new_epoch {
                    // new epoch appearing: process what we have buffered
                    // parsing method is format dependent
                    //println!("***MATCH***");

                    match &header.rinex_type {
                        Type::NavigationData => match parse_nav_epoch(&header, &epoch_buf) {
                            Ok((k, v)) => {
                                nav_rec.insert(k, v);
                                comment_ts = k.epoch;
                            },
                            Err(
                                err @ (ParsingError::NavOrbitField { .. }
                                | ParsingError::NavClockField { .. }),
                            ) => {
                                if strict_nav_fields {
                                    return Err(err);
                                }
                                report.rejected_records += 1;
                                report.diagnostics.push(nav_diagnostic(
                                    header,
                                    &epoch_buf,
                                    record_line,
                                    NavDiagnosticKind::InvalidField,
                                    &err,
                                ));
                            },
                            Err(
                                err @ (ParsingError::NoNavigationDefinition
                                | ParsingError::NavMsgType
                                | ParsingError::NavInvalidTimescale),
                            ) => {
                                if strict_nav_fields {
                                    return Err(err);
                                }
                                report.unsupported_records += 1;
                                report.diagnostics.push(nav_diagnostic(
                                    header,
                                    &epoch_buf,
                                    record_line,
                                    NavDiagnosticKind::UnsupportedMessage,
                                    &err,
                                ));
                            },
                            Err(err) => return Err(err),
                        },
                        Type::ObservationData => {
                            match parse_observation_epoch(
                                header,
                                &epoch_buf,
                                obs_ts,
                                &mut observations,
                            ) {
                                Ok(key) => {
                                    //println!("key={:?}", key);
                                    obs_rec.insert(key, observations.clone());
                                    comment_ts = key.epoch; // for comments storage
                                },
                                #[cfg(feature = "log")]
                                Err(e) => {
                                    error!("parsing: {}", e);
                                },
                                #[cfg(not(feature = "log"))]
                                Err(_) => {},
                            }

                            observations.signals.clear(); // reset for next parsing (single alloc)
                        },

                        Type::MeteoData => {
                            if let Ok(items) = parse_meteo_epoch(header, &epoch_buf) {
                                for (k, v) in items.iter() {
                                    met_rec.insert(k.clone(), *v);
                                    comment_ts = k.epoch; // for comments storage
                                }
                            }
                        },

                        Type::ClockData => {
                            if let Ok((epoch, key, profile)) =
                                parse_clock_epoch(header.version, &epoch_buf, clk_ts)
                            {
                                if let Some(e) = clk_rec.get_mut(&epoch) {
                                    e.insert(key, profile);
                                } else {
                                    let mut inner: BTreeMap<ClockKey, ClockProfile> =
                                        BTreeMap::new();
                                    inner.insert(key, profile);
                                    clk_rec.insert(epoch, inner);
                                }
                                comment_ts = epoch; // for comments storage
                            }
                        },

                        Type::AntennaData => {
                            if let Ok((antenna, content)) = parse_antex_antenna(&epoch_buf) {
                                atx_rec.push((antenna, content));
                            }
                        },
                    }
                }
            }

            // clear on new epoch detection
            if new_epoch {
                epoch_buf.clear();
                record_line = file_line;
            }

            // always stack new content
            epoch_buf.push_str(&line_buf);

            if eos || crinex_error {
                break;
            }

            line_buf.clear(); // always clear newline buf
        } //loop

        // wrap content and exit
        let record = match &header.rinex_type {
            Type::AntennaData => Record::AntexRecord(atx_rec),
            Type::ClockData => Record::ClockRecord(clk_rec),
            Type::MeteoData => Record::MeteoRecord(met_rec),
            Type::NavigationData => Record::NavRecord(nav_rec),
            Type::ObservationData => Record::ObsRecord(obs_rec),
        };
        Ok((record, comments, report))
    }

    fn is_new_epoch(line: &str, header: &Header) -> bool {
        if is_rinex_comment(line) {
            return false;
        }
        match &header.rinex_type {
            Type::AntennaData => is_new_antex_epoch(line),
            Type::ClockData => is_new_clock_epoch(line),
            Type::NavigationData => is_new_nav_epoch(line, header.version),
            Type::ObservationData => is_new_observation_epoch(line, header.version),
            Type::MeteoData => is_new_meteo_epoch(line, header.version),
        }
    }
}

fn nav_diagnostic(
    header: &Header,
    content: &str,
    record_line: usize,
    kind: NavDiagnosticKind,
    error: &ParsingError,
) -> NavParseDiagnostic {
    let mut lines = content.lines();
    let first = lines.next().unwrap_or("");
    let v4 = first.starts_with('>');
    let (sv_text, date_text, message) = if v4 {
        let sv = first.get(6..10).unwrap_or("").trim();
        let message = first
            .get(10..)
            .and_then(|s| s.split_ascii_whitespace().next())
            .and_then(|s| s.parse::<NavMessageType>().ok());
        let date = lines.next().and_then(|s| s.get(4..23)).unwrap_or("");
        (sv, date, message)
    } else {
        let width = if header.version.major < 3 { 3 } else { 4 };
        (
            first.get(..width).unwrap_or("").trim(),
            first.get(width..width + 19).unwrap_or(""),
            Some(NavMessageType::LNAV),
        )
    };
    let sv = sv_text.parse::<crate::prelude::SV>().ok().or_else(|| {
        if header.version.major < 3 {
            let constellation = header.constellation?;
            format!("{:x}{:02}", constellation, sv_text)
                .parse::<crate::prelude::SV>()
                .ok()
        } else {
            None
        }
    });
    let epoch = sv
        .and_then(|sv| nav_timescale(sv.constellation).ok())
        .and_then(|ts| parse_in_timescale(date_text.trim(), ts).ok());
    let (field_line, slot, field, raw) = match error {
        ParsingError::NavOrbitField {
            line,
            slot,
            field,
            value,
            ..
        } => (
            Some(record_line + usize::from(v4) + line - 1),
            Some(*slot),
            Some(field.clone()),
            Some(value.clone()),
        ),
        ParsingError::NavClockField {
            line,
            slot,
            field,
            value,
            ..
        } => (
            Some(record_line + line - 1),
            Some(*slot),
            Some((*field).to_string()),
            Some(value.clone()),
        ),
        ParsingError::NavInvalidTimescale => (
            Some(record_line + 1),
            None,
            Some("timePair".to_string()),
            content
                .lines()
                .nth(1)
                .and_then(|s| s.get(24..28))
                .map(str::to_string),
        ),
        _ if kind == NavDiagnosticKind::UnsupportedMessage => {
            (None, None, None, Some(first.to_string()))
        },
        _ => (None, None, None, None),
    };
    NavParseDiagnostic {
        kind,
        record_line,
        field_line,
        slot,
        field,
        raw,
        sv,
        epoch,
        message,
        reason: error.to_string(),
    }
}
