mod formatting;
mod parsing;

pub use formatting::FormattingError;
pub use parsing::ParsingError;

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
}
