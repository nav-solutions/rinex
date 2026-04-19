//! Errors specific to Navigation RINEX files parsing
use thiserror::Error;

use crate::{navigation::NavMessageType, prelude::Constellation};

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

    /// Error during the parsing of an f64 (floating point)
    /// number described in the text file, that should either
    /// starts with E+000 or E-000 exponent, or D+000 (D-000) exponent
    /// in fortran format. Both formats being valid in RINEX (including V3),
    /// which we adapt to a Rust compatible format
    #[error("floating point (exponent) parsing")]
    FloatingPointParsing,

    /// Incorrect or non-supported combination of
    /// message type for a geo satellite.
    #[error("L-NAV/SBAS no health flags specs")]
    LnavSbasHealthInvalidCombination,

    /// A corrupt "complex type" (describing complex interpretation)
    /// was found when parsing our own (built-in) navigation
    /// message database definitions: which should never happen
    /// and should be 100% correct. To prevent this,
    /// the message database is automatically tested in CI
    /// for each version of this library.
    #[error("invalid complex type: corrupt builtin database!")]
    InvalidComplexType,

    /// A corrupt "complex type" (describing complex interpretation)
    /// was found when parsing our own (built-in) navigation
    /// message flags definitions: which should never happen
    /// and should be 100% correct. To prevent this,
    /// the message database is automatically tested in CI
    /// for each version of this library.
    #[error("invalid flag type: corrupt builtin database!")]
    InvalidComplexFlagType,

    /// No standard definitions in our database for
    /// this message type and satellite combination.
    /// This is raised when the input text file is corrupt
    /// or this type of message is not supported yet.
    #[error("missing health flags specs for {:x}({:x})")]
    MissingHealthFlagsDefinition((NavMessageType, Constellation)),

    /// No standard definitions in our database for
    /// this message type and satellite combination.
    /// This is raised when the input text file is corrupt
    /// or this type of message is not supported yet.
    #[error("missing subsidary health flags specs for {:x}({:x})")]
    MissingSubsidaryHealthFlagsDefinition((NavMessageType, Constellation)),

    /// No standard definitions in our database for
    /// this message type and satellite combination.
    /// This is raised when the input text file is corrupt
    /// or this type of message is not supported yet.
    #[error("missing source flags specs for {:x}({:x})")]
    MissingSourceFlagsDefinition((NavMessageType, Constellation)),

    /// No standard definitions in our database for
    /// this message type and satellite combination.
    /// This is raised when the input text file is corrupt
    /// or this type of message is not supported yet.
    #[error("missing BeiDou satellite type flags specs for {:x}({:x})")]
    MissingBeiDouSatTypeDefinition((NavMessageType, Constellation)),

    /// No standard definitions in our database for
    /// this message type and satellite combination.
    /// This is raised when the input text file is corrupt
    /// or this type of message is not supported yet.
    #[error("missing signal integrity specs for {:x}({:x})")]
    MissingIntegrityDefinition((NavMessageType, Constellation)),

    /// No standard definitions in our database for
    /// this message type and satellite combination.
    /// This is raised when the input text file is corrupt
    /// or this type of message is not supported yet.
    #[error("missing status flags specs for {:x}({:x})")]
    MissingStatusFlagsDefinition((NavMessageType, Constellation)),

    /// Error when trying to identify health flags contained
    /// in one of the CNV-2 messages for a GPS or QZSS satellite.
    /// This is due to an incorrect binary value encoded, or
    /// issue in the bit map we have described.
    #[error("CNV-2 GPS/QZSS health flags")]
    Cnv2GpsQzssHealthFlags,

    /// Error when trying to identify the signal integrity
    /// from this BeiDou CNV-1 (B1-C signal integrity) message.
    #[error("CNV-1 BeiDou B1-C integrity flags")]
    Cnv1BeiDouB1cIntegrityFlags,

    /// Error when trying to identify the signal integrity
    /// from this BeiDou CNV-2 (B2a/B1c signal integrity) message.
    #[error("CNV-2 BeiDou B2a/B1c integrity flags")]
    Cnv2BeiDouB2aB1cIntegrityFlags,

    /// Error when trying to identify the signal integrity
    /// from this BeiDou CNV-3 (B2b signal integrity) message.
    #[error("CNV-3 BeiDou B2b integrity flags")]
    Cnv3BeiDouB2bIntegrityFlags,

    /// Error when trying to identify health flags contained
    /// in one of the Legacy or I-NAV or F-NAV message
    /// of a Galileo satellite.
    /// This is due to an incorrect binary value encoded, or
    /// issue in the bit map we have described.
    #[error("Galileo L-NAV/I-NAV/F-NAV flags")]
    LnavInavFnavGalHealthFlags,

    /// Error when trying to identify the signal source
    /// in one of the Legacy or I-NAV or F-NAV message
    /// of a Galileo satellite.
    /// This is due to an incorrect binary value encoded, or
    /// issue in the bit map we have described.
    #[error("Galileo L-NAV/I-NAV/F-NAV flags")]
    LnavInavFnavGalSourceFlags,

    /// Error when trying to identify health flags contained
    /// in one of the Legacy or FDMA message of a Glonass satellite.
    /// This is due to an incorrect binary value encoded, or
    /// issue in the bit map we have described.
    #[error("Glonass L-NAV/FDMA flags")]
    LnavFDMAGloHealthFlags,

    /// Error when trying to identify health flags contained
    /// in one of the Legacy or D1 or D2 message of a BeiDou satellite.
    /// This is due to an incorrect binary value encoded, or
    /// issue in the bit map we have described.
    #[error("BeiDou L-NAV/D1/D2 flags")]
    LnavD1D2BdsHealthFlags,

    /// Error when trying to identify health flags contained
    /// in one of the Legacy message or SBAS message for a
    /// geo stationnary satellite.
    /// This is due to an incorrect binary value encoded, or
    /// issue in the bit map we have described.
    #[error("L-NAV/SBAS health flags")]
    LnavSbasHealthFlags,

    /// Error when trying to identify subsidary/secondary health flags contained
    /// in one of FDMA message of a glonass satellite.
    /// This is due to an incorrect binary value encoded, or
    /// issue in the bit map we have described.
    #[error("Glonass subsidary FDMA health flags")]
    GloFDMASubsidaryFlags,
}
