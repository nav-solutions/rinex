use crate::{
    antex::Record as AntexRecord, clock::Record as ClockRecord, meteo::Record as MeteoRecord,
    navigation::Record as NavRecord, observation::Record as ObservationRecord, prelude::Epoch,
};

use std::collections::BTreeMap;

#[cfg(feature = "serde")]
use serde::Serialize;

mod formatting;
mod parsing;

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
