//! Stateful, pull-based observation input. Scientific recovery belongs to callers.
use super::{
    parsing::{descriptor_fields, parse_descriptor, parse_observations},
    ClockObservation, EpochFlag, ObsContext, ObsKey, Observations,
};
use crate::{
    hatanaka::DecompressorExpert,
    prelude::{Constellation, Epoch, Header, Observable, ParsingError, TimeScale, Version, SV},
    types::Type,
    utils::validate_ascii,
};
use std::{
    collections::{BTreeSet, VecDeque},
    io::{BufRead, BufReader, Read},
    iter::FusedIterator,
    ops::Range,
    str::FromStr,
};

/// Coordinates describe text consumed by the reader, never gzip file offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCoordinate {
    InputText,
    CrinexText,
    GzipDecodedText,
    GzipDecodedCrinexText,
}

/// Input provenance. Lines are one-based and ranges are half-open.
/// Multiple recovered CRINEX lines may share one compressed-input span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObsSourceSpan {
    /// Zero-based observation/event block index, including rejected blocks.
    /// Independent comments do not occupy block indices. A terminal failure
    /// before a block start is established may also have no block index.
    pub block_index: Option<usize>,
    pub lines: Range<usize>,
    pub body_lines: Range<usize>,
    /// Byte offsets within the supplied text reader, starting before the header.
    pub bytes: Range<u64>,
    pub coordinate: SourceCoordinate,
}

/// Whether input has ended, failed, or could still be read. EOF does not certify
/// scientific usability or imply that no rejected blocks were encountered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadStatus {
    Reading,
    EndOfInput,
    Failed,
}

/// Memory limits apply to individual lines, blocks and the initial header.
#[derive(Debug, Clone, Copy)]
pub struct ObservationReaderOptions {
    pub max_line_bytes: usize,
    pub max_block_bytes: usize,
}

impl Default for ObservationReaderOptions {
    fn default() -> Self {
        Self {
            max_line_bytes: 1024 * 1024,
            max_block_bytes: 16 * 1024 * 1024,
        }
    }
}

/// A special event with optional timestamp and uncompressed special records.
/// External events require a timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialEvent {
    AntennaMoving,
    NewSiteOccupation,
    HeaderInformation,
    External,
}

/// A numeric field from a flag-6 record, not an observation sample.
/// The observable identifies its column; phase-column values are carrier cycles.
/// Other columns are retained as reported, without scientific interpretation.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleSlip {
    pub sv: SV,
    pub observable: Observable,
    pub value: f64,
}

/// Valid events are data, not decoding failures. Header snapshots are emitted
/// only when state changes; subsequent items use the new header revision.
#[derive(Debug, Clone, PartialEq)]
pub enum ObsEvent {
    Special {
        epoch: Option<Epoch>,
        kind: SpecialEvent,
        records: Vec<String>,
        header_update: Option<Box<Header>>,
    },
    CycleSlip {
        epoch: Epoch,
        clock: Option<ClockObservation>,
        slips: Vec<CycleSlip>,
        satellites: Vec<SV>,
    },
}

/// Owned block output. Rejected blocks contain no partial observation samples.
#[derive(Debug)]
pub enum ObsOutcome {
    Observation {
        key: ObsKey,
        data: Observations,
        /// Satellites in source order, including rows containing only blanks.
        /// Absent signal entries for these satellites are missing measurements.
        satellites: Vec<SV>,
    },
    Event(ObsEvent),
    /// The next block boundary and decoder state remain reliable. This says
    /// nothing about whether a scientific application may ignore the loss.
    Rejected {
        error: ParsingError,
        context: ObsContext,
    },
    /// Full input comment line, without line terminator. Comments inside events
    /// belong to the event records and are not emitted separately. Comments
    /// inside an observation block follow that item, with their exact source
    /// spans retained; decode error line numbers include those comment lines.
    Comment(String),
}

#[derive(Debug)]
pub struct ObsItem {
    pub source: ObsSourceSpan,
    /// Revision zero is the initial header. An update event carries its new revision.
    pub header_revision: u64,
    pub outcome: ObsOutcome,
}

/// A terminal error. The reader emits it once, becomes Failed and never reads
/// again. Previously yielded items remain owned by the caller.
#[derive(Debug, thiserror::Error)]
#[error("{cause}")]
pub struct ObsReadError {
    pub source: Option<Box<ObsSourceSpan>>,
    pub context: ObsContext,
    #[source]
    pub cause: ParsingError,
}

