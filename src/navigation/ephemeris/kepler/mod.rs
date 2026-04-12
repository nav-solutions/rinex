use crate::prelude::{nav::Orbit, Constellation, Epoch, SV};

use crate::navigation::Ephemeris;

use anise::{
    constants::frames::IAU_EARTH_FRAME,
    math::{Vector3, Vector6},
};

mod helper;
pub use helper::Helper;

#[cfg(doc)]
use crate::bibliography::Bibliography;

/// [Kepler] stores all keplerian parameters
#[derive(Default, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Kepler {
    /// Semi major axis (in meters)
    pub a: f64,

    /// Eccentricity (n.a)
    pub e: f64,

    /// Inclination angle at reference time (in radians)
    pub i_0: f64,

    /// Longitude of ascending node at reference time (in radians)
    pub omega_0: f64,

    /// Mean anomaly at reference time (in radians)
    pub m_0: f64,

    /// Argument of perigee (in radians)
    pub omega: f64,

    /// Time of issue of ephemeris.
    /// NB GEO and GLO ephemerides do not have the notion of ToE, we set 0 here.
    /// Any calculations that imply ToE for those is incorrect anyways.
    pub toe: f64,
}

/// Orbit [Perturbations]
#[derive(Default, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Perturbations {
    /// Mean motion difference from computed value (in radians)
    pub dn: f64,

    /// Inclination rate of change (in radians.s⁻¹)
    pub i_dot: f64,

    /// Right ascension rate of change (in radians.s⁻¹)
    pub omega_dot: f64,

    /// Amplitude of sine harmonic correction term of the argument
    /// of latitude (in radians)
    pub cus: f64,

    /// Amplitude of cosine harmonic correction term of the argument
    /// of latitude (in radians)
    pub cuc: f64,

    /// Amplitude of sine harmonic correction term of the angle of inclination (in radians)
    pub cis: f64,

    /// Amplitude of cosine harmonic correction term of the angle of inclination (in radians)
    pub cic: f64,

    /// Amplitude of sine harmonic correction term of the orbit radius (in meters)
    pub crs: f64,

    /// Amplitude of cosine harmonic correction term of the orbit radius (in meters)
    pub crc: f64,
}

impl Ephemeris {
    /// Retrieves Orbit Keplerian parameters.
    /// This only applies to MEO Ephemerides, not GEO and Glonass.
    pub fn kepler(&self) -> Option<Kepler> {
        Some(Kepler {
            a: self.get_orbit_f64("sqrta")?.powf(2.0),
            e: self.get_orbit_f64("e")?,
            i_0: self.get_orbit_f64("i0")?,
            omega: self.get_orbit_f64("omega")?,
            omega_0: self.get_orbit_f64("omega0")?,
            m_0: self.get_orbit_f64("m0")?,
            toe: self.get_orbit_f64("toe")?,
        })
    }

    /// Creates new [Ephemeris] frame from [Kepler]ian parameters
    pub fn with_kepler(&self, kepler: Kepler) -> Self {
        let mut s = self.clone();
        s.set_orbit_f64("sqrta", kepler.a.sqrt());
        s.set_orbit_f64("e", kepler.e);
        s.set_orbit_f64("i0", kepler.i_0);
        s.set_orbit_f64("omega", kepler.omega);
        s.set_orbit_f64("omega0", kepler.omega_0);
        s.set_orbit_f64("m0", kepler.m_0);
        s.set_orbit_f64("toe", kepler.toe);
        s
    }

    /// Retrieves Orbit [Perturbations] from [Ephemeris]
    pub fn perturbations(&self) -> Option<Perturbations> {
        Some(Perturbations {
            cuc: self.get_orbit_f64("cuc")?,
            cus: self.get_orbit_f64("cus")?,
            cic: self.get_orbit_f64("cic")?,
            cis: self.get_orbit_f64("cis")?,
            crc: self.get_orbit_f64("crc")?,
            crs: self.get_orbit_f64("crs")?,
            dn: self.get_orbit_f64("deltaN")?,
            i_dot: self.get_orbit_f64("idot")?,
            omega_dot: self.get_orbit_f64("omegaDot")?,
        })
    }

