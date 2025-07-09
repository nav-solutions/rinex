use crate::{
    constants::{Constants, Omega},
    navigation::ephemeris::{Ephemeris, EphemerisError},
    prelude::{Constellation, Epoch, SV},
};

use nalgebra::{Matrix3, Rotation, Rotation3, SMatrix, Vector4};

use anise::math::{Vector3, Vector6};

/// Kepler equations [Solver], to calculate satellites position.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Solver {
    /// Satellite identity as [SV].
    sv: SV,

    /// Time difference (in seconds)
    t_k: f64,

    /// Corrected ascending angle (rad)
    u_k: f64,

    /// Corrected radius (rad)
    r_k: f64,

    /// Orbital inclination (corrected) in radians
    i_k: f64,

    /// Ascending node right ascension (in radians)
    omega_k: f64,

    /// First derivative of corrected ascending angle (rad/s)
    fd_u_k: f64,

    /// First derivative of corrected radius (rad/s)
    fd_r_k: f64,

    /// First derivative of corrected inclination (rad/s)
    fd_i_k: f64,

    /// First derivative of ascending node right ascension (rad/s)
    fd_omega_k: f64,

    /// Relativistic effect correction (meters)
    dtr: f64,

    /// First derivative of relativistic effect correction (meters/s)
    fd_dtr: f64,

    /// r_sv in meters ECEF
    r_sv: (f64, f64, f64),
}

impl Solver {
    /// Returns MEO to ECEF [Rotation3] matrix
    fn meo_orbit_to_ecef_rotation_matrix(&self) -> Rotation3<f64> {
        // Positive angles mean counterclockwise rotation
        let rotation_x = Rotation3::from_axis_angle(&Vector3::x_axis(), self.i_k);
        let rotation_z = Rotation3::from_axis_angle(&Vector3::z_axis(), self.omega_k);
        rotation_z * rotation_x
    }

    /// Returns GEO to ECEF [Rotation3] matrix
    fn geo_orbit_to_ecef_rotation_matrix(&self) -> Rotation3<f64> {
        let rotation_x = Rotation::from_axis_angle(&Vector3::x_axis(), 5.0f64.to_radians());
        let rotation_z = Rotation::from_axis_angle(&Vector3::z_axis(), -Omega::BDS * self.t_k);
        rotation_z * rotation_x
    }

    /// Returns ẍ and ÿ temporal derivative
    fn orbit_velocity(&self) -> (f64, f64) {
        let (sin_u_k, cos_u_k) = self.u_k.sin_cos();
        let fd_x = self.fd_r_k * cos_u_k - self.r_k * self.fd_u_k * sin_u_k;
        let fd_y = self.fd_r_k * sin_u_k + self.r_k * self.fd_u_k * cos_u_k;
        (fd_x, fd_y)
    }

    /// Returns MEO ECEF position in kilometers.
    fn meo_position_km(&self) -> Vector3 {
        let (x, y, z) = self.r_sv;
        let orbit_xyz = Vector3::new(x, y, z);
        let ecef_xyz = self.meo_orbit_to_ecef_rotation_matrix() * orbit_xyz;
        ecef_xyz / 1000.0
    }

    /// Returns ECEF velocity [Vector3] in kilometers/s.
    fn meo_velocity_km(&self) -> Vector3 {
        let (x, y, _) = self.r_sv;
        let (sin_omega_k, cos_omega_k) = self.omega_k.sin_cos();
        let (sin_i_k, cos_i_k) = self.i_k.sin_cos();
        // First Derivative of orbit position
        let (fd_x, fd_y) = self.orbit_velocity();
        // First Derivative of rotation Matrix

        let mut fd_r = SMatrix::<f64, 3, 4>::zeros();
        fd_r[(0, 0)] = cos_omega_k;
        fd_r[(0, 1)] = -sin_omega_k * cos_i_k;
        fd_r[(0, 2)] = -(x * sin_omega_k + y * cos_omega_k * cos_i_k);
        fd_r[(0, 3)] = y * sin_omega_k * sin_i_k;
        fd_r[(1, 0)] = sin_omega_k;
        fd_r[(1, 1)] = cos_omega_k * cos_i_k;
        fd_r[(1, 2)] = x * cos_omega_k - y * sin_omega_k * cos_i_k;
        fd_r[(1, 3)] = y * cos_omega_k * sin_i_k;
        fd_r[(2, 1)] = sin_i_k;
        fd_r[(2, 3)] = y * cos_i_k;

        let rhs = Vector4::new(fd_x, fd_y, self.fd_omega_k, self.fd_i_k);
        let vel = fd_r * rhs;
        vel / 1000.0
    }

    /// Returns ECEF position [Vector3] in km, for BeiDou GEO specifically
    pub fn beidou_geo_position_km(&self) -> Vector3 {
        let orbit_xyz = Vector3::new(self.r_sv.0, self.r_sv.1, 0.0);
        let rotation1 = self.meo_orbit_to_ecef_rotation_matrix();
        let rotation2 = self.geo_orbit_to_ecef_rotation_matrix();
        let ecef_xyz = rotation2 * rotation1 * orbit_xyz;
        ecef_xyz / 1000.0
    }

