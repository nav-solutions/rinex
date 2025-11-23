//! RINEX compression / decompression module
use thiserror::Error;

// TODO
// Improve Result<> IoError/Error relation ship
// the current User API .read().error()
// Will trigger a string comparison on every single .read() acces veritication

mod compressor;
mod crinex;
mod decompressor;
mod numdiff;
mod textdiff;

pub use compressor::Compressor;
pub use crinex::CRINEX;

pub use decompressor::{
    io::{DecompressorExpertIO, DecompressorIO},
    Decompressor, DecompressorExpert,
};

pub use numdiff::NumDiff;
pub use textdiff::TextDiff;

use thiserror::Error as ErrorTrait;

/// Hatanaka dedicated Errors
#[derive(Debug, ErrorTrait)]
pub enum Error {
    /// Buffer too small to accept incoming data
    #[error("buffer overflow")]
    BufferOverflow,

    /// Forwarded Epoch description does not look good: Invalid RINEX!
    #[error("invalid epoch format")]
    EpochFormat,

    /// Forwarded content is not consisten with CRINEX V1
    #[error("invalid v1 format")]
    BadV1Format,

    /// Forwarded content is not consisten with CRINEX V3
    #[error("invalid v3 format")]
    BadV3Format,

    /// Satellite identification failure: bad relationship between
    /// either:
    ///   - recovered Epoch description (in decompression scheme)
    ///   and parsing process
    ///   - data to compress does not accord to previous header specs
    #[error("satellite identification error")]
    SatelliteIdentification,

    /// Observable identification failure: when decompressing,
    /// we need to identify which physical observables relate to the pending
    /// satellite.
    #[error("observables identification error")]
    ObservablesIdentification,

    /// Recovered numsat (number of satellite in the descriptor) is corrupt
    /// while we expect digits here.
    #[error("corrupt numsat")]
    CorruptNumsat,
}