    /// Creates new [Ephemeris] with desired Orbit [Perturbations]
    pub fn with_perturbations(&self, perturbations: Perturbations) -> Self {
        let mut s = self.clone();
        s.set_orbit_f64("cuc", perturbations.cuc);
        s.set_orbit_f64("cus", perturbations.cus);
        s.set_orbit_f64("cic", perturbations.cic);
        s.set_orbit_f64("cis", perturbations.cis);
        s.set_orbit_f64("crc", perturbations.crc);
        s.set_orbit_f64("crs", perturbations.crs);
        s.set_orbit_f64("deltaN", perturbations.dn);
        s.set_orbit_f64("idot", perturbations.i_dot);
        s.set_orbit_f64("omegaDot", perturbations.omega_dot);
        s
    }

    /// Returns total seconds elapsed in the timescale, between [Epoch] and ToE [Epoch].
    /// NB: this does not apply toe GEO [Ephemeris]
    fn t_k(&self, sv: SV, t: Epoch) -> Option<f64> {
        // guard against bad usage
        if sv.constellation.is_sbas() {
            return None;
        }

        let sv_ts = sv.timescale()?;
        let toe = self.toe(sv)?;
        let dt = t.to_time_scale(sv_ts) - toe;
        Some(dt.to_seconds())
    }

    /// Returns [SV] [Orbit]al state at t [Epoch].
    /// Self must be correctly selected from navigation record.
    /// See [Bibliography::AsceAppendix3], [Bibliography::JLe19] and [Bibliography::BeiDouICD]
    /// ## Input
    /// - sv: [SV] satellite identity
    /// - epoch: desired [Epoch]
    pub fn kepler2position(&self, sv: SV, epoch: Epoch) -> Option<Orbit> {
        if sv.constellation.is_sbas() || sv.constellation == Constellation::Glonass {
            let (x_km, y_km, z_km) = (
                self.get_orbit_f64("satPosX")?,
                self.get_orbit_f64("satPosY")?,
                self.get_orbit_f64("satPosZ")?,
            );
            // TODO: velocity + integration
            Some(Orbit::from_position(
                x_km,
                y_km,
                z_km,
                epoch,
                IAU_EARTH_FRAME,
            ))
        } else {
            let helper = self.helper(sv, epoch)?;
            let pos = helper.ecef_position();
            let vel = helper.ecef_velocity();
            Some(Orbit::from_cartesian_pos_vel(
                Vector6::new(pos[0], pos[1], pos[2], vel[0], vel[1], vel[2]),
                epoch,
                IAU_EARTH_FRAME,
            ))
        }
    }

    /// Calculates ECEF (position, velocity) [Vector3] duplet
    /// ## Input
    /// - sv: desired [SV]
    /// - epoch: desired [Epoch]
    /// ## Returns
    /// - (position, velocity): [Vector3] duplet, in (km, km/s)
    /// See [Bibliography::AsceAppendix3], [Bibliography::JLe19] and [Bibliography::BeiDouICD]
    pub fn kepler2position_velocity(&self, sv: SV, epoch: Epoch) -> Option<(Vector3, Vector3)> {
        // In gloass and SBAS scenarios,
        // we only need to pick up the values from the record.
        // NB: this is incorrect, it requires an integration process
        //    that has yet to be understood and implemented.
        //    SBAS navigation is not supported yet anyway
        if sv.constellation.is_sbas() || sv.constellation == Constellation::Glonass {
            let (x_km, y_km, z_km) = (
                self.get_orbit_f64("satPosX")?,
                self.get_orbit_f64("satPosY")?,
                self.get_orbit_f64("satPosZ")?,
            );
            let (vel_x_km, vel_y_km, vel_z_km) = (
                self.get_orbit_f64("velX")?,
                self.get_orbit_f64("velY")?,
                self.get_orbit_f64("velZ")?,
            );

            let position = Vector3::new(x_km, y_km, z_km);
            let velocity = Vector3::new(vel_x_km, vel_y_km, vel_z_km);
            Some((position, velocity))
        } else {
            // form keplerian helper
            let helper = self.helper(sv, epoch)?;
            helper.position_velocity()
        }
    }
}
