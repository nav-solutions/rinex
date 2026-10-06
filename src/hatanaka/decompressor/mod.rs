//! CRINEX decompression module
use crate::{
    hatanaka::{Error, NumDiff, TextDiff},
    prelude::{Constellation, Observable, SV},
};

use std::{collections::HashMap, str::FromStr};

pub mod io;

use num_integer::div_ceil;

#[cfg(feature = "log")]
use log::{error, trace};

#[cfg(doc)]
use crate::{hatanaka::Compressor, prelude::Header};

#[cfg(doc)]
use std::io::Lines;

/// [Decompressor] is a structure to decompress CRINEX (compressed compacted RINEX)
/// into readable RINEX. It is scaled to operate according to the historical CRX2RNX tool,
/// which seems to limit itself to M=3 in the compression algorithm.
/// If you want complete control over the decompression algorithm, prefer [DecompressorExpert].
///
/// [Decompressor] implements the CRINEX decompression algorithm, following
/// the specifications written by Y. Hatanaka. Like RINEX, CRINEX (compact) RINEX
/// is a line based format (\n termination), this structures works on a line basis.
///
/// Although [Decompressor] is flexible and powerful, yet the CRINEX specifications
/// makes it hard to recover fatal stream corruption, currently this decompressor
/// does not tolerate them and would require further development and severe testing.
/// This is summarized in 3 cases:
/// - corruption of the number of satellites described in the epoch
/// - missing or invalid observable specifications
/// - missing or invalid constellation specifications
pub type Decompressor = DecompressorExpert<5>;

/// [State] of the Hatanaka decompression process.
#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub enum State {
    #[default]
    /// Gathering Epoch descriptor.
    Epoch,

    /// Gathering Clock offset, recovering complete epoch description.
    Clock,

    /// Observations gathering and recovering.
    Observation,
}

impl State {
    /// Minimal size of a valid [Epoch] description in V1 revision    
    /// - Timestamp: Year uses 2 digits
    /// - Flag
    /// - Numsat
    const MIN_COMPRESSED_EPOCH_SIZE_V1: usize = 17;
    const MIN_DECOMPRESSED_EPOCH_SIZE_V1: usize = 32;

    /// Room for the receiver clock offset in a V1 epoch description:
    /// F12.9 in columns 69-80, after the SV list padded to column 68.
    const V1_CLOCK_SIZE: usize = 12 + 68 - Self::MIN_DECOMPRESSED_EPOCH_SIZE_V1;

    /// Minimal size of a valid [Epoch] description in V3 revision  
    /// - >
    /// - Timestamp: Year uses 4 digits
    /// - Flag
    /// - Numsat
    const MIN_COMPRESSED_EPOCH_SIZE_V3: usize = 19;
    const MIN_DECOMPRESSED_EPOCH_SIZE_V3: usize = 35;

    /// Calculates number of bytes this state will forward to user
    fn size_to_produce(&self, v3: bool, numsat: usize, numobs: usize) -> usize {
        match self {
            // Epoch is recovered once Clock is recovered.
            // Because standard format says the clock data should be appended to epoch description
            // (in an inconvenient way, in V1 revision).
            Self::Clock => {
                if v3 {
                    Self::MIN_DECOMPRESSED_EPOCH_SIZE_V3
                } else {
                    let mut size = Self::MIN_DECOMPRESSED_EPOCH_SIZE_V1;
                    let num_extra = div_ceil(numsat, 12).saturating_sub(1);
                    size += num_extra * 17; // padding
                    size += numsat * 3; // formatted
                    size += Self::V1_CLOCK_SIZE; // possible clock offset
                    size
                }
            },
            Self::Observation => {
                if v3 {
                    3 + numobs * 16
                } else {
                    let mut size = 1;
                    size += numobs.saturating_sub(1); // separator
                    size += 15 * numobs; // formatted
                    let num_extra = div_ceil(numobs, 5).saturating_sub(1);
                    size += num_extra * 15; // padding
                    size
                }
            },
            // Other states do not generate any data
            // we need to consume lines to progress to states that actually produce something
            _ => 0,
        }
    }
}

/// [DecompressorExpert] is a structure to decompress CRINEX files.
///
/// The CRINEX file format supports mixing levels of compression in the same file, in
/// anticipation of a clever compressor that adapts the compression level to the data.
///
/// The actual diff-order is read from the file; as
/// long as it does not exceed M in this implementation, it will be decompressed correctly.
///
/// The sole member `M` is inherited from [NumDiff] and controls the maximum diff-order
/// for decompressing numerical data.
///
/// The historical RNX2CRX program and our implementation only use a level of 3.
/// This structure supports any M integer number.
pub struct DecompressorExpert<const M: usize> {
    /// Whether this is a V3 parser or not
    v3: bool,

