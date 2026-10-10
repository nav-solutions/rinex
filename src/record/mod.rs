use crate::{
    antex::Record as AntexRecord, clock::Record as ClockRecord, meteo::Record as MeteoRecord,
    navigation::Record as NavRecord, observation::Record as ObservationRecord, prelude::Epoch,
};

use std::collections::BTreeMap;

#[cfg(feature = "serde")]
use serde::Serialize;

mod formatting;
mod parsing;

/// A recoverable OBS/NAV record problem reported by `parse_with_diagnostics`.
/// No source text is retained. This does not assess navigation usability.
#[derive(Debug)]
pub struct ParsingDiagnostic {
    /// One-based input line within the body where the buffered block starts.
    /// For CRINEX this refers to the compressed input that produced the block,
    /// not an uncompressed file line. Header lines are excluded.
    /// A `NonAscii` failure additionally gives the line within the decoded block
    /// and its byte column; it is not a compressed-input position.
    pub record_line: usize,
    /// Decoding or duplicate-key outcome for this block.
    pub kind: ParsingDiagnosticKind,
}

/// Recoverable parser outcomes, independent of scientific eligibility.
#[derive(Debug)]
pub enum ParsingDiagnosticKind {
    /// The NAV block could not be decoded and was omitted from the record.
    /// `NavOrbitParsing` includes flag mapping and unsupported-definition errors,
    /// as well as invalid numeric input; the nested source identifies the cause.
    NavigationFailure(crate::ParsingError),
    /// The OBS block could not be decoded and was omitted from the record.
    ObservationFailure(crate::ParsingError),
    /// The last successfully decoded NAV record replaces its predecessor.
    DuplicateNavigation {
        key: crate::navigation::NavKey,
        conflicting: bool,
        /// One-based body line of the replaced record.
        previous_record_line: usize,
    },
    /// Same behavior as NAV duplicates; the key includes the epoch flag.
    DuplicateObservation {
        key: crate::observation::ObsKey,
        conflicting: bool,
    },
}

/// RINEX [Record] type, inner content is RINEX type dependent.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Record {
    /// [AntexRecord] contains antenna calibration profile
    AntexRecord(AntexRecord),

    /// [ClockRecord] contains SV and ground clock states
    ClockRecord(ClockRecord),

    /// Meteo sensor observations, stored as [MeteoRecord]
    MeteoRecord(MeteoRecord),

    /// Navigation messages stored as [NavRecord]
    NavRecord(NavRecord),

    /// Observation record: signals observation
    ObsRecord(ObservationRecord),
}

/// Record comments are high level informations, sorted by epoch
/// (timestamp) of appearance. We deduce the "associated" timestamp from the
/// previosuly parsed epoch, when parsing the record.
pub type Comments = BTreeMap<Epoch, Vec<String>>;

impl Record {
    /// [AntexRecord] unwrapping attempt.
    pub fn as_antex(&self) -> Option<&AntexRecord> {
        match self {
            Record::AntexRecord(r) => Some(r),
            _ => None,
        }
    }

    /// Mutable [AntexRecord] unwrapping attempt.
    pub fn as_mut_antex(&mut self) -> Option<&mut AntexRecord> {
        match self {
            Record::AntexRecord(r) => Some(r),
            _ => None,
        }
    }

    /// [ClockRecord] unwrapping attempt.
    pub fn as_clock(&self) -> Option<&ClockRecord> {
        match self {
            Record::ClockRecord(r) => Some(r),
            _ => None,
        }
    }

    /// Mutable [ClockRecord] unwrapping attempt.
    pub fn as_mut_clock(&mut self) -> Option<&mut ClockRecord> {
        match self {
            Record::ClockRecord(r) => Some(r),
            _ => None,
        }
    }

    /// [MeteoRecord] unwrapping attempt.
    pub fn as_meteo(&self) -> Option<&MeteoRecord> {
        match self {
            Record::MeteoRecord(r) => Some(r),
            _ => None,
        }
    }

    /// Mutable [MeteoRecord] unwrapping attempt.
    pub fn as_mut_meteo(&mut self) -> Option<&mut MeteoRecord> {
        match self {
            Record::MeteoRecord(r) => Some(r),
            _ => None,
        }
    }

    /// [NavRecord] unwrapping attempt.
    pub fn as_nav(&self) -> Option<&NavRecord> {
        match self {
            Record::NavRecord(r) => Some(r),
            _ => None,
        }
    }

    /// Mutable [NavRecord] unwrapping attempt.
    pub fn as_mut_nav(&mut self) -> Option<&mut NavRecord> {
        match self {
            Record::NavRecord(r) => Some(r),
            _ => None,
        }
    }

    /// [ObservationRecord] unwrapping attempt.
    pub fn as_obs(&self) -> Option<&ObservationRecord> {
        match self {
            Record::ObsRecord(r) => Some(r),
            _ => None,
        }
    }

    /// Mutable [ObservationRecord] unwrapping attempt.
    pub fn as_mut_obs(&mut self) -> Option<&mut ObservationRecord> {
        match self {
            Record::ObsRecord(r) => Some(r),
            _ => None,
        }
    }
}
