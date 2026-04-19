use thiserror::Error;

use std::io::Error as IoError;

/// Errors that may rise in Formatting process
#[derive(Error, Debug)]
pub enum FormattingError {
    #[error("i/o: output error")]
    OutputError(#[from] IoError),

    #[error("missing constellation information")]
    UndefinedConstellation,

    #[error("missing navigation standard specs")]
    MissingNavigationStandards,

    #[error("undefined observables")]
    UndefinedObservables,

    #[error("missing observable definition")]
    MissingObservableDefinition,

    #[error("nav: unknown radio message")]
    NoNavigationDefinition,

    #[error("nav: missing grid defs")]
    NoGridDefinition,
}

/// General error (processing, analysis..)
#[derive(Debug)]
pub enum Error {
    /// Invalid frequency
    InvalidFrequency,

    /// Sampling Period is not determined
    UndeterminedSamplingPeriod,

    /// Unknown frequency
    UnknownFrequency,

    /// Non supported GPS [Observable]
    UnknownGPSObservable,

    /// Non supported Galileo [Observable]
    UnknownGalieoObservable,

    /// Non supported Glonass [Observable]
    UnknownGlonassObservable,

    /// Non supported QZSS [Observable]
    UnknownQZSSObservable,

    /// Non supported BDS [Observable]
    UnknownBeiDouObservable,

    /// Non supported IRNSS [Observable]
    UnknownIRNSSObservable,

    /// Non supported SBAS [Observable]
    UnknownSBASObservable,

    /// Non supported DORIS [Observable]
    UnknownDORISObservable,

    /// Unknown GPS Frequency
    UnknownGPSFrequency,

    /// Unknown Galileo Frequency
    UnknownGalileoFrequency,

    /// Unknown QZSS Frequency
    UnknownQzssFrequency,

    /// Unknown Glonass Frequency
    UnknownGlonassFrequency,

    /// Unknown BDS Frequency
    UnknownBDSFrequency,

    /// Unknown IRNSS Frequency
    UnknownIRNSSFrequency,

    /// Unknown SBAS Frequency
    UnknownSBASFrequency,

    /// Unknown DORIS Frequency
    UnknownDORISFrequency,
}