    /// Constellation described by [Header]
    constellation: Constellation,

    /// Internal Finite [State] Machine.
    state: State,

    /// For internal logic: remains true until one epoch descriptor has been recovered.
    first_epoch: bool,

    /// pointers
    sv: SV,
    numsat: usize,  // total
    sv_ptr: usize,  // inside epoch
    numobs: usize,  // total
    obs_ptr: usize, // inside epoch

    /// [TextDiff] that works on entire Epoch line
    epoch_diff: TextDiff,

    /// Epoch descriptor, for single allocation
    epoch_descriptor: String,
    epoch_desc_len: usize, // for internal logic

    /// Missing observations of the line being decompressed
    blanks: Vec<bool>,
    /// Vehicles of the epoch being decompressed
    epoch_svs: Vec<SV>,

    /// [TextDiff] decompression kernel for observation flags
    flags_diff: HashMap<SV, TextDiff>,

    /// Numerical decompression kernel for clock data specifically.
    clock_diff: NumDiff<M>,

    /// Numerical decompression kernels, per SV and physics.
    obs_diff: HashMap<(SV, usize), NumDiff<M>>,

    /// [Observable]s specs for each [Constellation]
    gnss_observables: HashMap<Constellation, Vec<Observable>>,
}

impl<const M: usize> Default for DecompressorExpert<M> {
    fn default() -> Self {
        Self {
            v3: true,
            numsat: 0,
            sv_ptr: 0,
            numobs: 0,
            obs_ptr: 0,
            first_epoch: true,
            epoch_desc_len: 0,
            sv: Default::default(),
            state: Default::default(),
            constellation: Constellation::Mixed,
            epoch_diff: TextDiff::new(""),
            gnss_observables: HashMap::with_capacity(8), // cannot be initialized
            obs_diff: HashMap::with_capacity(8),         // cannot initialize yet
            flags_diff: HashMap::with_capacity(8),       // cannot initialize yet
            epoch_descriptor: String::with_capacity(256),
            blanks: Vec::with_capacity(64),
            epoch_svs: Vec::with_capacity(64),
            clock_diff: NumDiff::<M>::new(0, M),
        }
    }
}

impl<const M: usize> DecompressorExpert<M> {
    /// Minimal timestamp length in V1 revision
    const V1_TIMESTAMP_SIZE: usize = 24;
    const V1_NUMSAT_OFFSET: usize = Self::V1_TIMESTAMP_SIZE + 4;
    const V1_SAT_OFFSET: usize = Self::V1_NUMSAT_OFFSET + 3;
    /// Receiver clock offset position (column 69) in a V1 epoch description
    const V1_CLOCK_OFFSET: usize = 68;

    /// Minimal timestamp length in V3 revision
    const V3_TIMESTAMP_SIZE: usize = 26;
    const V3_NUMSAT_OFFSET: usize = Self::V3_TIMESTAMP_SIZE + 1 + 4;
    const V3_SAT_OFFSET: usize = Self::V3_NUMSAT_OFFSET + 9;

    /// Returns pointer offset to parse the i-th satellite in the epoch descriptor
    fn sv_slice_start(v3: bool, sat_index: usize) -> usize {
        let offset = if v3 {
            Self::V3_SAT_OFFSET
        } else {
            Self::V1_SAT_OFFSET
        };

        offset + sat_index * 3
    }

    /// Tries to return next satellite from epoch descriptor.
    fn next_satellite(&self) -> Option<SV> {
        let start = Self::sv_slice_start(self.v3, self.sv_ptr);
        let end = (start + 3).min(self.epoch_desc_len);

        let content = &self.epoch_descriptor[start..end].trim();

        // satellite parsing
        if let Ok(sv) = SV::from_str(content) {
            Some(sv)
        } else {
            // in old RINEX, single satellite system may omit the constellation,
            // we need to parse the digits and rely on the header specs
            if !self.v3 {
                match self.constellation {
                    Constellation::Mixed => None,
                    constellation => {
                        // PRN# parsing attempt
                        if let Ok(prn) = &self.epoch_descriptor[start..end].trim().parse::<u8>() {
                            Some(SV {
                                prn: *prn,
                                constellation,
                            })
                        } else {
                            None
                        }
                    },
                }
            } else {
                None
            }
        }
    }

    /// Macro to directly parse numsat from recovered descriptor
    fn epoch_numsat(&self) -> Option<usize> {
        let start = if self.v3 {
            Self::V3_NUMSAT_OFFSET
        } else {
            Self::V1_NUMSAT_OFFSET
        };

        if let Ok(numsat) = &self.epoch_descriptor[start..start + 3].trim().parse::<u8>() {
            Some(*numsat as usize)
        } else {
            None
        }
    }

