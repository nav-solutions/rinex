//! RINEX compression module

use crate::{
    epoch::epoch_decompose as epoch_decomposition,
    error::FormattingError,
    hatanaka::{NumDiff, TextDiff},
    observation::{HeaderFields, Record},
    prelude::{Constellation, Observable, SV},
    BufWriter,
};

use std::{collections::HashMap, io::Write};

use itertools::Itertools;

pub type Compressor = CompressorExpert<5>;

pub struct CompressorExpert<const M: usize> {
    /// True (by default) if this a CRINEX3 compressor.
    /// Modify this before getting started!
    pub v3: bool,

    /// True when epoch descriptor should be compressed.
    /// True on first epoch.
    epoch_compression: bool,

    /// Readable Epoch being compressed
    epoch_buf: String,

    /// Readable flags being compressed
    flags_buf: String,

    /// Epoch [TextDiff]
    epoch_diff: TextDiff,

    /// Flags kernel, per SV
    flags_diff: HashMap<SV, TextDiff>,

    /// Flag textdiff
    /// Compression kernels (per SV and signal)
    sv_kernels: HashMap<(SV, Observable), NumDiff<M>>,
    // /// Clock [NumDiff]
    // clock_diff: NumDiff<M>,
}

impl<const M: usize> Default for CompressorExpert<M> {
    fn default() -> Self {
        Self {
            v3: true,
            epoch_compression: false,
            epoch_diff: TextDiff::new(""),
            epoch_buf: String::with_capacity(128),
            flags_buf: String::with_capacity(128),
            sv_kernels: HashMap::with_capacity(8),
            flags_diff: HashMap::with_capacity(8),
        }
    }
}

impl<const M: usize> CompressorExpert<M> {
    /// Format [Record] using mutable [CompressorExpert].
    /// Compressed bytes are dumped in mutable [BufWriter].
    /// This permits the RNX2CRX compression ops.
    pub fn format<W: Write>(
        &mut self,
        w: &mut BufWriter<W>,
        record: &Record,
        header: &HeaderFields,
    ) -> Result<(), FormattingError> {
        for (k, v) in record.iter() {
            if !k.flag.is_ok() {
                // TODO not 100% correct, verify > 1
                self.epoch_compression = false;
            }

            let (y, m, d, hh, mm, ss, ns) = epoch_decomposition(k.epoch);

            // form unique SV list
            let svnn = v
                .signals
                .iter()
                .map(|sig| sig.satellite)
                .unique()
                .sorted()
                .collect::<Vec<_>>();

            if !self.epoch_compression {
                if self.v3 {
                    write!(w, "> ")?;
                } else {
                    write!(w, "&")?;
                }
            } else {
                if self.v3 {
                    write!(w, "  ")?;
                } else {
                    write!(w, " ")?;
                }
            }

            if self.v3 {
                self.epoch_buf.push_str(&format!(
                    "{:04} {:02} {:02} {:02} {:02} {:02}.{:07}  {}{:3}      ",
                    y,
                    m,
                    d,
                    hh,
                    mm,
                    ss,
                    ns / 100,
                    k.flag,
                    svnn.len(),
                ));
            } else {
                self.epoch_buf.push_str(&format!(
                    "{:02} {:02} {:02} {:02} {:02} {:02}.{:07}  {}{:3}      ",
                    y,
                    m,
                    d,
                    hh,
                    mm,
                    ss,
                    ns / 100,
                    k.flag,
                    svnn.len(),
                ));
            }

            // Append each SV to epoch description
            for sv in svnn.iter() {
                self.epoch_buf.push_str(&format!("{:x}", sv));
            }

            // Epoch compression
            if !self.epoch_compression {
                self.epoch_diff.force_init(&self.epoch_buf);
                writeln!(w, "{}", self.epoch_buf.trim_end())?;
            } else {
                let compressed = self.epoch_diff.compress(&self.epoch_buf);
                writeln!(w, "{}", compressed.trim_end())?;
            }

            if let Some(clk) = v.clock {
                // TODO: clock is not correctly supported yet
                if !self.epoch_compression {
                    writeln!(w, "{}", clk.offset_s)?;
                }
            } else {
                // No clock: BLANKed line
                write!(w, "\n")?;
            }

            // For each SV
            for sv in svnn.iter() {
                // Following header specs
                let sv_observables = header.codes.get(&sv.constellation);

                let sv_observables = match sv_observables {
                    Some(observables) => observables, // correctly identified,
                    None => {
                        // handles SBAS case
                        if sv.constellation.is_sbas() {
                            match header.codes.get(&Constellation::SBAS) {
                                Some(observables) => observables,
                                None => {
                                    // correctly formatted RINEX will never
                                    // end up here
                                    continue;
                                },
                            }
                        } else {
                            // correctly formatted RINEX will never
                            // end up here
                            continue;
                        }
                    },
                };

                for observable in sv_observables.iter() {
                    if let Some(signal) = v
                        .signals
                        .iter()
                        .filter(|sig| sig.satellite == *sv && &sig.observable == observable)
                        .reduce(|k, _| k)
                    {
                        let quantized = (signal.value * 1000.0).round() as i64;

                        // retrieve or build compression kernel
                        if let Some((_, sv_kernel)) = self
                            .sv_kernels
                            .iter_mut()
                            .filter(|((sv, obs), _)| {
                                *sv == signal.satellite && obs == &signal.observable
                            })
                            .reduce(|k, _| k)
                        {
                            let compressed = sv_kernel.compress(quantized);
                            write!(w, "{} ", compressed)?;
                        } else {
                            // first encounter: build kernel
                            let kernel = NumDiff::<M>::new(quantized, 3);
                            self.sv_kernels
                                .insert((signal.satellite, signal.observable.clone()), kernel);

                            write!(w, "{}&{} ", 3, quantized)?;
                        }

                        if let Some(lli) = signal.lli_flags {
                            self.flags_buf.push_str(&format!("{}", lli.bits() as u8));
                        } else {
                            self.flags_buf.push_str(" ");
                        }

                        if let Some(snr) = signal.signal_noise_ratio {
                            self.flags_buf.push_str(&format!("{}", snr as u8));
                        } else {
                            self.flags_buf.push_str(" ");
                        }
                    } else {
                        // BLANK is a single ' '
                        write!(w, "{}", ' ')?;
                        self.flags_buf.push_str("  ");
                    }
                }

                // Flags compression
                if let Some(flags_kernel) = self.flags_diff.get_mut(&sv) {
                    let compressed = flags_kernel.compress(&self.flags_buf);
                    writeln!(w, "{}", compressed)?;
                } else {
                    let mut kernel = TextDiff::new("");
                    let compressed = kernel.compress(&self.flags_buf);
                    writeln!(w, "{}", compressed)?;
                    self.flags_diff.insert(*sv, kernel);
                }
                self.flags_buf.clear();
            }

            // prepare for next epoch
            self.epoch_compression = true;
            self.epoch_buf.clear();
        }
        Ok(())
    }
}
