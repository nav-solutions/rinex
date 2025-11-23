use crate::prelude::{nav::Orbit, Constellation, Duration, Epoch, SV};

use crate::navigation::{Ephemeris, EphemerisError};

use anise::{
    constants::frames::IAU_EARTH_FRAME,
    math::{Vector3, Vector6},
};

mod solver;

/// [Keplerian] stores and describes all keplerian parameters needed
/// for satellite based navigation, described by
/// GPS, QZSS, Galileo and BDS radio messages.
/// This structure does not apply to Glonass nor SBAS navigation.
#[derive(Default, Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Keplerian {
    /// Reference [Epoch].
    pub epoch: Epoch,

    /// Semi major axis, in meters.
    pub sma_m: f64,

    /// Eccentricity.
    pub ecc: f64,

    /// Inclination at reference time, in radians.
    pub inc_rad: f64,

    /// Longitude of ascending node at reference time, in radians.
    pub longan_rad: f64,

    /// Mean anomaly at reference time, in radians.
    pub ma_rad: f64,

    /// Argument of perigee, in radians.
    pub aop_rad: f64,

    /// Mean motion difference, in radians.
    pub dn_rad: f64,

    /// Inclination rate of change, in radians.s⁻¹.
    pub i_dot_rad_s: f64,

    /// Right ascension rate of change (in radians.s⁻¹)
    pub omega_dot_rad_s: f64,

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

impl Keplerian {
    /// Returns [Duration] between provided [Epoch] and reference [Epoch], as required
    /// by keplerian calculations.
    pub(crate) fn dt(&self, epoch: Epoch) -> Duration {
        // convert to correct timescale, if need be
        let epoch = if epoch.time_scale != self.epoch.time_scale {
            epoch.to_time_scale(self.epoch.time_scale)
        } else {
            epoch
        };

        epoch - self.epoch
    }
}

impl Ephemeris {
    /// Groups all keplerian parameters as [Keplerian], ready to
    /// be used in radio based navigation. This does not apply to Glonass
    /// nor SBAS satellites.
    pub fn to_keplerian(&self, satellite: SV) -> Result<Keplerian, EphemerisError> {
        let (crs_m, crc_m) = self.harmonic_correction_rsin_rcos()?;
        let (cis_rad, cic_rad) = self.harmonic_correction_isin_icos()?;
        let (cus_rad, cuc_rad) = self.harmonic_correction_usin_ucos()?;

        Ok(Keplerian {
            crc_m,
            crs_m,
            cic_rad,
            cis_rad,
            cuc_rad,
            cus_rad,
            epoch: self.toe(satellite)?,
            ecc: self.eccentricity()?,
            sma_m: self.semi_major_axis_m()?,
            ma_rad: self.mean_anomaly_rad()?,
            inc_rad: self.inclination_rad()?,
            aop_rad: self.argument_of_perigee_rad()?,
            dn_rad: self.mean_motion_difference_rad()?,
            longan_rad: self.longitude_ascending_node_rad()?,
            i_dot_rad_s: self.inclination_rate_of_change_rad_s()?,
            omega_dot_rad_s: self.right_ascension_rate_of_change_rad_s()?,
        })
    }

    /// Copies and returns an [Ephemeris] with updated [Keplerian] parameters.
    pub fn with_keplerian(&self, keplerian: Keplerian) -> Self {
        let mut s = self.clone();
        s.set_orbit_f64("sqrta", keplerian.sma_m.sqrt());
        s.set_orbit_f64("e", keplerian.ecc);
        s.set_orbit_f64("i0", keplerian.inc_rad);
        s.set_orbit_f64("omega", keplerian.aop_rad);
        s.set_orbit_f64("omega0", keplerian.longan_rad);
        s.set_orbit_f64("m0", keplerian.ma_rad);

        let toe = keplerian.epoch.to_time_of_week().1 as f64;
        s.set_orbit_f64("toe", toe);

        s.set_orbit_f64("cuc", keplerian.cuc_rad);
        s.set_orbit_f64("cus", keplerian.cus_rad);
        s.set_orbit_f64("cic", keplerian.cic_rad);
        s.set_orbit_f64("cis", keplerian.cis_rad);
        s.set_orbit_f64("crc", keplerian.crc_m);
        s.set_orbit_f64("crs", keplerian.crs_m);
        s.set_orbit_f64("deltaN", keplerian.dn_rad);
        s.set_orbit_f64("idot", keplerian.i_dot_rad_s);
        s.set_orbit_f64("omegaDot", keplerian.omega_dot_rad_s);
        s
    }