/// Explicit duplicate handling for whole-input collection. Preserve retains
/// every block in source order, including identical and conflicting duplicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObsDuplicatePolicy {
    Preserve,
    Reject,
}

/// A whole-input collection using the same iterator. Events, revisions, source
/// positions, comments and rejected blocks are retained in order.
#[derive(Debug)]
pub struct ObsCollection {
    pub initial_header: Header,
    pub initial_header_revision: u64,
    pub items: Vec<ObsItem>,
    pub status: ReadStatus,
}

/// Collection failed after producing the retained prefix. On a duplicate-policy
/// rejection the offending item is also retained, and status remains Reading.
#[derive(Debug, thiserror::Error)]
#[error("{error}")]
pub struct ObsCollectionError {
    pub partial: Box<ObsCollection>,
    #[source]
    pub error: ObsReadError,
}

struct Line {
    text: String,
    source: ObsSourceSpan,
}

/// A synchronous observation stream. It owns framing, header state and CRINEX
/// kernels; callers own filtering, duplicate policy and scientific recovery.
///
/// Supports RINEX 2/3/4 OBS through 4.02 and CRINEX 1.0/3.0. CRINEX 3.1
/// time extensions are not decoded and its header is rejected before body
/// decoding. Each item owns its data and source span. Observation fields retain
/// their encoded values, including negatives and zeroes. Supported metadata is
/// associated with the item's header revision; decoding applies no scientific
/// corrections. Flag-6 cycle-slip records are emitted as events.
///
/// A rejected block permits further reads only when its boundary and decoder
/// state remain known. A terminal error is emitted once and fuses the iterator.
/// Exhausting the iterator verifies EOF; stopping early leaves the unread tail
/// unverified. With the `flate2` feature, `from_gzip` also verifies gzip trailers.
///
/// Header updates are atomic. Unsupported state-changing records are terminal
/// errors. [crate::Rinex] reuses this reader but cannot retain header revisions
/// in its single-header observation map, so it rejects header changes.
/// Marker updates accept standard marker types and DOMES (`DDDDDMDDD`) numbers;
/// custom marker types and other number formats are terminal update errors.
/// Initial headers retain the metadata fields implemented by [Header];
/// unimplemented initial records are not retained.
/// Updates to scale factors, phase shifts and GLONASS frequency slots are
/// unsupported. Retain the initial header and update-event snapshots when
/// storing items from multiple revisions.
pub struct ObservationReader<R: BufRead> {
    input: R,
    header: Header,
    options: ObservationReaderOptions,
    header_lines: usize,
    input_line: usize,
    input_offset: u64,
    coordinate: SourceCoordinate,
    revision: u64,
    block_index: usize,
    status: ReadStatus,
    pending: Option<Line>,
    decoded: VecDeque<Line>,
    comments: VecDeque<ObsItem>,
    decompressor: Option<DecompressorExpert<5>>,
    compressed_start: Option<ObsSourceSpan>,
    output: Vec<u8>,
}

impl<R: BufRead> std::fmt::Debug for ObservationReader<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObservationReader")
            .field("header_revision", &self.revision)
            .field("next_block_index", &self.block_index)
            .field("input_line", &self.input_line)
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}

impl<R: BufRead> ObservationReader<R> {
    /// Consume the initial header and prepare to read observation/event blocks.
    pub fn new(input: R) -> Result<Self, ObsReadError> {
        Self::with_options(input, ObservationReaderOptions::default())
    }

