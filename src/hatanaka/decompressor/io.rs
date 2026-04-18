//! CRINEX decompression from direct I/O

use crate::{
    hatanaka::decompressor::DecompressorExpert,
    prelude::{Constellation, Observable},
};

use std::io::{BufRead, BufReader, Lines, Read};

/// [DecompressorIO] emplements [DecompressorExpert] internally and works
/// directly on any [Read]able I/O interface.
///
/// Use it if your application to decompress CRINEX to readable RINEX efficiently.
/// Refer to [Decompressor] for more information on the algorithm and the limitations.
/// The historical CRX2RNX tool uses M=5 as the maximal compression order to ever be supported:
/// this implementation uses the same setup.
pub type DecompressorIO<R> = DecompressorExpertIO<5, R>;

/// Unlike [DecompressorIO], [DecompressorExpertIO] is not limited in the decompression
/// algorithm and gives you all flexibility. This is needed if your data compressor
/// used a compression level above 3.
pub struct DecompressorExpertIO<const M: usize, R: Read> {
    /// Internal LinesIter. The decompressor works on a line basis.
    lines: Lines<BufReader<R>>,
    /// Internal Decompressor.
    decomp: DecompressorExpert<M>,
}

impl<const M: usize, R: Read> Read for DecompressorExpertIO<M, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // try to pull a new complete line from internal reader
        if let Some(line) = self.lines.next() {
            let line = line?;
            let len = line.len();
            let buf_size = buf.len();
            let size = self
                .decomp
                .decompress(&line, len, buf, buf_size)
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "hatanaka error"))?;
            Ok(size)
        } else {
            Ok(0)
        }
    }
}

impl<const M: usize, R: Read> DecompressorExpertIO<M, R> {
    /// Builds a new [DecompressorExpertIO] from [Read]able interface,
    /// and implements the buffering for you.
    /// Use .Iter() to pull content from the decompressor.
    /// You have two exploitation scenarios:
    /// - either you intend to stream a complete CRINEX,
    /// including its header section. In this case, the core will
    /// pick up everything and adapt: you don't have anything to do.
    /// - or you intend to stream CRINEX directly. In this case,
    /// you should provide the specifications that were given by the
    /// missing Header.
    pub fn from_read(reader: R) -> Self {
        Self {
            lines: BufReader::new(reader).lines(),
            decomp: DecompressorExpert::<M>::default(),
        }
    }
    /// Builds a new [DecompressorExpertIO] from your own Buffered Reader implementation.
    /// Refer to [Self::from_read] for more information.
    pub fn from_bufread(reader: BufReader<R>) -> Self {
        Self {
            lines: reader.lines(),
            decomp: DecompressorExpert::<M>::default(),
        }
    }
    /// Provide [Observable] definitions if you intend to stream CRINEX directly
    /// and skipped the header section.
    pub fn with_observables(&mut self, constell: Constellation, observables: Vec<Observable>) {
        self.decomp.gnss_observables.insert(constell, observables);
    }
}