    /// Resolves satellite orbital state, expressed at [Orbit] at desired [Epoch].
    /// Depending on the constellation, this involves two strategies:
    /// - deploying the kepler solver for GPS, QZSS, BDS and Galileo constellations
    /// - deploying the satellite position integrator for Glonass and SBAS satellites.
    ///
    /// ## Input
    /// - satellite: [SV]
    /// - epoch: [Epoch] of navigation
    /// - max_iteration: maximal number of iteration allowed to reasonnably converge.
    ///
    /// ## Output
    /// - state expressed as [Orbit].
    pub fn resolve_orbital_state(
        &self,
        satellite: SV,
        epoch: Epoch,
        max_iteration: usize,
    ) -> Result<Orbit, EphemerisError> {
        let pos_vel_km = self.resolve_position_velocity_km(satellite, epoch, max_iteration)?;
        Ok(Orbit::from_cartesian_pos_vel(
            pos_vel_km,
            epoch,
            IAU_EARTH_FRAME,
        ))
    }

    /// Resolves satellite position at desired [Epoch], expressed as ECEF coordinates in kilometers.
    /// Depending on the constellation, this involves two strategies:
    /// - deploying the kepler solver for GPS, QZSS, BDS and Galileo constellations
    /// - deploying the satellite position integrator for Glonass and SBAS satellites.
    /// - max_iteration: maximal number of iteration allowed to reasonnably converge.
    ///
    /// ## Input
    /// - satellite: [SV]
    /// - epoch: [Epoch] of navigation
    /// - max_iteration: maximal number of iteration allowed to reasonnably converge.
    ///
    /// ## Output
    /// - ECEF position as [Vector3].
    pub fn resolve_position_km(
        &self,
        satellite: SV,
        epoch: Epoch,
        max_iteration: usize,
    ) -> Result<Vector3, EphemerisError> {
        let pos_vel_km = self.resolve_position_velocity_km(satellite, epoch, max_iteration)?;
        Ok(Vector3::new(pos_vel_km[0], pos_vel_km[1], pos_vel_km[2]))
    }

    /// Resolves satellite position and velocityn at desired [Epoch], expressed as ECEF coordinates in kilometers.
    /// Depending on the constellation, this involves two strategies:
    /// - deploying the kepler solver for GPS, QZSS, BDS and Galileo constellations
    /// - deploying the satellite position integrator for Glonass and SBAS satellites.
    ///
    /// ## Input
    /// - satellite: [SV]
    /// - epoch: [Epoch] of navigation
    /// - max_iteration: maximal number of iteration allowed to reasonnably converge.
    ///
    /// ## Output
    /// - ECEF position and velocity as [Vector6].
    pub fn resolve_position_velocity_km(
        &self,
        satellite: SV,
        epoch: Epoch,
        max_iteration: usize,
    ) -> Result<Vector6, EphemerisError> {
        if satellite.constellation.is_sbas() || satellite.constellation == Constellation::Glonass {
            let (x_km, y_km, z_km) = (
                self.get_orbit_field_f64("posX")?,
                self.get_orbit_field_f64("posY")?,
                self.get_orbit_field_f64("posZ")?,
            );
            let (velx_km, vely_km, velz_km) = (
                self.get_orbit_field_f64("velX")?,
                self.get_orbit_field_f64("velY")?,
                self.get_orbit_field_f64("velZ")?,
            );

            Ok(Vector6::new(x_km, y_km, z_km, velx_km, vely_km, velz_km)) // TODO: wrong
        } else {
            let solver = self.solver(satellite, epoch, max_iteration)?;
            solver.position_velocity_km()
        }
    }
}