    /// Apply byte limits to each input line, logical block and initial header.
    /// Limits must be nonzero, and the block limit must cover the line limit.
    pub fn with_options(input: R, options: ObservationReaderOptions) -> Result<Self, ObsReadError> {
        if options.max_line_bytes == 0 || options.max_block_bytes < options.max_line_bytes {
            return Err(terminal(
                None,
                ObsContext::default(),
                ParsingError::ObsStream("invalid reader limits".into()),
            ));
        }
        let mut reader = Self::init(input, options);
        let mut text = String::new();
        loop {
            let line = reader.input_line()?.ok_or_else(|| {
                terminal(
                    None,
                    ObsContext::default(),
                    ParsingError::ObsStream("missing END OF HEADER".into()),
                )
            })?;
            validate_ascii(&line.text, line.source.lines.start)
                .map_err(|e| terminal(Some(line.source.clone()), ObsContext::default(), e))?;
            match label(&line.text) {
                Some("APPROX POSITION XYZ" | "ANTENNA: DELTA H/E/N" | "ANTENNA: DELTA X/Y/Z") => {
                    finite_fields(&line.text[..60], 3).map_err(|e| {
                        terminal(Some(line.source.clone()), ObsContext::default(), e)
                    })?;
                },
                Some("INTERVAL") => {
                    finite_fields(&line.text[..20], 1).map_err(|e| {
                        terminal(Some(line.source.clone()), ObsContext::default(), e)
                    })?;
                },
                _ => {},
            }
            let end = label(&line.text) == Some("END OF HEADER");
            if text.len() + line.text.len() + 1 > options.max_block_bytes {
                return Err(terminal(
                    Some(line.source),
                    ObsContext::default(),
                    ParsingError::ObsStream("header exceeds byte limit".into()),
                ));
            }
            text.push_str(&line.text);
            text.push('\n');
            if end {
                break;
            }
        }
        let header = Header::parse(&mut BufReader::new(text.as_bytes()))
            .map_err(|e| terminal(None, ObsContext::default(), e))?;
        reader.header_lines = reader.input_line;
        reader.set_header(header)?;
        timescale(&reader.header).map_err(|e| terminal(None, ObsContext::default(), e))?;
        Ok(reader)
    }

    pub(crate) fn from_header(
        input: R,
        header: Header,
        options: ObservationReaderOptions,
    ) -> Result<Self, ObsReadError> {
        let mut reader = Self::init(input, options);
        reader.set_header(header)?;
        timescale(&reader.header).map_err(|e| terminal(None, ObsContext::default(), e))?;
        Ok(reader)
    }

    fn init(input: R, options: ObservationReaderOptions) -> Self {
        Self {
            input,
            header: Header::basic_obs(),
            options,
            header_lines: 0,
            input_line: 0,
            input_offset: 0,
            coordinate: SourceCoordinate::InputText,
            revision: 0,
            block_index: 0,
            status: ReadStatus::Reading,
            pending: None,
            decoded: VecDeque::new(),
            comments: VecDeque::new(),
            decompressor: None,
            compressed_start: None,
            output: Vec::new(),
        }
    }

    fn set_header(&mut self, header: Header) -> Result<(), ObsReadError> {
        if header.rinex_type != Type::ObservationData
            || header.obs.is_none()
            || !(2..=4).contains(&header.version.major)
            || header.version > Version::new(4, 2)
        {
            return Err(terminal(
                None,
                ObsContext::default(),
                ParsingError::ObsStream("expected supported RINEX 2/3/4 observation header".into()),
            ));
        }
        if let Some(crinex) = header.obs.as_ref().and_then(|h| h.crinex.as_ref()) {
            if !matches!(
                crinex.version,
                Version {
                    major: 1 | 3,
                    minor: 0
                }
            ) {
                return Err(terminal(
                    None,
                    ObsContext::default(),
                    ParsingError::ObsStream(format!(
                        "unsupported CRINEX version {}; supported versions are 1.0 and 3.0",
                        crinex.version
                    )),
                ));
            }
            self.coordinate = SourceCoordinate::CrinexText;
            self.decompressor = Some(DecompressorExpert::new(
                crinex.version.major == 3,
                header.constellation.unwrap_or_default(),
                header.obs.as_ref().unwrap().codes.clone(),
            ));
        }
        self.header = header;
        Ok(())
    }

    /// Current read-only header. Retain event snapshots when keeping data from
    /// multiple revisions; do not reinterpret old data using the final header.
    pub fn header(&self) -> &Header {
        &self.header
    }
    pub fn status(&self) -> ReadStatus {
        self.status
    }

    /// Collect remaining items in source order, retaining the prefix
    /// even if a terminal error occurs. A Rejected item is retained, not hidden.
    pub fn collect_records(
        mut self,
        policy: ObsDuplicatePolicy,
    ) -> Result<ObsCollection, ObsCollectionError> {
        let mut collection = ObsCollection {
            initial_header: self.header.clone(),
            initial_header_revision: self.revision,
            items: Vec::new(),
            status: self.status,
        };
        let mut keys = BTreeSet::new();
        while let Some(next) = self.next() {
            match next {
                Err(error) => {
                    collection.status = self.status;
                    return Err(ObsCollectionError {
                        partial: Box::new(collection),
                        error,
                    });
                },
                Ok(item) => {
                    let duplicate = match &item.outcome {
                        ObsOutcome::Observation { key, .. }
                            if policy == ObsDuplicatePolicy::Reject =>
                        {
                            !keys.insert(*key)
                        },
                        _ => false,
                    };
                    let source = item.source.clone();
                    collection.items.push(item);
                    if duplicate {
                        return Err(ObsCollectionError {
                            partial: Box::new(collection),
                            error: terminal(
                                Some(source),
                                ObsContext::default(),
                                ParsingError::ObsStream("duplicate observation key".into()),
                            ),
                        });
                    }
                },
            }
        }
        collection.status = self.status;
        Ok(collection)
    }