    /// Builds new CRINEX decompressor.
    /// Inputs
    /// - v3: whether this CRINEX V1 or V3 content will follow
    /// - constellation: file [Constellation] as defined in file [Header]
    /// - gnss_observables: [Observable]s per [Constellation] as defined in [Header].
    pub fn new(
        v3: bool,
        constellation: Constellation,
        gnss_observables: HashMap<Constellation, Vec<Observable>>,
    ) -> Self {
        Self {
            v3,
            numsat: 0,
            sv_ptr: 0,
            numobs: 0,
            obs_ptr: 0,
            constellation,
            gnss_observables,
            first_epoch: true,
            epoch_desc_len: 0,
            sv: Default::default(),
            state: Default::default(),
            epoch_diff: TextDiff::new(""),
            obs_diff: HashMap::with_capacity(8), // cannot initialize yet
            flags_diff: HashMap::with_capacity(8), // cannot initialize yet
            blanks: Vec::with_capacity(64),
            epoch_svs: Vec::with_capacity(64),
            epoch_descriptor: String::with_capacity(256),
            clock_diff: NumDiff::<M>::new(0, M),
        }
    }

    /// Decompresses following line and pushes recovered content into buffer.
    /// Inputs
    ///  - line: trimed line (no \n termination), which is consistent with
    /// [Lines] [Iterator].
    /// - len: line.len()
    /// - buf: destination buffer
    /// - size: size available in destination buffer
    /// Returns
    ///  - size: produced size (total bytes recovered).
    /// It is possible that, depending on current state, that several input lines
    /// are needed to recover a new line. Recovered content may span several lines as well,
    /// especially when working with a V1 stream.
    pub fn decompress(
        &mut self,
        line: &str,
        len: usize,
        buf: &mut [u8],
        size: usize,
    ) -> Result<usize, Error> {
        if size
            < self
                .state
                .size_to_produce(self.v3, self.numsat, self.numobs)
        {
            return Err(Error::BufferOverflow);
        }

        match self.state {
            State::Epoch => self.run_epoch(line, len),
            State::Clock => self.run_clock(line, len, buf),
            State::Observation => self.run_observation(line, len, buf),
        }
    }

    /// Process the given line, during [State::Epoch] state.
    fn run_epoch(&mut self, line: &str, len: usize) -> Result<usize, Error> {
        let min_len = if self.v3 {
            State::MIN_COMPRESSED_EPOCH_SIZE_V3
        } else {
            State::MIN_COMPRESSED_EPOCH_SIZE_V1
        };

        if len < min_len {
            return Err(Error::EpochFormat);
        }

        let trimmed = &line[1..].trim_end();

        if line.starts_with('&') {
            if self.v3 {
                #[cfg(feature = "log")]
                error!("CRINEX-V3: illegal start of line");
                return Err(Error::BadV3Format);
            }

            self.epoch_diff.force_init(trimmed);
            self.epoch_descriptor = trimmed.to_string();
            self.epoch_desc_len = trimmed.len();
        } else if line.starts_with('>') {
            if !self.v3 {
                #[cfg(feature = "log")]
                error!("CRINEX-V1: illegal start of line");
                return Err(Error::BadV1Format);
            }

            self.epoch_diff.force_init(trimmed);
            self.epoch_descriptor = trimmed.to_string();
            self.epoch_desc_len = trimmed.len();
        } else {
            self.epoch_descriptor = self.epoch_diff.decompress(trimmed).to_string();
            self.epoch_desc_len = self.epoch_descriptor.len();
        }

        // numsat needs to be recovered right away,
        // because it is used to determine the next production size
        if let Some(numsat) = self.epoch_numsat() {
            #[cfg(feature = "log")]
            trace!(
                "recovered epoch: \"{}\" [size={}, numsat={}]",
                self.epoch_descriptor,
                self.epoch_desc_len,
                self.numsat,
            );

            // proceed
            self.numsat = numsat;

            // The text kernel never shrinks: when fewer vehicles are observed
            // than in a previous epoch, stale identifiers trail the description.
            // Only numsat vehicles are meaningful.
            let expected_len = Self::sv_slice_start(self.v3, self.numsat);
            if self.epoch_desc_len > expected_len {
                self.epoch_descriptor.truncate(expected_len);
                self.epoch_desc_len = expected_len;
            }

            self.state = State::Clock;
            Ok(0)
        } else {
            // corrupt epoch numsat
            #[cfg(feature = "log")]
            error!("corrupt numsat");
            Err(Error::CorruptNumsat)
        }
    }

    /// Fills user buffer with recovered epoch, following either V1 or V3 standards
    fn format_epoch(&self, clock_data: Option<i64>, buf: &mut [u8]) -> usize {
        if self.v3 {
            self.format_epoch_v3(clock_data, buf)
        } else {
            self.format_epoch_v1(clock_data, buf)
        }
    }