    /// Returns ECEF velocity [Vector3] in km/s, for BeiDou GEO specifically
    pub fn beidou_geo_velocity_km(&self) -> Vector3 {
        let (x, y, _) = self.r_sv;
        let (sin_omega_k, cos_omega_k) = self.omega_k.sin_cos();
        let (sin_i_k, cos_i_k) = self.i_k.sin_cos();

        let (fd_x, fd_y) = self.orbit_velocity();
        let fd_xgk = -y * self.fd_omega_k - fd_y * cos_i_k * sin_omega_k + fd_x * cos_omega_k;
        let fd_ygk = x * self.fd_omega_k + fd_y * cos_i_k * cos_omega_k + fd_x * sin_omega_k;
        let fd_zgk = fd_y * sin_i_k + y * self.fd_i_k * cos_i_k;

        let rx = Rotation3::from_axis_angle(&Vector3::x_axis(), 5.0);
        let rz = Rotation3::from_axis_angle(&Vector3::z_axis(), -Omega::BDS * self.t_k);

        let (sin_omega_tk, cos_omega_tk) = (Omega::BDS * self.t_k).sin_cos();
        let fd_rz = self.fd_omega_k
            * Matrix3::new(
                -sin_omega_tk,
                cos_omega_tk,
                0.0,
                -cos_omega_tk,
                -sin_omega_tk,
                0.0,
                0.0,
                0.0,
                0.0,
            );

        let pos = self.beidou_geo_position_km();
        let fd_pos = Vector3::new(fd_xgk, fd_ygk, fd_zgk);
        let vel = fd_rz * rx * pos + rz * rx * fd_pos;
        vel
    }

    /// Returns ECEF (position, velocity) [Vector3]s in (km, km/s), for BeiDou GEO specifically.
    pub fn beidou_geo_position_velocity_km(&self) -> (Vector3, Vector3) {
        let (x, y, _) = self.r_sv;
        let (sin_i_k, cos_i_k) = self.i_k.sin_cos();
        let (sin_omega_k, cos_omega_k) = self.omega_k.sin_cos();

        let (fd_x, fd_y) = self.orbit_velocity();

        let fd_xgk = -y * self.fd_omega_k - fd_y * cos_i_k * sin_omega_k + fd_x * cos_omega_k;
        let fd_ygk = x * self.fd_omega_k + fd_y * cos_i_k * cos_omega_k + fd_x * sin_omega_k;
        let fd_zgk = fd_y * sin_i_k + y * self.fd_i_k * cos_i_k;

        let rx = Rotation3::from_axis_angle(&Vector3::x_axis(), 5.0);
        let rz = Rotation3::from_axis_angle(&Vector3::z_axis(), -Omega::BDS * self.t_k);

        let (sin_omega_tk, cos_omega_tk) = (Omega::BDS * self.t_k).sin_cos();

        let fd_rz = self.fd_omega_k
            * Matrix3::new(
                -sin_omega_tk,
                cos_omega_tk,
                0.0,
                -cos_omega_tk,
                -sin_omega_tk,
                0.0,
                0.0,
                0.0,
                0.0,
            );

        let pos_km = self.beidou_geo_position_km();
        let fd_pos = Vector3::new(fd_xgk, fd_ygk, fd_zgk);
        let vel_km = fd_rz * rx * pos_km + rz * rx * fd_pos;

        (pos_km, vel_km)
    }

    /// Returns ECEF position and velocity Vector in kilometers.
    pub fn position_velocity_km(&self) -> Result<Vector6, EphemerisError> {
        let (pos_km, vel_km) = match self.sv.constellation {
            Constellation::GPS | Constellation::Galileo => {
                (self.meo_position_km(), self.meo_velocity_km())
            },
            Constellation::BeiDou => {
                if self.sv.is_beidou_geo() {
                    (self.beidou_geo_position_km(), self.beidou_geo_velocity_km())
                } else if self.sv.is_beidou_geo() {
                    // TODO
                    return Err(EphemerisError::BeidouIgsoNotSupported);
                } else {
                    (self.meo_position_km(), self.meo_velocity_km())
                }
            },
            constell => {
                return Err(EphemerisError::NotSupported(constell));
            },
        };

        Ok(Vector6::new(
            pos_km[0], pos_km[1], pos_km[2], vel_km[0], vel_km[1], vel_km[2],
        ))
    }
}