    fn input_line(&mut self) -> Result<Option<Line>, ObsReadError> {
        let mut bytes = Vec::new();
        let start = self.input_offset;
        let result = self
            .input
            .by_ref()
            .take(self.options.max_line_bytes as u64 + 1)
            .read_until(b'\n', &mut bytes);
        self.input_offset += bytes.len() as u64;
        let source = ObsSourceSpan {
            block_index: None,
            lines: self.input_line + 1..self.input_line + 2,
            body_lines: self.input_line.saturating_sub(self.header_lines) + 1
                ..self.input_line.saturating_sub(self.header_lines) + 2,
            bytes: start..self.input_offset,
            coordinate: self.coordinate,
        };
        let count =
            result.map_err(|e| terminal(Some(source.clone()), ObsContext::default(), e.into()))?;
        if count == 0 {
            return Ok(None);
        }
        self.input_line += 1;
        if count > self.options.max_line_bytes {
            return Err(terminal(
                Some(source),
                ObsContext::default(),
                ParsingError::ObsStream("line exceeds byte limit".into()),
            ));
        }
        let text = String::from_utf8(bytes).map_err(|e| {
            terminal(
                Some(source.clone()),
                ObsContext::default(),
                std::io::Error::new(std::io::ErrorKind::InvalidData, e).into(),
            )
        })?;
        Ok(Some(Line {
            text: text.trim_end_matches(['\r', '\n']).to_owned(),
            source,
        }))
    }

    fn next_line(&mut self) -> Result<Option<Line>, ObsReadError> {
        if let Some(line) = self.pending.take().or_else(|| self.decoded.pop_front()) {
            return Ok(Some(line));
        }
        loop {
            let Some(line) = self.input_line()? else {
                if self
                    .decompressor
                    .as_ref()
                    .is_some_and(|d| !d.at_epoch_boundary())
                {
                    return Err(terminal(
                        self.compressed_start.clone(),
                        ObsContext::default(),
                        ParsingError::ObsStream("truncated CRINEX epoch or event".into()),
                    ));
                }
                return Ok(None);
            };
            let Some(decompressor) = &mut self.decompressor else {
                return Ok(Some(line));
            };
            validate_ascii(&line.text, line.source.lines.start)
                .map_err(|e| terminal(Some(line.source.clone()), ObsContext::default(), e))?;
            if crate::is_rinex_comment(&line.text) && !decompressor.in_event() {
                return Ok(Some(line));
            }
            if decompressor.at_epoch_boundary() {
                self.compressed_start = Some(line.source.clone());
            }
            let size = decompressor.output_size(line.text.len());
            if size > self.options.max_block_bytes {
                return Err(terminal(
                    Some(line.source),
                    ObsContext::default(),
                    ParsingError::ObsStream("CRINEX output exceeds byte limit".into()),
                ));
            }
            self.output.resize(size, 0);
            let produced = decompressor
                .decompress(&line.text, line.text.len(), &mut self.output, size)
                .map_err(|e| {
                    terminal(
                        Some(line.source.clone()),
                        ObsContext::default(),
                        ParsingError::CRINEX(e),
                    )
                })?;
            if produced == 0 {
                continue;
            }
            let recovered = std::str::from_utf8(&self.output[..produced]).map_err(|_| {
                terminal(
                    Some(line.source.clone()),
                    ObsContext::default(),
                    ParsingError::BadUtf8Crinex,
                )
            })?;
            let mut source = self
                .compressed_start
                .take()
                .unwrap_or_else(|| line.source.clone());
            extend(&mut source, &line.source);
            for text in recovered.trim_end_matches(['\r', '\n']).split('\n') {
                self.decoded.push_back(Line {
                    text: text.trim_end_matches('\r').to_owned(),
                    source: source.clone(),
                });
            }
            if let Some(line) = self.decoded.pop_front() {
                return Ok(Some(line));
            }
        }
    }

