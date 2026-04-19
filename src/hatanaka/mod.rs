//! RINEX compression and decompression module
use thiserror::Error;

/// The Hatanaka scheme performs higher-order numeric delta compression and
/// textual diff compression on RINEX data. It produces ASCII output.
/// The ASCII output should be further compressed with a general-purpose compressor
/// (gzip, etc.)
///
/// It is lossless with regard to the data, but not to formatting ideosyncracies
/// such as -.123 vs -0.123.
///
/// Hatanaka, Yuki (2008). "A Compression Format and Tools for GNSS Observation Data".
/// Bulletin of the Geographical Survey Institute. 55: 21–30.
/// https://www.gsi.go.jp/common/000045517.pdf
mod compressor;
mod crinex;
mod decompressor;
mod errors;
mod numdiff;
mod textdiff;

pub use compressor::Compressor;
pub use crinex::CRINEX;

pub use decompressor::{
    io::{DecompressorExpertIO, DecompressorIO},
    Decompressor, DecompressorExpert,
};

pub use errors::CRX2RNXError;
pub use numdiff::NumDiff;
pub use textdiff::TextDiff;
