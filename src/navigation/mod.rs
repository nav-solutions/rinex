//! Navigation module
mod earth_orientation;
mod ephemeris;
mod frame;
mod header;
mod ionosphere;
mod message;
mod parsing;
mod time;

pub mod rinex;

pub(crate) mod formatting;

pub(crate) use formatting::format;
pub(crate) use parsing::{is_new_epoch, parse_epoch};

pub use crate::navigation::{
    earth_orientation::EarthOrientation,
    ephemeris::{flags::*, orbits::OrbitItem, Ephemeris},
    frame::{NavFrame, NavFrameType},
    header::HeaderFields,
    ionosphere::{BdModel, IonosphereModel, KbModel, KbRegionCode, NgModel, NgRegionFlags},
    message::NavMessageType,
    time::TimeOffset,
};

#[cfg(feature = "nav")]
pub use crate::navigation::ephemeris::kepler::{Helper, Kepler, Perturbations};

#[cfg(feature = "processing")]
pub(crate) mod mask; // mask Trait implementation

#[cfg(feature = "processing")]
pub(crate) mod decim; // decim Trait implementation

#[cfg(feature = "processing")]
pub(crate) mod repair; // repair Trait implementation

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use crate::prelude::{Epoch, SV};

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NavKey {
    /// Time of Clock (ToC) as [Epoch].
    pub epoch: Epoch,

    /// [SV] broadcasting this information.
    pub sv: SV,

    /// [NavMessageType] associated to following [NavFrame]
    pub msgtype: NavMessageType,

    /// [NavFrame] type following.
    pub frmtype: NavFrameType,
}

/// Navigation data are [NavFrame]s indexed by [NavKey].
/// [NavKey] contains everything that is required to store & index a [NavFrame].
/// Several types of frames may exist (in modern RINEX). Refer to following types.
pub type Record = BTreeMap<NavKey, NavFrame>;