    fn read_item(&mut self) -> Result<Option<ObsItem>, ObsReadError> {
        if let Some(comment) = self.comments.pop_front() {
            return Ok(Some(comment));
        }
        let first = loop {
            let Some(line) = self.next_line()? else {
                self.status = ReadStatus::EndOfInput;
                return Ok(None);
            };
            if crate::is_rinex_comment(&line.text) {
                validate_ascii(&line.text, line.source.lines.start)
                    .map_err(|e| terminal(Some(line.source.clone()), ObsContext::default(), e))?;
                return Ok(Some(ObsItem {
                    source: line.source,
                    header_revision: self.revision,
                    outcome: ObsOutcome::Comment(line.text),
                }));
            }
            if !line.text.trim().is_empty() {
                break line;
            }
        };
        let mut source = first.source.clone();
        source.block_index = Some(self.block_index);
        self.block_index += 1;
        let mut context = ObsContext::default();
        let ts = timescale(&self.header).map_err(|e| terminal(Some(source.clone()), context, e))?;
        let descriptor = parse_descriptor(&self.header, &first.text, ts, &mut context);
        let (flag, count) = match descriptor_fields(&first.text, self.header.version) {
            Ok(fields) => fields,
            // Modern markers preserve framing even if the satellite count is
            // invalid, provided the confirmed flag cannot change header state.
            Err(_)
                if self.header.version.major >= 3
                    && matches!(
                        context.flag,
                        Some(EpochFlag::Ok | EpochFlag::PowerFailure | EpochFlag::CycleSlip)
                    ) =>
            {
                (context.flag.unwrap(), 0)
            },
            Err(error) => {
                return Err(terminal(
                    Some(source),
                    context,
                    descriptor.err().unwrap_or(error),
                ))
            },
        };
        let special = matches!(
            flag,
            EpochFlag::AntennaBeingMoved
                | EpochFlag::NewSiteOccupation
                | EpochFlag::HeaderInformationFollows
                | EpochFlag::ExternalEvent
        );
        if special && descriptor.is_err() {
            return Err(terminal(Some(source), context, descriptor.err().unwrap()));
        }
        let mut text = first.text.clone();
        text.push('\n');
        let mut records = Vec::new();
        if special {
            for _ in 0..count {
                let line = self.block_line(&source, context)?.ok_or_else(|| {
                    terminal(
                        Some(source.clone()),
                        context,
                        ParsingError::ObsStream("truncated special event".into()),
                    )
                })?;
                extend(&mut source, &line.source);
                validate_ascii(&line.text, line.source.lines.start)
                    .map_err(|e| terminal(Some(source.clone()), context, e))?;
                records.push(line.text.clone());
                append(&mut text, &line.text, self.options.max_block_bytes)
                    .map_err(|e| terminal(Some(source.clone()), context, e))?;
            }
        } else if self.header.version.major >= 3 {
            loop {
                // CRINEX itself knows when the declared epoch is complete. Do
                // not consume or decode a following compressed epoch merely
                // to delimit the current, already complete block.
                if self
                    .decompressor
                    .as_ref()
                    .is_some_and(|d| d.at_epoch_boundary())
                    && self.decoded.is_empty()
                {
                    break;
                }
                let Some(line) = self.block_line(&source, context)? else {
                    break;
                };
                if line.text.starts_with('>') {
                    self.pending = Some(line);
                    break;
                }
                extend(&mut source, &line.source);
                self.check_span(&source, context)?;
                reject_hidden_update(&line.text)
                    .map_err(|e| terminal(Some(source.clone()), context, e))?;
                if crate::is_rinex_comment(&line.text) {
                    validate_ascii(&line.text, line.source.lines.start)
                        .map_err(|e| terminal(Some(line.source.clone()), context, e))?;
                    append(&mut text, &line.text, self.options.max_block_bytes)
                        .map_err(|e| terminal(Some(source.clone()), context, e))?;
                    self.comments.push_back(ObsItem {
                        source: line.source,
                        header_revision: self.revision,
                        outcome: ObsOutcome::Comment(line.text),
                    });
                } else {
                    append(&mut text, &line.text, self.options.max_block_bytes)
                        .map_err(|e| terminal(Some(source.clone()), context, e))?;
                }
            }
        } else {
            let extra = count.div_ceil(12).saturating_sub(1);
            for _ in 0..extra {
                if !self.append_v2_line(&mut text, &mut source, context)? {
                    break;
                }
            }
            let mut satellites = String::new();
            for (index, line) in text
                .lines()
                .filter(|line| !crate::is_rinex_comment(line))
                .enumerate()
            {
                if index > extra {
                    break;
                }
                satellites.push_str(line.get(32..line.len().min(68)).unwrap_or(""));
            }
            let specs = self.header.obs.as_ref().unwrap();
            let mut rows = 0;
            for index in 0..count {
                let name = satellites
                    .get(index * 3..index * 3 + 3)
                    .ok_or_else(|| {
                        terminal(
                            Some(source.clone()),
                            context,
                            ParsingError::BadV2SatellitesDescription,
                        )
                    })?
                    .trim();
                let sv = SV::from_str(name)
                    .or_else(|_| {
                        let constellation = self
                            .header
                            .constellation
                            .filter(|c| *c != Constellation::Mixed)
                            .ok_or(ParsingError::BadV2SatellitesDescription)?;
                        Ok::<_, ParsingError>(SV {
                            constellation,
                            prn: name
                                .parse()
                                .map_err(|_| ParsingError::BadV2SatellitesDescription)?,
                        })
                    })
                    .map_err(|e| terminal(Some(source.clone()), context, e))?;
                let system = if sv.constellation.is_sbas() {
                    Constellation::SBAS
                } else {
                    sv.constellation
                };
                let codes = specs.codes.get(&system).ok_or_else(|| {
                    terminal(Some(source.clone()), context, ParsingError::BadObservable)
                })?;
                rows += codes.len().div_ceil(5);
            }
            for _ in 0..rows {
                if !self.append_v2_line(&mut text, &mut source, context)? {
                    break;
                }
            }
        }
        let descriptor = match descriptor {
            Ok(descriptor) => descriptor,
            Err(error) => {
                return Ok(Some(ObsItem {
                    source,
                    header_revision: self.revision,
                    outcome: ObsOutcome::Rejected { error, context },
                }))
            },
        };
        let flag = descriptor.flag;
        let outcome = if special {
            let kind = match flag {
                EpochFlag::AntennaBeingMoved => SpecialEvent::AntennaMoving,
                EpochFlag::NewSiteOccupation => SpecialEvent::NewSiteOccupation,
                EpochFlag::HeaderInformationFollows => SpecialEvent::HeaderInformation,
                _ => SpecialEvent::External,
            };
            if kind == SpecialEvent::NewSiteOccupation
                && !records.iter().any(|r| label(r) == Some("MARKER NAME"))
            {
                return Err(terminal(
                    Some(source),
                    context,
                    ParsingError::ObsHeaderUpdate("new site without MARKER NAME".into()),
                ));
            }
            let update = apply_update(&self.header, &records)
                .map_err(|e| terminal(Some(source.clone()), context, e))?;
            let header_update = if update != self.header {
                if let Some(decompressor) = &mut self.decompressor {
                    decompressor.update_observables(update.obs.as_ref().unwrap().codes.clone());
                }
                self.header = update;
                self.revision += 1;
                Some(Box::new(self.header.clone()))
            } else {
                None
            };
            ObsOutcome::Event(ObsEvent::Special {
                epoch: descriptor.epoch,
                kind,
                records,
                header_update,
            })
        } else {
            match parse_observations(&self.header, &text, &descriptor) {
                Err(error) => ObsOutcome::Rejected { error, context },
                Ok((data, satellites)) if flag == EpochFlag::CycleSlip => {
                    let mut slips = Vec::new();
                    for signal in data.signals {
                        slips.push(CycleSlip {
                            sv: signal.sv,
                            observable: signal.observable,
                            value: signal.value,
                        });
                    }
                    ObsOutcome::Event(ObsEvent::CycleSlip {
                        epoch: descriptor.epoch.unwrap(),
                        clock: data.clock,
                        slips,
                        satellites,
                    })
                },
                Ok((data, satellites)) => ObsOutcome::Observation {
                    key: ObsKey {
                        epoch: descriptor.epoch.unwrap(),
                        flag,
                    },
                    data,
                    satellites,
                },
            }
        };
        Ok(Some(ObsItem {
            source,
            header_revision: self.revision,
            outcome,
        }))
    }

