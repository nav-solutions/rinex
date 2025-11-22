use crate::prelude::{nav::Orbit, Constellation, Epoch, SV, Duration};

use crate::navigation::Ephemeris;

use anise::{
    constants::frames::IAU_EARTH_FRAME,
    math::{Vector3, Vector6},
};

mod solver;
pub use helper::Helper;

#[cfg(doc)]
use crate::bibliography::Bibliography;

/// [Keplerian] stores and describes all keplerian parameters needed
/// for satellite based navigation, described by 
/// GPS, QZSS, Galileo and BDS radio messages.
/// This structure does not apply to Glonass nor SBAS navigation.
#[derive(Default, Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Keplerian {
    /// Semi major axis, in kilometers.
    pub sma_km: f64,

    /// Eccentricity (n.a)
    pub ecc: f64,

    /// Inclination at reference time,  (in radians.
    pub inc_rad: f64,

    /// Longitude of ascending node at reference time, in radians.
    pub lan_rad: f64,

    /// Mean anomaly at reference time, in radians.
    pub ma_rad: f64,

    /// Argument of perigee, in radians.
    pub aop_rad: f64,

    /// Reference [Epoch].
    pub epoch: f64,

    /// Earth orbit [Perturbations], due to gravity and tidal effects.
    pub perturbations: Perturbations,
}
    
impl Keplerian {
    /// Returns [Duration] between provided [Epoch] and reference [Epoch], as required
    /// by keplerian calculations.
    pub(crate) fn t_k(&self, epoch: Epoch) -> Duration {
        // convert to correct timescale (if need be)
        let epoch = if epoch.timescale != self.epoch.timescale {
            epoch.to_time_scale(self.epoch.timescale) 
        } else {
            epoch
        };

        epoch - self.epoch
    }
}

/// Earth orbit [Perturbations], as described by radio messages,
/// to correct the [Keplerian] model to realitiy (gravity effects, tidal effects..).
#[derive(Default, Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Perturbations {
    /// Mean motion difference, in radians.
    pub dn_rad: f64,

    /// Inclination rate of change, in radians.s⁻¹.
    pub i_dot_rad_s: f64,

    /// Right ascension rate of change (in radians.s⁻¹)
    pub raomega_dot: f64,

    /// Amplitude of sine harmonic correction term of the argument
    /// of latitude (in radians)
    pub cus_rad: f64,

    /// Amplitude of cosine harmonic correction term of the argument
    /// of latitude (in radians)
    pub cuc_rad: f64,

    /// Amplitude of sine harmonic correction term of the angle of inclination (in radians)
    pub cis_rad: f64,

    /// Amplitude of cosine harmonic correction term of the angle of inclination (in radians)
    pub cic_rad: f64,

    /// Amplitude of sine harmonic correction term of the orbit radius (in meters)
    pub crs_m: f64,

    /// Amplitude of cosine harmonic correction term of the orbit radius (in meters)
    pub crc_m: f64,
}

impl Ephemeris {
    /// Groups all keplerian parameters as [Keplerian], ready to
    /// be used in radio based navigation. This does not apply to Glonass
    /// nor SBAS satellites.
    pub fn to_keplerian(&self) -> Option<Keplerian> {
        Some(Keplerian {
            sma_km: {
                let sma_m = self.get_orbit_f64("sqrta")?.powf(2.0);
                sma_m * 1.0e-3
            },
            ecc: self.get_orbit_f64("e")?,
            inc_rad: self.get_orbit_f64("i0")?,
            omega: self.get_orbit_f64("omega")?,
            omega_0: self.get_orbit_f64("omega0")?,
            m_0: self.get_orbit_f64("m0")?,
            epoch: self.get_orbit_f64("toe")?,
            perturbations: self.perturbations()?,
        })
    }

