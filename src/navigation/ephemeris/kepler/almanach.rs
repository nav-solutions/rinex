#[cfg(feature = "protos")]
use gnss_protos::gps::lnav::GpsQzssLnavAlmanach;

use crate::navigation::ephemeris::rad_to_semicircles;

/// Almanach keplerian parameters we can reconstruct
/// for a given satellite, from the satellite keplerian parameters
#[derive(Default, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Almanach {
    /// satellite ID
    pub sv_id: u8,

    /// clock offset (s)
    pub af0: f64,

    /// Clock drift (s.s⁻¹)
    pub af1: f64,

    /// Semi major axis (in meters)
    pub a: f64,

    /// Eccentricity (n.a)
    pub e: f64,

    /// Correction to inclination, relative to 0.3 semi-circles (in radians)
    pub di: f64,

    /// Longitude of ascending node at reference time (in radians)
    pub omega_0: f64,

    /// Mean anomaly at reference time (in radians)
    pub m_0: f64,

    /// Argument of perigee (in radians)
    pub omega: f64,

    /// Rate of right ascension (in radians.s⁻¹)
    pub omega_dot: f64,

    /// Time of issue of almanach.
    pub toa_seconds: u32,
}

impl Almanach {
    /// Converts this [Almanach] to quantizable [GpsQzssLnavAlmanach]
    /// which can be transmitted as per GPS/QZSS protocol.
    #[cfg(feature = "protos")]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "nav", feature = "protos"))))]
    pub fn to_gps_qzss_lnav(&self, health: u8) -> GpsQzssLnavAlmanach {
        GpsQzssLnavAlmanach {
            af0: self.af0,
            af1: self.af1,
            data_id: 0, // TODO ?
            sv_id: self.sv_id,
            sv_health: health,
            sqrt_a: self.a.sqrt(),
            eccentricity: self.e,
            toa_seconds: self.toa_seconds,
            m0: rad_to_semicircles(self.m_0),
            di: rad_to_semicircles(self.di),
            omega: rad_to_semicircles(self.omega),
            omega0: rad_to_semicircles(self.omega_0),
            omega_dot: rad_to_semicircles(self.omega_dot),
        }
    }
}