    fn append_v2_line(
        &mut self,
        text: &mut String,
        source: &mut ObsSourceSpan,
        context: ObsContext,
    ) -> Result<bool, ObsReadError> {
        while let Some(line) = self.block_line(source, context)? {
            if super::parsing::is_new_epoch(&line.text, self.header.version) {
                self.pending = Some(line);
                return Ok(false);
            }
            extend(source, &line.source);
            self.check_span(source, context)?;
            reject_hidden_update(&line.text)
                .map_err(|e| terminal(Some(source.clone()), context, e))?;
            if crate::is_rinex_comment(&line.text) {
                validate_ascii(&line.text, line.source.lines.start)
                    .map_err(|e| terminal(Some(line.source.clone()), context, e))?;
                append(text, &line.text, self.options.max_block_bytes)
                    .map_err(|e| terminal(Some(source.clone()), context, e))?;
                self.comments.push_back(ObsItem {
                    source: line.source,
                    header_revision: self.revision,
                    outcome: ObsOutcome::Comment(line.text),
                });
                continue;
            }
            append(text, &line.text, self.options.max_block_bytes)
                .map_err(|e| terminal(Some(source.clone()), context, e))?;
            return Ok(true);
        }
        Ok(false)
    }

    fn block_line(
        &mut self,
        source: &ObsSourceSpan,
        context: ObsContext,
    ) -> Result<Option<Line>, ObsReadError> {
        self.next_line().map_err(|mut error| {
            error.context = context;
            if let Some(span) = &mut error.source {
                span.block_index = source.block_index;
            } else {
                error.source = Some(Box::new(source.clone()));
            }
            error
        })
    }