impl Ephemeris {
    /// Obtain a [Solver] to solve Kepler equations.
    /// This is only compatible with MEO [Ephemeris], and should
    /// never be invoked on GEO / SBAS / Glonass [Ephemeris].
    ///
    /// ## Inputs
    /// - sv: satellite identity as [SV]
    /// - toe: ToE as [Epoch] expressed in any [Timescale]
    /// - epoch: [Epoch] expressed in any [Timescale]
    pub(crate) fn to_keplerian_solver(
        &self,
        sv: SV,
        toe: Epoch,
        epoch: Epoch,
        max_iter: usize,
    ) -> Result<Solver, EphemerisError> {
        // const
        let gm_m3_s2 = Constants::gm(sv);
        let omega = Constants::omega(sv);
        let dtr_f = Constants::dtr_f(sv);

        let timescale = match sv.timescale() {
            Some(ts) => ts,
            None => {
                return Err(EphemerisError::NotSupported(sv.constellation));
            },
        };

        // translate if need be
        let toe = toe.to_time_scale(timescale);
        let epoch = epoch.to_time_scale(timescale);

        let t_k = (epoch - toe).to_seconds();

        let mut kepler = self.to_kepler().ok_or(EphemerisError::MissingData)?;

        let perturbations = self.to_perturbations().ok_or(EphemerisError::MissingData)?;

        // considering the filed a_dot
        if let Some(a_dot) = self.a_dot() {
            kepler.semi_major_m += a_dot * t_k;
        }

        let n0 = (gm_m3_s2 / kepler.semi_major_m.powi(3)).sqrt(); // average angular velocity
        let n = n0 + perturbations.dn_rad; // corrected mean angular velocity
        let m_k = kepler.m0_rad + n * t_k; // average anomaly

        // Iterative calculation of e_k
        let mut e_k_lst: f64 = 0.0;
        let mut e_k;
        let mut i = 0;

        loop {
            e_k = m_k + kepler.eccentricity * e_k_lst.sin();

            if (e_k - e_k_lst).abs() < 1e-10 {
                break;
            }

            i += 1;
            e_k_lst = e_k;

            if i == max_iter {
                return Err(EphemerisError::Diverged);
            }
        }

        // true anomaly
        let (sin_e_k, cos_e_k) = e_k.sin_cos();

        let v_k = ((1.0 - kepler.eccentricity.powi(2)).sqrt() * sin_e_k)
            .atan2(cos_e_k - kepler.eccentricity);

        let phi_k = v_k + kepler.omega_rad; // latitude argument
        let (x2_sin_phi_k, x2_cos_phi_k) = (2.0 * phi_k).sin_cos();

        // latitude argument correction
        let du_k = perturbations.cus_rad * x2_sin_phi_k + perturbations.cuc_rad * x2_cos_phi_k;
        let u_k = phi_k + du_k;

        // orbital radius correction
        let dr_k = perturbations.crs_m * x2_sin_phi_k + perturbations.crc_m * x2_cos_phi_k;
        let r_k = kepler.semi_major_m * (1.0 - kepler.eccentricity * e_k.cos()) + dr_k;

        // inclination angle correction
        let di_k = perturbations.cis_rad * x2_sin_phi_k + perturbations.cic_rad * x2_cos_phi_k;

        // first derivatives
        let fd_omega_k = perturbations.omegadot_rad_s - omega;

        let fd_e_k = n / (1.0 - kepler.eccentricity * e_k.cos());

        let fd_phi_k = ((1.0 + kepler.eccentricity) / (1.0 - kepler.eccentricity)).sqrt()
            * ((v_k / 2.0).cos() / (e_k / 2.0).cos()).powi(2)
            * fd_e_k;

        let fd_u_k = (perturbations.cus_rad * x2_cos_phi_k - perturbations.cuc_rad * x2_sin_phi_k)
            * fd_phi_k
            * 2.0
            + fd_phi_k;

        let fd_r_k = kepler.semi_major_m * kepler.eccentricity * e_k.sin() * fd_e_k
            + 2.0
                * (perturbations.crs_m * x2_cos_phi_k - perturbations.crc_m * x2_sin_phi_k)
                * fd_phi_k;

        let fd_i_k = perturbations.idot_rad_s
            + 2.0
                * (perturbations.cis_rad * x2_cos_phi_k - perturbations.cic_rad * x2_sin_phi_k)
                * fd_phi_k;

        // relativistic effect correction
        let dtr = dtr_f * kepler.eccentricity * kepler.semi_major_m.sqrt() * e_k.sin();
        let fd_dtr = dtr_f * kepler.eccentricity * kepler.semi_major_m.sqrt() * e_k.cos() * fd_e_k;

        // ascending node longitude correction (RAAN ?)
        let omega_k = if sv.is_beidou_geo() {
            // BeiDou IGSO
            kepler.omega0_rad + perturbations.omegadot_rad_s * t_k - omega * kepler.toe_s
        } else {
            // GPS, Galileo, BeiDou MEO
            kepler.omega0_rad + (perturbations.omegadot_rad_s - omega) * t_k - omega * kepler.toe_s
        };

        // corrected inclination angle
        let i_k = kepler.i0_rad + di_k + perturbations.idot_rad_s * t_k;

        // position in orbital plane
        let (x, y) = (r_k * u_k.cos(), r_k * u_k.sin());

        let r_sv = (x, y, 0.0);

        Ok(Solver {
            sv,
            t_k,
            omega_k,
            dtr,
            fd_dtr,
            u_k,
            i_k,
            fd_u_k,
            r_k,
            fd_r_k,
            fd_i_k,
            fd_omega_k,
            r_sv,
        })
    }
}
