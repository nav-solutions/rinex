use crate::{
    antex::{
        record::{is_new_epoch as is_new_antex_epoch, parse_antenna as parse_antex_antenna},
        Record as AntexRecord,
    },
    clock::{
        record::{is_new_epoch as is_new_clock_epoch, parse_epoch as parse_clock_epoch},
        ClockKey, ClockProfile, Record as ClockRecord,
    },
    is_rinex_comment,
    meteo::{
        is_new_epoch as is_new_meteo_epoch, parse_epoch as parse_meteo_epoch, Record as MeteoRecord,
    },
    navigation::{
        is_new_epoch as is_new_nav_epoch, parse_epoch as parse_nav_epoch, Record as NavRecord,
    },
    observation::{
        ObsEvent, ObsOutcome, ObservationReader, ObservationReaderOptions,
        Record as ObservationRecord,
    },
    prelude::{Epoch, Header, ParsingError, TimeScale},
    record::{Comments, ParsingDiagnostic, ParsingDiagnosticKind, Record},
    types::Type,
    utils::validate_ascii,
};

use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read},
};

#[cfg(feature = "log")]
use log::error;

impl Record {
    /// Parses [Record] section by consuming [Read]er entirely.
    /// This requires reference to [Header] that was just parsed by consuming [Read]er until this point.
    pub fn parse<R: Read>(
        header: &mut Header,
        reader: &mut BufReader<R>,
    ) -> Result<(Self, Comments), ParsingError> {
        Self::parse_record(header, reader, None)
    }

    /// Parses the body and reports recoverable OBS/NAV failures and duplicate
    /// keys. NAV and OBS duplicates retain the last successfully decoded record.
    /// Non-ASCII OBS/NAV blocks are omitted in full. Non-ASCII comments or
    /// compressed input, I/O failures and unrepresentable precision are fatal.
    /// Valid OBS events are retained in diagnostics; header changes and unknown
    /// OBS framing are fatal. Use [ObservationReader] to retain header revisions.
    pub fn parse_with_diagnostics<R: Read>(
        header: &mut Header,
        reader: &mut BufReader<R>,
    ) -> Result<(Self, Comments, Vec<ParsingDiagnostic>), ParsingError> {
        let mut diagnostics = Vec::new();
        let (record, comments) = Self::parse_record(header, reader, Some(&mut diagnostics))?;
        Ok((record, comments, diagnostics))
    }

    fn parse_record<R: Read>(
        header: &mut Header,
        reader: &mut BufReader<R>,
        mut diagnostics: Option<&mut Vec<ParsingDiagnostic>>,
    ) -> Result<(Self, Comments), ParsingError> {
        if header.rinex_type == Type::ObservationData {
            return Self::parse_observation_record(header, reader, diagnostics);
        }
        let mut input_line = 0;
        let mut record_line = 1;
        // eos reached: process pending buffer & exit
        let mut eos = false;

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
        let mut nav_previous_lines = diagnostics.as_ref().map(|_| BTreeMap::new());

        // MET
        let mut met_rec = MeteoRecord::new();

        // CLK
        let mut clk_rec = ClockRecord::new();

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
            if size > 0 {
                input_line += 1;
            }
            if size == 0 {
                // reached EOS
                // we might still have something to process prior exiting
                eos |= true;
            }

            if header.rinex_type != Type::NavigationData {
                validate_ascii(&line_buf, input_line)?;
            }

            // (special case) COMMENTS: store as is
            if is_rinex_comment(&line_buf) {
                validate_ascii(&line_buf, input_line)?;
                let comment = line_buf.split_at(60).0.trim_end();
                comment_content.push(comment.to_string());

                // skip parsing
                line_buf.clear();
                continue;
            }

            // (special case) COMMENTS: store as is
            if let Some(content) = line_buf.get(..60).filter(|_| line_buf.contains("COMMENT")) {
                let content = content.trim();
                if let Some(comments) = comments.get_mut(&comment_ts) {
                    comments.push(content.to_string());
                } else {
                    comments.insert(comment_ts, vec![content.to_string()]);
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
                            Ok((key, frame)) => {
                                let conflicting = diagnostics.as_ref().and_then(|_| {
                                    nav_rec.get(&key).map(|previous| previous != &frame)
                                });
                                nav_rec.insert(key, frame);
                                if let Some(lines) = nav_previous_lines.as_mut() {
                                    let previous_record_line = lines.insert(key, record_line);
                                    if let (
                                        Some(diagnostics),
                                        Some(conflicting),
                                        Some(previous_record_line),
                                    ) = (
                                        diagnostics.as_deref_mut(),
                                        conflicting,
                                        previous_record_line,
                                    ) {
                                        diagnostics.push(ParsingDiagnostic {
                                            record_line,
                                            observation_context: None,
                                            kind: ParsingDiagnosticKind::DuplicateNavigation {
                                                key,
                                                conflicting,
                                                previous_record_line,
                                            },
                                        });
                                    }
                                }
                                comment_ts = key.epoch;
                            },
                            Err(error) => {
                                if let Some(diagnostics) = diagnostics.as_deref_mut() {
                                    diagnostics.push(ParsingDiagnostic {
                                        record_line,
                                        observation_context: None,
                                        kind: ParsingDiagnosticKind::NavigationFailure(error),
                                    });
                                }
                            },
                        },
                        Type::ObservationData => unreachable!("OBS uses ObservationReader"),

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
            }

            // always stack new content
            if epoch_buf.is_empty() {
                record_line = input_line;
            }
            epoch_buf.push_str(&line_buf);

            if eos {
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
            Type::ObservationData => unreachable!("OBS uses ObservationReader"),
        };
        Ok((record, comments))
    }