    /// Fills user buffer with recovered epoch, following V3 standards
    fn format_epoch_v3(&self, clock_data: Option<i64>, buf: &mut [u8]) -> usize {
        // V3 format is much simpler
        // all we need to do is extract SV `XXY` to append in each following lines
        let mut produced = 1;
        buf[0] = b'>'; // special marker

        let bytes = self.epoch_descriptor.as_bytes();

        // push timestamp +flag
        buf[produced..produced + 34].copy_from_slice(&bytes[..34]);
        produced += 34;

        // provide clock data, if any.
        // RINEX V3 formats the receiver clock offset as F15.12 (seconds) in
        // columns 42-56, CRINEX stores it with the decimal point removed:
        // 10^-12 s units.
        if let Some(clock_data) = clock_data {
            let value = clock_data as f64 / 1.0E12;
            let formatted = format!("      {:15.12}", value);
            let fmt_len = formatted.len(); // TODO improve: this is constant
            let bytes = formatted.as_bytes();
            buf[produced..produced + fmt_len].copy_from_slice(&bytes);
            produced += fmt_len; // TODO improve: this is constant
        }

        produced
    }

    /// Fills user buffer with recovered epoch, following V1 standards
    fn format_epoch_v1(&self, clock_data: Option<i64>, buf: &mut [u8]) -> usize {
        let mut produced = 1;
        buf[0] = b' '; // single whitespace

        let bytes = self.epoch_descriptor.as_bytes();

        // push first line (up to 68 bytes)
        let first_len = self.epoch_desc_len.min(67);

        buf[produced..produced + first_len].copy_from_slice(&bytes[..first_len]);
        produced += first_len;

        // push clock offset (if any).
        // RINEX V2 formats the receiver clock offset as F12.9 (seconds) in
        // columns 69-80, CRINEX stores it with the decimal point removed:
        // 10^-9 s units. Pad the SV list up to column 68 first, in case fewer
        // than 12 SVs were observed.
        if let Some(clock_data) = clock_data {
            while produced < Self::V1_CLOCK_OFFSET {
                buf[produced] = b' ';
                produced += 1;
            }
            let value = clock_data as f64 / 1.0E9;
            let formatted_ck = format!("{:12.9}", value);
            let fmt_len = formatted_ck.len(); // TODO: improve (constant)
            let formatted_ck = formatted_ck.as_bytes();
            buf[produced..produced + fmt_len].copy_from_slice(&formatted_ck);
            produced += fmt_len;
        }

        // construct all following lines that need to be wrapped and padded:
        // 12 SVs (36 characters) per continuation line.
        // Lines are separated by '\n', the caller terminates the last one.
        let mut offset = 67;
        let nb_extra = div_ceil(self.epoch_desc_len.saturating_sub(67), 36);

        for _ in 0..nb_extra {
            // conclude previous line
            buf[produced] = b'\n';
            produced += 1;

            // extra padding
            buf[produced..produced + 32].copy_from_slice(&[
                b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
                b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
                b' ', b' ', b' ', b' ',
            ]);

            produced += 32;

            // copy data slice
            let end = (offset + 36).min(self.epoch_desc_len);
            let size = end - offset;

            buf[produced..produced + size].copy_from_slice(&bytes[offset..end]);

            offset += size;
            produced += size;
        }

        produced
    }

