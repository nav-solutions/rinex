//! Errors specific to Navigation RINEX files parsing
use thiserror::Error;

/// Errors that may arise during the parsing of navigation
/// RINEX files, either in the Header section (mandatory prior
/// file processing) or during Record section.
#[derive(Debug, Error)]
pub enum NavRINEXParsingError {
    /// Parsing failed to grab mode data (usually one more complete line)
    /// from input file, while parsing either of the supported Navigation
    /// message class. This may arise in correction parameters, like the
    /// Klobuchar model, that require several lines to be fully described.
    #[error("Missing line")]
    MissingLine,

    /// This library has a built-in Navigation message database
    /// which is fully integrated at build time, that we
    /// to retrieve the correct interpretation for each supported
    /// message. You will run into this when either
    /// - the entrypoint of the message is corrupt
    /// - we don't have any reference for this message in our database.
    #[error("No known standard specs for this radio message")]
    MissingStandardSpecifications,

    /// Invalid Navigation frame type description.
    /// We support all possible radio message categories:
    /// [https://docs.rs/rinex/latest/rinex/navigation/enum.NavFrameType.html].
    /// This is most likely due to a corruption in the text file.
    #[error("Invalid frame class")]
    InvalidFrameClass,

    /// Invalid Radio message type description.
    /// We support all valid message types:
    /// [https://docs.rs/rinex/latest/rinex/navigation/enum.NavMessageType.html].
    /// This is most likely due to a corruption in the text file.
    #[error("Invalid radio message type")]
    InvalidMessageType,

    /// Failed to parse one of the Klobuchar model parameters.
    /// This is due to an invalid floating point number in the text file.
    #[error("Klobuchar model parameters parsing error")]
    IonosphereKlobucharParameter,

    /// Failed to parse one of the Nequick-G models parameters.
    /// This is due to an invalid floating point number in the text file.
    #[error("Nequick-G model parameters parsing error")]
    IonosphereNequickGParameter,

    /// Failed to parse one of the BdGIM models parameters.
    /// This is due to an invalid floating point number in the text file.
    #[error("BdGIM Model model parameters parsing error")]
    IonosphereBdgimParameter,

    /// Failed to identified either the reference or referenced timescale
    /// During the timescale identification process of a System Time Offset
    /// message parsing. Either due to
    /// - unsupported timesystem
    /// - or invalid description in the text file
    #[error("Failed to parse system time reference timescale")]
    InvalidTimerefTimescale,

    /// Failed to parse one of the integer numbers defining the
    /// reference epoch of a System Time Offset correction parameters.
    /// This is due to an invalid integer number in the text file.
    #[error("Failed to parse (integer) system time reference week (integer)")]
    TimerefEpochWeekCounter,

    /// Error during the actual system time correction parsing.
    /// This is due to an invalid floating point number in the text file.
    #[error("Error during system time offset parsing")]
    TimerefOffsetParsing,

    // not tested
    #[error("invalid health flag definition")]
    NavHealthFlagDefinition,

    #[error("invalid bitfield")]
    NavFlagsMapping,

    #[error("invalid data source flag definition")]
    NavDataSourceDefinition,

    #[error("unknown complex type")]
    NavUnknownComplexType,

    #[error("invalid or missing flag definition")]
    NavFlagsDefinition,

    #[error("illegal null orbit field")]
    NavNullOrbit,

    #[error("sto data")]
    SystemTimeData,

    #[error("eop missing line")]
    EopMissingData,
}
