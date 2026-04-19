/// Errors possibly raised during the CRX2RNX or RNX2CRNX algorithms
use thiserror::Error;

/// Errors possibly raised during the CRX2RNX (RINEX decompression) algorithm
#[derive(Debug, Error)]
pub enum CRX2RNXError {
    /// Buffer too small to new data.
    /// This arises when the internal CRX2RNX core is about to produce
    /// (recover) too many bytes that cannot fit into the target buffer.
    /// This should never arise, the target buffer should be large enough,
    /// based on the RINEX format.
    #[error("buffer overflow")]
    BufferOverflow,

    /// This error may arise when the internal state machine is trying
    /// to recover the Epoch descriptor. This error is raised
    /// when the file reader (external data provider) is providing
    /// a new epoch descriptor that is too short compared to the
    /// standard (c)RINEX format. This may appear when trying to decompress
    /// a corrupt compressed file.
    #[error("input epoch size underflow")]
    InputEpochUnderflow,

    /// This error may arise when the internal state machine is trying
    /// to decompress data (inside the epoch).
    /// This error is raised when the input (compressed) line
    /// starts with '>' (which is the CRINEX V3 new epoch announcement),
    /// while the CRINEX header described a CRINEX V1 file.
    /// So the compressed data does not fit the file header (allocation & preparation):
    /// the input file is completely corrupt.
    #[error("corrupt v1 context or content")]
    CorruptV1ContextOrContent,

    /// This error may arise when the internal state machine is trying
    /// to decompress data (inside the epoch).
    /// This error is raised when the input (compressed) line
    /// starts with '&' (which is the CRINEX V1 new epoch announcement),
    /// while the CRINEX header described a CRINEX V3 file.
    /// So the compressed data does not fit the file header (allocation & preparation):
    /// the input file is completely corrupt.
    #[error("corrupt v3 context or content")]
    CorruptV3ContextOrContent,

    /// This error may arise when the internal state machine is trying to recover the epoch descriptor.
    /// The recovered numsat (number of satellite in epoch to follow) is corrupt and does not match an unsigned integer.
    /// This can be due to either
    /// - corruption in the CRINEX description that led to the state machine
    /// to this state
    /// - this decoder behaving incorrectly (should not happen).
    #[error("numsat integer number parsing error")]
    NumsatIntegerParsing,

    /// This error may arise as the state machine is trying
    /// to recover new observations inside the epoch, as it progresses
    /// from one satellite to the next, we need to progress in the
    /// epoch descriptor and identify the satellites the next data points will
    /// refer to. This error is raised in CRINEX3 files, where the satelite
    /// identification should always pass "as is".
    #[error("invalid CRINEX 3 sallite")]
    InvalidCRINEX3Satellite,

    /// This error may arise as the state machine is trying
    /// to recover new observations inside the epoch, as it progresses
    /// from one satellite to the next, we need to progress in the
    /// epoch descriptor and identify the satellites the next data points will
    /// refer to. This error is raised because this CRINEX 1 Header
    /// did not define a GNSS system explicitly.
    #[error("Illegal CRINEX-1 header with out explicit constellation")]
    IncorrectCRINEX1GnssHeader,

    /// This error may arise as the state machine is trying
    /// to recover new observations inside the epoch, as it progresses
    /// from one satellite to the next, we need to progress in the
    /// epoch descriptor and identify the satellites the next data points will
    /// refer to.
    /// This error is raised when the PRN number does not match a valid 2-digit unsigned number.
    #[error("satellite PRN number parsing error")]
    SatellitePrnIntegerParsing,

    /// Observable identification failure: when decompressing,
    /// we need to identify which physical observables relate to the pending
    /// satellite.
    #[error("observables identification error")]
    ObservablesIdentification,
}