    /// Process following line, in [State::Clock]
    fn run_clock(&mut self, line: &str, _len: usize, buf: &mut [u8]) -> Result<usize, Error> {
        // The receiver clock line is one of
        //  - "" : no clock offset for this epoch
        //  - "m&value" : kernel reset at order m, value is the offset itself
        //  - "value" : compressed offset, to run through the kernel
        // Depending on the caller, the line may still carry its line
        // termination or trailing blanks: strip them before interpreting it,
        // otherwise a kernel reset silently fails to parse and the following
        // values are either dropped or decompressed with an uninitialized kernel.
        let line = line.trim();
        let bytes = line.as_bytes();

        let clock_data = if bytes.len() > 2 && bytes[1] == b'&' {
            match (line[..1].parse::<usize>(), line[2..].parse::<i64>()) {
                (Ok(order), Ok(val)) => {
                    // valid kernel reset
                    self.clock_diff.force_init(val, order);
                    Some(val)
                },
                _ => None,
            }
        } else if !line.is_empty() {
            match line.parse::<i64>() {
                Ok(val) => Some(self.clock_diff.decompress(val)?),
                Err(_) => None,
            }
        } else {
            None
        };

        // now that we have potentially recovered clock data
        // we can format the complete epoch description
        let produced = self.format_epoch(clock_data, buf);

        // prepare for observation state
        self.obs_ptr = 0;
        self.sv_ptr = 0;
        self.first_epoch = false;

        // this actually grabs the first satellite
        if let Some(sv) = self.next_satellite() {
            self.sv = sv;
        } else {
            // failed to grab one: corrupt content.
            #[cfg(feature = "log")]
            error!("failed to grab 1st sat");
            return Err(Error::SatelliteIdentification);
        }

        // This prepares the compression kernel for new rising satellites.
        //
        // We also verify that all satellites being described are sane & valid.
        // On any corrupt content, we decided to abort in order to avoid possibly
        // complex resynchronization scenario. But that also means we are not able
        // to partly decompress the first valid data fields. In otherwords, we are
        // very sensitive to valid satellites description.
        self.epoch_svs.clear();

        for i in 0..self.numsat {
            let start = Self::sv_slice_start(self.v3, i);

            // any invalid SV description, will cause us to wait for a new epoch.
            // In other terms, epoch is fully disregarded.
            let sv = match SV::from_str(&self.epoch_descriptor[start..start + 3]) {
                Ok(sv) => sv,
                Err(_) => {
                    // SV parsing may be in failure in case of very old V1 CRINEX mono GNSS
                    // that omit the constellation
                    if !self.v3 {
                        if let Ok(prn) = &self.epoch_descriptor[start + 1..start + 3]
                            .trim()
                            .parse::<u8>()
                        {
                            SV {
                                prn: *prn,
                                constellation: self.constellation,
                            }
                        } else {
                            #[cfg(feature = "log")]
                            error!("corrupt satellite identified");
                            return Err(Error::SatelliteIdentification);
                        }
                    } else {
                        #[cfg(feature = "log")]
                        error!("corrupt satellite identified");
                        return Err(Error::SatelliteIdentification);
                    }
                },
            };

            self.epoch_svs.push(sv);

            // initialize compression kernel on first satellite encounter
            if self.flags_diff.get(&sv).is_none() {
                // Initializes with a little bit of capacity to improve performances.
                let textdiff = TextDiff::new("               ");
                self.flags_diff.insert(sv, textdiff);
            }
        }

        // vehicles absent from this epoch restart from scratch when they
        // come back: their kernels are reset by the compressor and their
        // flags are published in full.
        let epoch_svs = &self.epoch_svs;
        self.flags_diff.retain(|sv, _| epoch_svs.contains(sv));
        self.obs_diff.retain(|(sv, _), _| epoch_svs.contains(sv));

        let obs = self
            .get_observables(&self.sv.constellation)
            .ok_or(Error::SatelliteIdentification)?;

        self.numobs = obs.len();
        self.state = State::Observation;
        Ok(produced)
    }

    /// Process following line, in [State::Observation].
    ///
    /// A compressed observation line is made of exactly `numobs` fields
    /// separated by a single blank, followed by the (text compressed)
    /// LLI/SNR flags of all observables. A field is either empty (missing
    /// observation), a kernel reset `m&value`, or a compressed value.
    /// Trailing missing fields and unchanged flags are usually trimmed
    /// off the line by the compressor.
    fn run_observation(&mut self, line: &str, _len: usize, buf: &mut [u8]) -> Result<usize, Error> {
        let line = line.trim_end();
        let line_len = line.len();
        let numobs = self.numobs;

        let mut produced = 0;

        if self.v3 {
            // prepend SVNN identity
            let start = Self::sv_slice_start(true, self.sv_ptr);
            let end = (start + 3).min(self.epoch_desc_len);
            let bytes = self.epoch_descriptor.as_bytes();
            buf[..3].copy_from_slice(&bytes[start..end]);
            produced += 3;
        }

        self.blanks.clear();
        self.blanks.resize(numobs, true);

        let mut pos = 0;

        for ptr in 0..numobs {
            // V1: at most 5 observations per line
            if !self.v3 && ptr > 0 && ptr % 5 == 0 {
                buf[produced] = b'\n';
                produced += 1;
            }

            // grab next field, if the line was not trimmed before it
            let field = if pos < line_len {
                let rest = &line[pos..];
                match rest.find(' ') {
                    Some(size) => {
                        pos += size + 1;
                        &rest[..size]
                    },
                    None => {
                        pos = line_len;
                        rest
                    },
                }
            } else {
                ""
            };

            let value = self.decompress_field(ptr, field)?;

            let formatted = match value {
                Some(value) => format!("{:14.3}  ", value as f64 / 1000.0),
                None => "                ".to_string(),
            };

            self.blanks[ptr] = value.is_none();

            let bytes = formatted.as_bytes();
            buf[produced..produced + bytes.len()].copy_from_slice(bytes);
            produced += bytes.len();
        }

        // whatever remains is the compressed flags text.
        // When the line was trimmed, flags are simply unchanged.
        let compressed_flags = if pos < line_len { &line[pos..] } else { "" };

        let textdiff = self
            .flags_diff
            .get_mut(&self.sv)
            .expect("internal error: bad crinex content?");

        let flags = textdiff.decompress(compressed_flags);

        Self::write_flags(flags, &self.blanks, self.v3, buf);

        // conclude this SV
        self.sv_ptr += 1;

        if self.sv_ptr == self.numsat {
            self.state = State::Epoch;
        } else {
            self.sv = self
                .next_satellite()
                .ok_or(Error::SatelliteIdentification)?;

            let constellation = if self.sv.constellation.is_sbas() {
                Constellation::SBAS
            } else {
                self.sv.constellation
            };

            self.numobs = self
                .get_observables(&constellation)
                .ok_or(Error::SatelliteIdentification)?
                .len();
        }

        Ok(produced)
    }