    fn parse_observation_record<R: Read>(
        header: &mut Header,
        reader: &mut BufReader<R>,
        mut diagnostics: Option<&mut Vec<ParsingDiagnostic>>,
    ) -> Result<(Self, Comments), ParsingError> {
        let mut stream = ObservationReader::from_header(
            reader,
            header.clone(),
            ObservationReaderOptions::default(),
        )
        .map_err(|e| e.cause)?;
        let mut record = ObservationRecord::new();
        let mut comments = Comments::new();
        let mut comment_epoch = Epoch::default();
        for next in &mut stream {
            let item = next.map_err(|e| e.cause)?;
            let record_line = item.source.body_lines.start;
            match item.outcome {
                ObsOutcome::Observation { key, data, .. } => {
                    let conflicting = diagnostics
                        .as_ref()
                        .and_then(|_| record.get(&key).map(|previous| previous != &data));
                    record.insert(key, data);
                    if let (Some(out), Some(conflicting)) =
                        (diagnostics.as_deref_mut(), conflicting)
                    {
                        out.push(ParsingDiagnostic {
                            record_line,
                            observation_context: None,
                            kind: ParsingDiagnosticKind::DuplicateObservation { key, conflicting },
                        });
                    }
                    comment_epoch = key.epoch;
                },
                ObsOutcome::Rejected { error, context } => {
                    if matches!(error, ParsingError::EpochPrecision) {
                        return Err(error);
                    }
                    if let Some(out) = diagnostics.as_deref_mut() {
                        out.push(ParsingDiagnostic {
                            record_line,
                            observation_context: Some(context),
                            kind: ParsingDiagnosticKind::ObservationFailure(error),
                        });
                    }
                },
                ObsOutcome::Event(event) => {
                    if matches!(
                        &event,
                        ObsEvent::Special {
                            header_update: Some(_),
                            ..
                        }
                    ) {
                        return Err(ParsingError::ObsHeaderUpdate("Rinex has one header; use ObservationReader::collect_records to retain header revisions".into()));
                    }
                    if let Some(out) = diagnostics.as_deref_mut() {
                        out.push(ParsingDiagnostic {
                            record_line,
                            observation_context: None,
                            kind: ParsingDiagnosticKind::ObservationEvent(event),
                        });
                    }
                },
                ObsOutcome::Comment(line) => {
                    comments
                        .entry(comment_epoch)
                        .or_default()
                        .push(line[..60].trim_end().to_owned());
                },
            }
        }
        Ok((Record::ObsRecord(record), comments))
    }

    fn is_new_epoch(line: &str, header: &Header) -> bool {
        if is_rinex_comment(line) {
            return false;
        }
        match &header.rinex_type {
            Type::AntennaData => is_new_antex_epoch(line),
            Type::ClockData => is_new_clock_epoch(line),
            Type::NavigationData => is_new_nav_epoch(line, header.version),
            Type::ObservationData => unreachable!("OBS uses ObservationReader"),
            Type::MeteoData => is_new_meteo_epoch(line, header.version),
        }
    }
}