    fn check_span(&self, source: &ObsSourceSpan, context: ObsContext) -> Result<(), ObsReadError> {
        if source.bytes.end - source.bytes.start > self.options.max_block_bytes as u64 {
            return Err(terminal(
                Some(source.clone()),
                context,
                ParsingError::ObsStream("block including comments exceeds byte limit".into()),
            ));
        }
        Ok(())
    }
}

#[cfg(feature = "flate2")]
impl<R: Read> ObservationReader<BufReader<flate2::read::MultiGzDecoder<R>>> {
    /// Gzip CRC/trailer errors are terminal errors, not successful EOF.
    pub fn from_gzip(input: R) -> Result<Self, ObsReadError> {
        let mut reader = Self::new(BufReader::new(flate2::read::MultiGzDecoder::new(input)))
            .map_err(|mut error| {
                if let Some(source) = &mut error.source {
                    source.coordinate = if source.coordinate == SourceCoordinate::CrinexText {
                        SourceCoordinate::GzipDecodedCrinexText
                    } else {
                        SourceCoordinate::GzipDecodedText
                    };
                }
                error
            })?;
        reader.coordinate = if reader.decompressor.is_some() {
            SourceCoordinate::GzipDecodedCrinexText
        } else {
            SourceCoordinate::GzipDecodedText
        };
        Ok(reader)
    }
}

impl<R: BufRead> Iterator for ObservationReader<R> {
    type Item = Result<ObsItem, ObsReadError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.status != ReadStatus::Reading {
            return None;
        }
        match self.read_item() {
            Ok(item) => item.map(Ok),
            Err(error) => {
                self.status = ReadStatus::Failed;
                Some(Err(error))
            },
        }
    }
}
impl<R: BufRead> FusedIterator for ObservationReader<R> {}

fn terminal(
    source: Option<ObsSourceSpan>,
    context: ObsContext,
    cause: ParsingError,
) -> ObsReadError {
    ObsReadError {
        source: source.map(Box::new),
        context,
        cause,
    }
}
fn label(line: &str) -> Option<&str> {
    line.get(60..).map(str::trim).filter(|s| !s.is_empty())
}
fn extend(source: &mut ObsSourceSpan, next: &ObsSourceSpan) {
    source.lines.end = next.lines.end;
    source.body_lines.end = next.body_lines.end;
    source.bytes.end = next.bytes.end;
}
fn append(text: &mut String, line: &str, max: usize) -> Result<(), ParsingError> {
    if text.len() + line.len() + 1 > max {
        return Err(ParsingError::ObsStream("block exceeds byte limit".into()));
    }
    text.push_str(line);
    text.push('\n');
    Ok(())
}
fn timescale(header: &Header) -> Result<TimeScale, ParsingError> {
    let fields = header
        .obs
        .as_ref()
        .ok_or(ParsingError::BadObsBadTimescaleDefinition)?;
    if let (Some(first), Some(last)) = (fields.timeof_first_obs, fields.timeof_last_obs) {
        if first.time_scale != last.time_scale {
            return Err(ParsingError::BadObsBadTimescaleDefinition);
        }
    }
    fields
        .timeof_first_obs
        .or(fields.timeof_last_obs)
        .map(|t| t.time_scale)
        .ok_or(ParsingError::BadObsBadTimescaleDefinition)
}