    /// Decompresses one observation field of the current SV.
    /// Returns the recovered value (in 10^-3 units), or None when the
    /// observation is missing or the field is not interpretable.
    fn decompress_field(&mut self, ptr: usize, field: &str) -> Result<Option<i64>, Error> {
        if field.is_empty() {
            return Ok(None);
        }

        let bytes = field.as_bytes();

        if bytes.len() > 2 && bytes[1] == b'&' {
            // kernel reset
            let level = match field[..1].parse::<usize>() {
                Ok(level) => level,
                Err(_) => return Ok(None),
            };
            let value = match field[2..].parse::<i64>() {
                Ok(value) => value,
                Err(_) => return Ok(None),
            };

            if let Some(kernel) = self.obs_diff.get_mut(&(self.sv, ptr)) {
                kernel.force_init(value, level);
            } else {
                let kernel = NumDiff::<M>::new(value, level);
                self.obs_diff.insert((self.sv, ptr), kernel);
            }

            return Ok(Some(value));
        }

        let value = match field.parse::<i64>() {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };

        // compressed value: meaningless without a kernel
        match self.obs_diff.get_mut(&(self.sv, ptr)) {
            Some(kernel) => Ok(Some(kernel.decompress(value)?)),
            None => Ok(None),
        }
    }

    /// Helper to retrieve observable for given system
    fn get_observables(&self, constellation: &Constellation) -> Option<&Vec<Observable>> {
        // in case of (multi-gnss) "mixed" files,
        // we only stored one description using "mixed"
        if let Some(mixed) = self.gnss_observables.get(&Constellation::Mixed) {
            Some(mixed)
        } else {
            // SBAS special case
            if constellation.is_sbas() {
                self.gnss_observables.get(&Constellation::SBAS)
            } else {
                self.gnss_observables.get(constellation)
            }
        }
    }

