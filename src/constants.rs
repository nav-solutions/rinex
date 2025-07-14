#[cfg(feature = "nav")]
use crate::prelude::{Constellation, SV};

#[cfg(feature = "nav")]
pub(crate) struct GM;

#[cfg(feature = "nav")]
impl GM {
    pub const GPS: f64 = 3.9860050E14;
    pub const BDS: f64 = 3.986004418E14;
    pub const GLO: f64 = 3.9860044E14;
    pub const GAL: f64 = 3.986004418E14;
}

#[cfg(feature = "nav")]
pub(crate) struct Omega;

#[cfg(feature = "nav")]
impl Omega {
    pub const GPS: f64 = 7.2921151467E-5;
    pub const BDS: f64 = 7.292115E-5;
    pub const GLO: f64 = 7.292115E-5;
    pub const GAL: f64 = 7.2921151467E-5;
}

/// - 2 * sqrt(gm) / c / c
#[allow(dead_code)]
pub(crate) struct DtrF;
#[allow(dead_code)]
impl DtrF {
    pub const GPS: f64 = -0.000000000444280763339306;
    pub const BDS: f64 = -0.00000000044428073090439775;
    pub const GAL: f64 = -0.00000000044428073090439775;
}

/// Const values used in RINEX crate
#[cfg(feature = "nav")]
pub(crate) struct Constants;

#[cfg(feature = "nav")]
impl Constants {
    /// Earth gravitation.
    pub const fn gm(sv: SV) -> f64 {
        match sv.constellation {
            Constellation::BeiDou => GM::BDS,
            Constellation::Galileo => GM::GAL,
            Constellation::Glonass => GM::GLO,
            _ => GM::GPS,
        }
    }

    /// Earth rotation rate
    pub const fn omega(sv: SV) -> f64 {
        match sv.constellation {
            Constellation::BeiDou => Omega::BDS,
            Constellation::Galileo => Omega::GAL,
            Constellation::Glonass => Omega::GLO,
            _ => Omega::GPS,
        }
    }

    /// Auxiliary Quantities for Calculating Relativistic Effects in Clock Correction
    pub const fn dtr_f(sv: SV) -> f64 {
        match sv.constellation {
            Constellation::GPS => DtrF::GPS,
            Constellation::BeiDou => DtrF::BDS,
            Constellation::Galileo => DtrF::GAL,
            _ => DtrF::GPS,
        }
    }
}
