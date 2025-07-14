use crate::{
    navigation::ephemeris::{Ephemeris, EphemerisError},
    prelude::{nav::Orbit, Constellation, Epoch, SV},
};

use anise::{
    astro::AzElRange,
    constants::frames::EARTH_J2000,
    math::{Vector3, Vector6},
    prelude::Almanac,
};

mod solver;

#[cfg(doc)]
use crate::bibliography::Bibliography;

/// [Kepler] stores all keplerian parameters
#[derive(Default, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Kepler {
    /// Semi major axis (in meters)
    pub semi_major_m: f64,

    /// Eccentricity
    pub eccentricity: f64,

    /// Inclination angle at reference time (in radians)
    pub i0_rad: f64,

    /// Longitude of ascending node at reference time (in radians)
    pub omega0_rad: f64,

    /// Mean anomaly at reference time (in radians)
    pub m0_rad: f64,

    /// Argument of perigee (in radians)
    pub omega_rad: f64,

    /// Time of issue of ephemeris.
    pub toe_s: f64,
}

/// Orbit [Perturbations] used in keplerian calculations
#[derive(Default, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Perturbations {
    /// Mean motion difference from computed value (in radians)
    pub dn_rad: f64,

    /// Inclination rate of change (in radians.s⁻¹)
    pub idot_rad_s: f64,

    /// Right ascension rate of change (in radians.s⁻¹)
    pub omegadot_rad_s: f64,

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
    /// Retrieves Orbit Keplerian parameters.
    /// This only applies to MEO [Ephemeris], not GEO and Glonass.
    fn to_kepler(&self) -> Option<Kepler> {
        Some(Kepler {
            semi_major_m: self.get_orbit_field_f64("sqrta")?.powf(2.0),
            eccentricity: self.get_orbit_field_f64("e")?,
            i0_rad: self.get_orbit_field_f64("i0")?,
            omega_rad: self.get_orbit_field_f64("omega")?,
            omega0_rad: self.get_orbit_field_f64("omega0")?,
            m0_rad: self.get_orbit_field_f64("m0")?,
            toe_s: self.get_orbit_field_f64("toe")?,
        })
    }

    // /// Creates new [Ephemeris] frame from [Kepler]ian parameters
    // fn with_kepler(&self, kepler: Kepler) -> Self {
    //     let mut s = self.clone();
    //     s.set_orbit_field_f64("sqrta", kepler.semi_major_m.sqrt());
    //     s.set_orbit_field_f64("e", kepler.eccentricity);
    //     s.set_orbit_field_f64("i0", kepler.i0_rad);
    //     s.set_orbit_field_f64("omega", kepler.omega_rad);
    //     s.set_orbit_field_f64("omega0", kepler.omega0_rad);
    //     s.set_orbit_field_f64("m0", kepler.m0_rad);
    //     s.set_orbit_field_f64("toe", kepler.toe_s);
    //     s
    // }

    /// Retrieves Orbit [Perturbations] from [Ephemeris]
    fn to_perturbations(&self) -> Option<Perturbations> {
        Some(Perturbations {
            cuc_rad: self.get_orbit_field_f64("cuc")?,
            cus_rad: self.get_orbit_field_f64("cus")?,
            cic_rad: self.get_orbit_field_f64("cic")?,
            cis_rad: self.get_orbit_field_f64("cis")?,
            crc_m: self.get_orbit_field_f64("crc")?,
            crs_m: self.get_orbit_field_f64("crs")?,
            dn_rad: self.get_orbit_field_f64("deltaN")?,
            idot_rad_s: self.get_orbit_field_f64("idot")?,
            omegadot_rad_s: self.get_orbit_field_f64("omegaDot")?,
        })
    }

    // /// Creates new [Ephemeris] with desired Orbit [Perturbations]
    // fn with_perturbations(&self, perturbations: Perturbations) -> Self {
    //     let mut s = self.clone();
    //     s.set_orbit_field_f64("cuc", perturbations.cuc_rad);
    //     s.set_orbit_field_f64("cus", perturbations.cus_rad);
    //     s.set_orbit_field_f64("cic", perturbations.cic_rad);
    //     s.set_orbit_field_f64("cis", perturbations.cis_rad);
    //     s.set_orbit_field_f64("crc", perturbations.crc_m);
    //     s.set_orbit_field_f64("crs", perturbations.crs_m);
    //     s.set_orbit_field_f64("deltaN", perturbations.dn_rad);
    //     s.set_orbit_field_f64("idot", perturbations.idot_rad_s);
    //     s.set_orbit_field_f64("omegaDot", perturbations.omegadot_rad_s);
    //     s
    // }

    /// Resolves ECEF (position, velocity) [Vector3]s using Kepler equations.
    ///
    /// ## Input
    /// - sv: desired [SV]
    /// - toc: Time of clock as [Epoch]
    /// - epoch: [Epoch]
    /// - max_iter: maximal number of iterations in the solver
    ///
    /// ## Returns
    /// - resolved (position, velocity) as [Vector3] duplet, in kilometers,
    /// kilometers/s.
    ///
    /// - position (x, y, z), velocity (x, y, z) as [Vector6], in kilometers,
    /// kilometers/s.
    /// See [Bibliography::AsceAppendix3], [Bibliography::JLe19] and [Bibliography::BeiDouICD]
    pub fn sv_position_velocity_km(
        &self,
        sv: SV,
        toc: Epoch,
        epoch: Epoch,
        max_iter: usize,
    ) -> Result<Vector6, EphemerisError> {
        if sv.constellation.is_sbas() || sv.constellation == Constellation::Glonass {
            // position
            let pos_km = Vector3::new(
                self.get_orbit_field_f64("posX")
                    .ok_or(EphemerisError::MissingData)?,
                self.get_orbit_field_f64("posY")
                    .ok_or(EphemerisError::MissingData)?,
                self.get_orbit_field_f64("posZ")
                    .ok_or(EphemerisError::MissingData)?,
            );

            // velocity is mandatory
            let vel_km = Vector3::new(
                self.get_orbit_field_f64("velX")
                    .ok_or(EphemerisError::MissingData)?,
                self.get_orbit_field_f64("velY")
                    .ok_or(EphemerisError::MissingData)?,
                self.get_orbit_field_f64("velZ")
                    .ok_or(EphemerisError::MissingData)?,
            );

            // use accel when it exists
            let accel_km = match (
                self.get_orbit_field_f64("accelX"),
                self.get_orbit_field_f64("accelY"),
                self.get_orbit_field_f64("accelZ"),
            ) {
                (Some(accel_x_km), Some(accel_y_km), Some(accel_z_km)) => {
                    Vector3::new(accel_x_km, accel_y_km, accel_z_km)
                },
                _ => Vector3::new(0.0, 0.0, 0.0),
            };

            let dt = (toc - epoch).to_seconds();

            let mut r = pos_km;
            r += vel_km * dt;
            r += 0.5 * accel_km * dt.powi(2);

            return Ok(Vector6::new(r[0], r[1], r[2], 0.0, 0.0, 0.0));
        }

        // Kepler solver
        let toe = self.toe(sv)?;
        let solver = self.to_keplerian_solver(sv, toe, epoch, max_iter)?;
        solver.position_velocity_km()
    }

    /// Resolves satellite [Orbit]al state at selected [Epoch].
    /// See [Bibliography::AsceAppendix3], [Bibliography::JLe19] and [Bibliography::BeiDouICD]
    ///
    /// ## Inputs
    /// - sv: satellite identity as [SV]
    /// - toc: Time of clock as [Epoch]
    /// - epoch: [Epoch]
    /// - max_iter: maximal number of iterations in the solver
    ///
    /// ## Returns
    /// - [Orbit], [EphemerisError]
    pub fn sv_orbit(
        &self,
        sv: SV,
        toc: Epoch,
        epoch: Epoch,
        max_iter: usize,
    ) -> Result<Orbit, EphemerisError> {
        let pos_vel_km = self.sv_position_velocity_km(sv, toc, epoch, max_iter)?;

        Ok(Orbit::from_cartesian_pos_vel(
            pos_vel_km,
            epoch,
            EARTH_J2000,
        ))
    }

    /// Resolves this [SV] attitude angles as seen from RX position.
    ///
    /// ## Inputs
    /// - sv: satellite identity as [SV]
    /// - toc: Time of clock as [Epoch]
    /// - epoch: [Epoch]
    /// - almanac: [Almanac] definition
    /// - rx_position_km: observation position in kilometers ECEF
    /// - max_iter: maximal number of iterations in the solver
    ///
    /// ## Returns
    /// - [AzElRange], [EphemerisError]
    pub fn sv_elevation_azimuth_range(
        &self,
        sv: SV,
        toc: Epoch,
        epoch: Epoch,
        almanac: &Almanac,
        rx_position_km: (f64, f64, f64),
        max_iter: usize,
    ) -> Result<AzElRange, EphemerisError> {
        let sv_orbit = self.sv_orbit(sv, toc, epoch, max_iter)?;
        let (x_km, y_km, z_km) = rx_position_km;
        let rx_orbit = Orbit::from_position(x_km, y_km, z_km, epoch, EARTH_J2000);

        match almanac.azimuth_elevation_range_sez(rx_orbit, sv_orbit, None, None) {
            Ok(alzerange) => Ok(alzerange),
            Err(e) => Err(EphemerisError::AlmanacError(e)),
        }
    }
}