    /// Inserts the LLI/SNR flags into the observation line(s) already
    /// formatted in the buffer. Observables that are missing keep their
    /// flags in the kernel but are printed fully blank.
    fn write_flags(flags: &str, blanks: &[bool], v3: bool, buf: &mut [u8]) {
        let bytes = flags.as_bytes();

        for (i, blank) in blanks.iter().enumerate() {
            if *blank {
                continue;
            }

            let offset = if v3 {
                3 + i * 16 // SVNN prefix, no wrapping
            } else {
                i * 16 + i / 5 // 5 observations per line
            };

            if let Some(lli) = bytes.get(i * 2) {
                if *lli != b' ' {
                    buf[offset + 14] = *lli;
                }
            }

            if let Some(snr) = bytes.get(i * 2 + 1) {
                if *snr != b' ' {
                    buf[offset + 15] = *snr;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        hatanaka::decompressor::{Decompressor, State},
        prelude::SV,
    };
    use std::str::{from_utf8, FromStr};

    /// An epoch without any satellite, or a header without observable,
    /// must not underflow the size to produce.
    #[test]
    fn size_to_produce_without_satellite() {
        let epoch = State::Clock.size_to_produce(false, 0, 0);
        assert!(epoch >= State::MIN_DECOMPRESSED_EPOCH_SIZE_V1);
        assert_eq!(State::Observation.size_to_produce(false, 3, 0), 1);
        assert_eq!(State::Observation.size_to_produce(true, 3, 0), 3);
    }

    #[test]
    fn epoch_size_to_produce_v1() {
        for (numsat, expected) in [
            (
                9,
                " 17  1  1  3 33 40.0000000  0  9G30G27G11G16G 8G 7G23G 9G 1",
            ),
            (
                10,
                " 17  1  1  0  0  0.0000000  0 10G31G27G 3G32G16G 8G14G23G22G26",
            ),
            (
                11,
                " 17  1  1  0  0  0.0000000  0 11G31G27G 3G32G16G 8G14G23G22G26G27",
            ),
            (
                12,
                " 17  1  1  0  0  0.0000000  0 12G31G27G 3G32G16G 8G14G23G22G26G27G28",
            ),
            (
                13,
                " 21 01 01 00 00 00.0000000  0 13G07G08G10G13G15G16G18G20G21G23G26G27
                G29",
            ),
            (
                14,
                " 21 01 01 00 00 00.0000000  0 14G07G08G10G13G15G16G18G20G21G23G26G27
                G29G30",
            ),
            (
                24,
                " 21 12 21  0  0  0.0000000  0 24G07G08G10G16G18G21G23G26G32R04R05R10
                R12R19R20R21E04E11E12E19E24E25E31E33",
            ),
            (
                25,
                " 21 12 21  0  0  0.0000000  0 25G07G08G10G16G18G21G23G26G32R04R05R10
                R12R19R20R21E04E11E12E19E24E25E31E33
                S23",
            ),
            (
                26,
                " 21 12 21  0  0  0.0000000  0 26G07G08G10G16G18G21G23G26G32R04R05R10
                R12R19R20R21E04E11E12E19E24E25E31E33
                S23S36",
            ),
        ] {
            let size = State::Epoch.size_to_produce(false, numsat, 0);
            assert_eq!(size, 0); // Should wait for Clock data !

            // room for a possible clock offset (F12.9, after the SVs padded to col 68)
            let size = State::Clock.size_to_produce(false, numsat, 0);
            assert_eq!(
                size,
                expected.len() + State::V1_CLOCK_SIZE,
                "failed for \"{}\"",
                expected
            );
        }
    }

    #[test]
    fn data_size_to_produce_v1() {
        for (numobs, expected) in [
            (1, " 110158976.908 8"),
            (2, " 110158976.908 8  85838153.10248"),
            (3, " 110158976.908 8  85838153.10248  20962551.380  "),
            (
                4,
                " 119147697.073 7  92670417.710 7  22249990.480    22249983.480  ",
            ),
            (
                5,
                "  24017462.340       -3054.209       -2379.903          43.650          41.600  ",
            ),
            (
                6,
                "  24017462.340       -3054.209       -2379.903          43.650          41.600  
                25509828.140  ",
            ),
            (
                9,
                "  24017462.340       -3054.209       -2379.903          43.650          41.600  
                25509828.140        2836.327        2210.128          41.600  ",
            ),
            (
                10,
                "  24017462.340       -3054.209       -2379.903          43.650          41.600  
                25509828.140        2836.327        2210.128          41.600          41.650  ",
            ),
            (
                14,
                "  24017462.340       -3054.209       -2379.903          43.650          41.600  
                25509828.140        2836.327        2210.128          41.600          41.650  
               100106048.706 6  25509827.540        2118.232          39.550  ",
            ),
        ] {
            let size = State::Observation.size_to_produce(false, 0, numobs);
            assert_eq!(size, expected.len(), "failed for \"{}\"", expected);
        }
    }

    #[test]
    fn epoch_size_to_produce_v3() {
        for (numsat, expected) in [
            (18, "> 2022 03 04 00 00  0.0000000  0 18"),
            (22, "> 2022 03 04 00 00  0.0000000  0 22"),
        ] {
            let size = State::Epoch.size_to_produce(false, numsat, 0);
            assert_eq!(size, 0); // Should wait for Clock data !

            let size = State::Clock.size_to_produce(true, numsat, 0);
            assert_eq!(size, expected.len(), "failed for \"{}\"", expected);
        }
    }

    #[test]
    fn data_size_to_produce_v3() {
        for (numobs, expected) in [
            (1, "G01  20243517.560  "),
            (2, "G03  20619020.680   108353702.79708"),
            (
                4,
                "R10  22432243.520   119576492.91607      1307.754          43.250  ",
            ),
            (
                8,
                "R17  20915624.780   111923741.34508      1970.309          49.000    20915629.120    87051816.58507      1532.457          46.500  ",
            ),
        ] {
            let size = State::Observation.size_to_produce(true, 0, numobs);
            assert_eq!(size, expected.len(), "failed for \"{}\"", expected);
        }
    }

    #[test]
    fn v1_sv_slice() {
        let recovered = "21 01 01 00 00 00.0000000  0 24G07G08G10G13G15G16G18G20G21G23G26G27G30R01R02R03R08R09R15R16R17R18R19R20";
        for sv_index in 0..24 {
            let start = Decompressor::sv_slice_start(false, sv_index);
            let slice_str = &recovered[start..start + 3];
            if sv_index == 0 {
                assert_eq!(slice_str, "G07");
            } else if sv_index == 1 {
                assert_eq!(slice_str, "G08");
            }
            let _ = SV::from_str(slice_str.trim()).unwrap();
        }
    }

    #[test]
    fn v1_numsat_slice() {
        let recovered = "21 01 01 00 00 00.0000000  0 24G07G08G10G13G15G16G18G20G21G23G26G27G30R01R02R03R08R09R15R16R17R18R19R20";
        let offset = Decompressor::V1_NUMSAT_OFFSET;
        let numsat_str = &recovered[offset..offset + 3];
        assert_eq!(numsat_str, " 24");
        let numsat = numsat_str.trim().parse::<u64>().unwrap();
        assert_eq!(numsat, 24);
    }

    #[test]
    fn v3_sv_slice() {
        let recovered = " 2020 06 25 00 00 00.0000000  0 43      C05C07C10C12C19C20C23C32C34C37E01E03E05E09E13E15E24E31G02G05G07G08G09G13G15G18G21G27G28G30R01R02R08R09R10R11R12R17R18R19S23S25S36";
        for sv_index in 0..43 {
            let start = Decompressor::sv_slice_start(true, sv_index);
            let slice_str = &recovered[start..start + 3];
            if sv_index == 0 {
                assert_eq!(slice_str, "C05");
            } else if sv_index == 1 {
                assert_eq!(slice_str, "C07");
            }
            let _ = SV::from_str(slice_str.trim()).unwrap();
        }
    }

    #[test]
    fn v3_numsat_slice() {
        let recovered = " 2020 06 25 00 00 00.0000000  0 43      C05C07C10C12C19C20C23C32C34C37E01E03E05E09E13E15E24E31G02G05G07G08G09G13G15G18G21G27G28G30R01R02R08R09R10R11R12R17R18R19S23S25S36";
        let offset = Decompressor::V3_NUMSAT_OFFSET;
        let numsat_str = &recovered[offset..offset + 3];
        assert_eq!(numsat_str, " 43");
        let numsat = numsat_str.trim().parse::<u64>().unwrap();
        assert_eq!(numsat, 43);
    }

    #[test]
    fn v1_flags_format() {
        for (flags, numobs, buffer, expected) in [
            (
                " 5",
                3,
                " 131869667.223                    25093963.200",
                " 131869667.223 5                  25093963.200",
            ),
            (
                " 5  1",
                3,
                " 131869667.223                    25093963.200",
                " 131869667.223 5                  25093963.2001",
            ),
            (
                " 5  12",
                3,
                " 131869667.223                    25093963.200",
                " 131869667.223 5                  25093963.20012",
            ),
            (
                "45  12",
                3,
                " 131869667.223                    25093963.200",
                " 131869667.22345                  25093963.20012",
            ),
            (
                "4 06 1   6",
                5,
                " 106305408.320    27089583.280       -1635.689          45.200   109078577.583 6",
                " 106305408.3204   27089583.28006     -1635.689 1        45.200   109078577.583 6",
            ),
            (
                "49484 4 4",
                5,
                " 106305408.320    27089583.280       -1635.689          45.200   109078577.583",
                " 106305408.32049  27089583.28048     -1635.6894         45.2004  109078577.5834",
            ),
            (
                " 6 6 7060407 4 4",
                11,
                "  23203962.113    23203960.554    23203963.222   121937655.118    95016353.749  
  91057352.202    23203961.787    23203960.356          41.337          28.313  
        46.834",
                "  23203962.113 6  23203960.554 6  23203963.222 7 121937655.11806  95016353.74904
  91057352.20207  23203961.787 4  23203960.356 4        41.337          28.313  
        46.834",
            ),
        ] {
            let flags_len = flags.len();
            let buffer_len = buffer.len();
            let bytes = buffer.as_bytes();

            let mut buf = [0; 256];
            buf[..buffer_len].copy_from_slice(&bytes);

            Decompressor::write_flags(flags, &vec![false; numobs], false, &mut buf);

            let output = from_utf8(&buf[..expected.len()]).expect("did not generate valid UTF-8");

            // verify that (in place) write did its job
            assert_eq!(output, expected, "failed for \"{}\"", flags);
        }
    }

    #[test]
    fn v3_flags_format() {
        for (flags, buffer, expected) in [(
            "  06     6",
            "G01  24600158.420   129274705.784          38.300    24600162.420   100733552.500  ",
            "G01  24600158.420   129274705.78406        38.300    24600162.420   100733552.500 6",
        )] {
            let flags_len = flags.len();
            let buffer_len = buffer.len();
            let bytes = buffer.as_bytes();

            let mut buf = [0; 128];
            buf[..buffer_len].copy_from_slice(&bytes);

            let numobs = buffer.split_ascii_whitespace().count() - 1;

            Decompressor::write_flags(flags, &vec![false; numobs], true, &mut buf);

            let output = from_utf8(&buf[..expected.len()]).expect("did not generate valid UTF-8");

            // verify that (in place) write did its job
            assert_eq!(output, expected);
        }
    }
}
