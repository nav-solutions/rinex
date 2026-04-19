//! Meteo RINEX module
mod formatting;
mod header;
mod observables;
mod parsing;
mod rinex;
mod sensor;

pub use header::HeaderFields;
pub use observables::MeteoObservable;
pub use sensor::Sensor;

use crate::prelude::{Epoch, Observable};
use std::collections::BTreeMap;

pub(crate) use formatting::format;
pub(crate) use parsing::{is_new_epoch, parse_epoch};

#[cfg(feature = "processing")]
pub(crate) mod mask; // mask Trait implementation

#[cfg(feature = "processing")]
pub(crate) mod decim; // decim Trait implementation

#[cfg(feature = "processing")]
pub(crate) mod repair; // repair Trait implementation

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// [MeteoKey] is used to index meteo sensor measurements,
/// described in Meteo RINEX files.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MeteoKey {
    /// [Epoch] of sensor sampling.
    pub epoch: Epoch,

    /// [MeteoObservable] defines the physics and how to interpret
    /// the sensor measurement (including unit and scaling).
    pub observable: MeteoObservable,
}

/// Measurements, indexed by [MeteoKey].
/// The measurement unit is defined by the attached [Observable].
pub type Record = BTreeMap<MeteoKey, f64>;
