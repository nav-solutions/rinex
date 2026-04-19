//! Errors specific to Observation RINEX files parsing
use thiserror::Error;

/// Errors that may arise during the parsing of observation
/// RINEX files, either in the Header section (mandatory prior
/// file processing) or during Record section.
#[derive(Debug, Error)]
pub enum ObsRINEXParsingError {
    /// Parsing failed to grab one more line from the file reader.
    /// This is most likely due to a corruption in the file,
    /// where the epoch description does not match the actual epoch content
    /// (missing content).
    #[error("Missing line")]
    MissingLine,

    /// Corruption, invalid or non supported reference timescale
    /// for these Observations. The timescale must be correct or supported,
    /// otherwise we can't process this file (index data points correctly).
    #[error("Reference timescale")]
    ReferenceTimescale,

    /// Invalid or corrupt epoch flag description.
    /// We support all valid Epoch flags:
    /// [https://docs.rs/rinex/latest/rinex/observation/enum.EpochFlag.html],
    /// this is most likely due to a corruption in the text file.
    #[error("Invalid Epoch flag")]
    InvalidEpochFlag,

    /// Failed to parse the number of satellites contained
    /// in the Epoch to follow. This is most likely due to a
    /// corruption in that integer number in the text file.
    #[error("Epoch numsat parsing")]
    EpochNumsat,

    /// Raised by corruption in the satellites description
    /// in RINEX v2 epoch descriptors. The descriptor defines
    /// all satellites to follow. This is most likely due to
    /// a incorrectly truncated line.
    #[error("Corrupt RINEXv2 satellites description")]
    V2SatellitesDescription,

    /// Receiver hardware external events are not supported
    /// by this library yet. We only support the following flags
    /// - Epoch::OK flag (nominal)
    /// - Epoch::PowerFailure (power failure start or end)
    /// - or Epoch::CycleSlip flags (nominal possibly corrupt by CS)
    /// This topic is tracked by a dedicated issue on Github.
    #[error("Non supported receiver event")]
    NonSupportedReceiverEvent,

    /// The provided content does not fit the description of a valid
    /// RINEX Observable, because it must be 2 or 3 UTF-8 character
    /// descriptor.
    #[error("Invalid Observable")]
    InvalidObservable,

    /// The parser failed when it encountered an invalid signal Observable.
    /// We support all valid Observables:
    /// [https://docs.rs/rinex/latest/rinex/prelude/enum.Observable.html].
    /// A valid observable starts with either:
    /// - 'C' for pseudo-range measurements
    /// - 'L' for phase-range measurements
    /// - 'S' for SSI measurements
    /// - 'P' for glonass legacy pseudo-range measurements
    /// This is most likely due to corruption in the text file.
    #[error("Incorrect Observable")]
    IncorrectObservable,

    /// The proposed "Observable" descriptor does not match
    /// the correct definition of an "Observable". This is due
    /// to an internal error (that should never happen) where
    /// the parser proposed more than 3 UTF-8 caracters to be parsed.
    #[error("Incorrect Observable")]
    ObservableSizeError,

    /// Invalid or corrupt Observation RINEX "Observable".
    /// We support all valid Observables:
    /// [https://docs.rs/rinex/latest/rinex/prelude/enum.Observable.html].
    /// This is raised by a corruption in the text file.
    #[error("Invalid Observable")]
    IncorrectObservable,
    // not tested
}