    /// Copies and returns an [Ephemeris] with updated [Keplerian] parameters.
    pub fn with_keplerian(mut self,     keplerian: Keplerian) -> Self {
        self.set_orbit_f64("sqrta",     keplerian.a.sqrt());
        self.set_orbit_f64("e",         keplerian.ecc);
        self.set_orbit_f64("i0",        keplerian.inc_rad);
        self.set_orbit_f64("omega",     keplerian.omega);
        self.set_orbit_f64("omega0",    keplerian.omega_0);
        self.set_orbit_f64("m0",        keplerian.m_0);
        self.set_orbit_f64("toe",       keplerian.epoch);
        self.set_orbit_f64("cuc",       keplerian.perturbations.cuc_rad);
        self.set_orbit_f64("cus",       keplerian.perturbations.cus_rad);
        self.set_orbit_f64("cic",       keplerian.perturbations.cic_rad);
        self.set_orbit_f64("cis",       keplerian.perturbations.cis_rad);
        self.set_orbit_f64("crc",       keplerian.perturbations.crc_m);
        self.set_orbit_f64("crs",       keplerian.perturbations.crs_m);
        self.set_orbit_f64("deltaN",    keplerian.perturbations.dn_rad);
        self.set_orbit_f64("idot",      keplerian.perturbations.i_dot_rad_s);
        self.set_orbit_f64("omegaDot",  keplerian.perturbations.omega_dot_rad_s);
        self
    }

    /// Retrieves keplerian orbit [Perturbations] for Earth model.
    fn perturbations(&self) -> Option<Perturbations> {
        Some(Perturbations {
            cuc_rad: self.get_orbit_f64("cuc")?,
            cus_rad: self.get_orbit_f64("cus")?,
            cic_rad: self.get_orbit_f64("cic")?,
            cis_rad: self.get_orbit_f64("cis")?,
            crc_m: self.get_orbit_f64("crc")?,
            crs_m: self.get_orbit_f64("crs")?,
            dn_rad: self.get_orbit_f64("deltaN")?,
            omega_dot: self.get_orbit_f64("omegaDot")?,
            i_dot_rad_s: self.get_orbit_f64("idot")?,
        })
    }

    /// Resolves satellite orbital state, expressed at [Orbit] at desired [Epoch].
    ///
    /// ## Input
    /// - satellite: [SV] 
    /// - epoch: [Epoch] of navigation
    ///
    /// ## Output
    /// - state expressed as [Orbit].
    pub fn orbital_state(&self, satellite: SV, epoch: Epoch) -> Option<Orbit> {
        let pos_vel_km = self.position_velocity_ecef_km()?;
        Some(Orbit::from_cartesian_pos_vel(
            pos_vel_km,
            epoch,
            IAU_EARTH_FRAME,
        ))
    }

    /// Resolves satellite position at desired [Epoch], expressed as ECEF coordinates in kilometers.
    pub fn position_ecef_km(&self, satellite: SV, epoch: Epoch) -> Option<Vector3> {
        let pos_vel_km = self.to_position_velocity_ecef_km(satellite, epoch)?;
        Vector3::new(pos_vel_km[0], pos_vel_km[1], pos_vel_km[2])
    }
    
    /// Resolves satellite position and velocityn at desired [Epoch], expressed as ECEF coordinates in kilometers.
    pub fn position_velocity_ecef_km(&self, satellite: SV, epoch: Epoch) -> Option<Vector6> {
        if satellite.constellation.is_sbas() || satellite.constellation == Constellation::Glonass {
            let (x_km, y_km, z_km) = (
                self.get_orbit_f64("satPosX")?,
                self.get_orbit_f64("satPosY")?,
                self.get_orbit_f64("satPosZ")?,
            );
            let (velx_km, vely_km, velz_km) = (
                self.get_orbit_f64("velX")?,
                self.get_orbit_f64("velY")?,
                self.get_orbit_f64("velZ")?,
            );
        } else {
            let solver = self.solver(satellite, epoch)?;
            let (pos, vel) = (solver.ecef_position(), solver.ecef_velocity());
            Vector6::new(pos[0], pos[1], pos[2], vel[0], vel[1], vel[2])
        }
    }
}