fn reject_hidden_update(line: &str) -> Result<(), ParsingError> {
    if matches!(
        label(line),
        Some(
            "SYS / # / OBS TYPES"
                | "# / TYPES OF OBSERV"
                | "MARKER NAME"
                | "MARKER NUMBER"
                | "MARKER TYPE"
                | "APPROX POSITION XYZ"
                | "ANT # / TYPE"
                | "REC # / TYPE / VERS"
                | "ANTENNA: DELTA H/E/N"
                | "ANTENNA: DELTA X/Y/Z"
                | "RCV CLOCK OFFS APPL"
                | "TIME OF FIRST OBS"
                | "TIME OF LAST OBS"
                | "SYS / SCALE FACTOR"
                | "SYS / PHASE SHIFT"
                | "GLONASS SLOT / FRQ #"
                | "INTERVAL"
                | "SIGNAL STRENGTH UNIT"
        )
    ) {
        return Err(ParsingError::ObsStream(
            "header record inside observation block; decoder state cannot be guaranteed".into(),
        ));
    }
    Ok(())
}

fn apply_update(header: &Header, records: &[String]) -> Result<Header, ParsingError> {
    let mut text = String::new();
    for line in records {
        let marker = label(line).ok_or_else(|| {
            ParsingError::ObsHeaderUpdate("special record has no header label".into())
        })?;
        if marker == "COMMENT" {
            continue;
        }
        // Unsupported state-affecting records fail closed rather than being
        // silently ignored by the general, permissive initial-header parser.
        match marker {
            "MARKER NAME"
            | "OBSERVER / AGENCY"
            | "REC # / TYPE / VERS"
            | "ANT # / TYPE"
            | "TIME OF FIRST OBS"
            | "TIME OF LAST OBS"
            | "SYS / # / OBS TYPES"
            | "# / TYPES OF OBSERV"
            | "SYS / DCBS APPLIED"
            | "SYS / PCVS APPLIED"
            | "LEAP SECONDS"
            | "COSPAR NUMBER"
            | "PRN / # OF OBS"
            | "# OF SATELLITES" => {},
            "MARKER NUMBER" => {
                let number = line[..20].trim();
                if crate::marker::GeodeticMarker::default()
                    .with_number(number)
                    .number()
                    .as_deref()
                    != Some(number)
                {
                    return Err(ParsingError::ObsHeaderUpdate(
                        "unsupported MARKER NUMBER; expected a DOMES identifier".into(),
                    ));
                }
            },
            "MARKER TYPE" => {
                crate::marker::MarkerType::from_str(line[..20].trim())
                    .map_err(|_| ParsingError::ObsHeaderUpdate("unsupported marker type".into()))?;
            },
            "RCV CLOCK OFFS APPL" => {
                if !matches!(line[..6].trim(), "0" | "1") {
                    return Err(ParsingError::ObsHeaderUpdate(
                        "invalid clock-offset application flag".into(),
                    ));
                }
            },
            "INTERVAL" => {
                finite_fields(&line[..20], 1)?;
                if crate::parse_f64(line[..20].trim()).map_or(true, |v| v < 0.0) {
                    return Err(ParsingError::ObsHeaderUpdate(
                        "negative sampling interval".into(),
                    ));
                }
            },
            "APPROX POSITION XYZ" | "ANTENNA: DELTA H/E/N" | "ANTENNA: DELTA X/Y/Z" => {
                finite_fields(&line[..60], 3)?;
            },
            _ => {
                return Err(ParsingError::ObsHeaderUpdate(format!(
                    "unsupported record {marker}"
                )))
            },
        }
        text.push_str(line);
        text.push('\n');
    }
    let mut updated = header.parse_observation_update(&text)?;
    // Event records are not initial-header comments.
    updated.comments = header.comments.clone();
    timescale(&updated)?;
    Ok(updated)
}
fn finite_fields(content: &str, expected: usize) -> Result<(), ParsingError> {
    // Position and antenna records use 3F14.4, even when a value fills its
    // entire field and no whitespace separates the adjacent numbers.
    let width = if expected == 3 { 14 } else { content.len() };
    let values: Vec<_> = (0..expected)
        .filter_map(|index| content.get(index * width..(index + 1) * width))
        .collect();
    if values.len() != expected
        || values
            .iter()
            .any(|s| crate::parse_f64(s.trim()).map_or(true, |v| !v.is_finite()))
    {
        return Err(ParsingError::ObsHeaderUpdate(
            "invalid numeric header record".into(),
        ));
    }
    Ok(())
}
