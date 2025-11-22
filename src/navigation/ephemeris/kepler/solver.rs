#[cfg(feature = "log")]
use log::{debug, error};

use crate::{
    navigation::Ephemeris,
    prelude::{Constellation, Epoch, SV},
};

use nalgebra::{Matrix3, Rotation, Rotation3, SMatrix, Vector4};

use anise::math::{Vector3, Vector6};

/// Kepler [Solver] to calculate satellite orbital states from radio messages.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Solver {
    /// Satellite identity as [SV].
    pub satellite: SV,

    /// The difference between the calculated time and the ephemeris reference time
    pub dt_seconds: f64,

    /// Ascending angle (corrected) in radians
    pub u_k: f64,

    /// Radius(corrected) in radians
    pub r_k: f64,

    /// Orbital inclination (corrected) in radians
    pub i_k: f64,

    /// Ascending node right ascension (in radians)
    pub omega_k: f64,

    /// First Derivative of Ascending angle(corrected)
    pub fd_u_k: f64,

    /// First Derivative of Radius(corrected)
    pub fd_r_k: f64,

    /// First Derivative of Orbital inclination(corrected)
    pub fd_i_k: f64,

    /// First Derivative of Ascending node right ascension
    pub fd_omega_k: f64,

    /// Relativistic Effect Correction
    pub dtr: f64,

    /// First Derivative of Relativistic Effect Correction
    pub fd_dtr: f64,

    /// r_sv in meters ECEF
    pub r_sv: (f64, f64, f64),
}

impl Solver {
    /// Returns ẍ and ÿ first temporal derivatives of orbital position.
    fn orbit_velocity(&self) -> (f64, f64) {
        let (sin_u_k, cos_u_k) = self.u_k.sin_cos();
        let fd_x = self.fd_r_k * cos_u_k - self.r_k * self.fd_u_k * sin_u_k;
        let fd_y = self.fd_r_k * sin_u_k + self.r_k * self.fd_u_k * cos_u_k;
        (fd_x, fd_y)
    }

    /// Returns MEO to ECEF [Rotation3] matrix
    fn meo_orbit_to_ecef_rotation_matrix(&self) -> Rotation<f64, 3> {
        // Positive angles mean counterclockwise rotation
        let rotation_x = Rotation3::from_axis_angle(&Vector3::x_axis(), self.i_k);
        let rotation_z = Rotation3::from_axis_angle(&Vector3::z_axis(), self.omega_k);
        rotation_z * rotation_x
    }

    /// Returns BDS-GEO to ECEF [Rotation3] matrix
    fn beidou_geo_to_ecef_rotation_matrix(&self) -> Rotation<f64, 3> {
        let omega_bds = 7.292115E-5_f64; // rotation velocity constant (from BDS ICD)
        let rotation_x = Rotation::from_axis_angle(&Vector3::x_axis(), 5.0f64.to_radians());
        let rotation_z =
            Rotation::from_axis_angle(&Vector3::z_axis(), -omega_bds * self.dt_seconds);
        rotation_z * rotation_x
    }

    /// Returns position in kilometers, in the case of a BDS-GEO orbit.
    fn beidou_geo_position_km(&self) -> Vector3 {
        let orbit_xyz = Vector3::new(self.r_sv.0, self.r_sv.1, 0.0);
        let rotation1 = self.meo_orbit_to_ecef_rotation_matrix();
        let rotation2 = self.beidou_geo_to_ecef_rotation_matrix();
        let ecef_xyz = rotation2 * rotation1 * orbit_xyz;
        ecef_xyz / 1000.0
    }

    /// Returns velocity in kilometers/second, in the case of a BDS-GEO orbit.
    fn beidou_geo_velocity_km(&self) -> Vector3 {
        let omega_bds = 7.292115E-5_f64; // rotation velocity constant (from BDS ICD)
        let (x, y, _) = self.r_sv;
        let (sin_omega_k, cos_omega_k) = self.omega_k.sin_cos();
        let (sin_i_k, cos_i_k) = self.i_k.sin_cos();
        let (fd_x, fd_y) = self.orbit_velocity();
        let fd_xgk = -y * self.fd_omega_k - fd_y * cos_i_k * sin_omega_k + fd_x * cos_omega_k;
        let fd_ygk = x * self.fd_omega_k + fd_y * cos_i_k * cos_omega_k + fd_x * sin_omega_k;
        let fd_zgk = fd_y * sin_i_k + y * self.fd_i_k * cos_i_k;

        let rx = Rotation3::from_axis_angle(&Vector3::x_axis(), 5.0);
        let rz = Rotation3::from_axis_angle(&Vector3::z_axis(), -omega_bds * self.dt_seconds);
        let (sin_omega_tk, cos_omega_tk) = (omega_bds * self.dt_seconds).sin_cos();

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
        let vel = fd_rz * rx * pos_km + rz * rx * fd_pos;
        vel
    }

    /// Returns ECEF position [Vector3] in km.
    pub fn position_km(&self) -> Option<Vector3> {
        let position_velocity_km = self.position_velocity_km()?;
        Some(Vector3::new(
            position_velocity_km[0],
            position_velocity_km[1],
            position_velocity_km[2],
        ))
    }

    /// Returns ECEF position and velocity as Vector6 in kilometers.
    pub fn position_velocity_km(&self) -> Option<Vector6> {
        if self.satellite.is_beidou_geo() {
            let (position_km, velocity_km) =
                (self.beidou_geo_position_km(), self.beidou_geo_velocity_km());
            Some(Vector6::new(
                position_km[0],
                position_km[1],
                position_km[2],
                velocity_km[0],
                velocity_km[1],
                velocity_km[2],
            ))
        } else {
            match self.satellite.constellation {
                Constellation::GPS | Constellation::Galileo | Constellation::BeiDou => {
                    let (x_m, y_m, z_m) = self.r_sv;
                    let pos_m =
                        self.meo_orbit_to_ecef_rotation_matrix() * Vector3::new(x_m, y_m, z_m);

                    let (fd_x, fd_y) = self.orbit_velocity();
                    let (sin_omega_k, cos_omega_k) = self.omega_k.sin_cos();
                    let (sin_i_k, cos_i_k) = self.i_k.sin_cos();

                    // first derivative of rotation Matrix
                    let mut fd_r = SMatrix::<f64, 3, 4>::zeros();
                    fd_r[(0, 0)] = cos_omega_k;
                    fd_r[(0, 1)] = -sin_omega_k * cos_i_k;
                    fd_r[(0, 2)] = -(x_m * sin_omega_k + y_m * cos_omega_k * cos_i_k);
                    fd_r[(0, 3)] = y_m * sin_omega_k * sin_i_k;
                    fd_r[(1, 0)] = sin_omega_k;
                    fd_r[(1, 1)] = cos_omega_k * cos_i_k;
                    fd_r[(1, 2)] = x_m * cos_omega_k - y_m * sin_omega_k * cos_i_k;
                    fd_r[(1, 3)] = y_m * cos_omega_k * sin_i_k;
                    fd_r[(2, 1)] = sin_i_k;
                    fd_r[(2, 3)] = y_m * cos_i_k;

                    let vel_m = fd_r * Vector4::new(fd_x, fd_y, self.fd_omega_k, self.fd_i_k);

                    Some(Vector6::new(
                        pos_m[0] / 1000.0,
                        pos_m[1] / 1000.0,
                        pos_m[2] / 1000.0,
                        vel_m[0] / 1000.0,
                        vel_m[1] / 1000.0,
                        vel_m[2] / 1000.0,
                    ))
                },
                _ => {
                    #[cfg(feature = "log")]
                    error!("solver: {} is not supported", self.satellite.constellation);
                    None
                },
            }
        }
    }
}

impl Ephemeris {
    /// Deploy a keplerian [Solver] to resolve navigation equations.
    /// This applies to all but Glonass and SBAS satellites.
    ///
    /// ## Input
    /// - satellite: [SV]
    /// - epoch: navigation [Epoch]
    /// - max_iteration: maximal number of iteration allowed to reasonnably converge.
    ///
    /// ## Output
    /// - [Solver]
    pub(crate) fn solver(
        &self,
        satellite: SV,
        epoch: Epoch,
        max_iteration: usize,
    ) -> Option<Solver> {
        // gravitational constant
        let gm_m3_s2 = match satellite.constellation {
            Constellation::BeiDou => 3.986004418E14_f64,
            Constellation::Glonass => 3.9860044E14_f64,
            Constellation::Galileo => 3.986004418E14_f64,
            _ => 3.9860050E14_f64, // from GPS ICD
        };

        // rotation velocity constant
        let omega = match satellite.constellation {
            Constellation::BeiDou => 7.292115E-5_f64,
            Constellation::Glonass => 7.292115E-5_f64,
            Constellation::Galileo => 7.2921151467E-5_f64,
            _ => 7.2921151467E-5_f64, // from GPS ICD
        };

        // relativistic correction
        // - 2 * sqrt(gm) / c / c
        let dtr_f = match satellite.constellation {
            Constellation::BeiDou => -0.00000000044428073090439775_f64,
            Constellation::Galileo => -0.00000000044428073090439775_f64,
            _ => -0.000000000444280763339306_f64, // from GPS ICD
        };

        // obtain helper structures
        let mut keplerian = self.to_keplerian(satellite)?;

        let dt_seconds = keplerian.dt(epoch).to_seconds();

        // apply the semi-major axis correction if any
        if let Some(a_dot_m_s) = self.cnav_adot_m_s() {
            keplerian.sma_m += a_dot_m_s * dt_seconds;
        }

        let sqrt_sma_m = keplerian.sma_m.sqrt();

        let n0 = (gm_m3_s2 / keplerian.sma_m.powi(3)).sqrt(); // average angular velocity
        let n = n0 + keplerian.dn_rad; // corrected mean angular velocity
        let m_k = keplerian.ma_rad + n * dt_seconds; // average anomaly

        // Iterative calculation of e_k
        let mut e_k;
        let mut i = 0;
        let mut e_k_lst = 0.0f64;

        loop {
            if i > max_iteration {
                #[cfg(feature = "log")]
                error!(
                    "({}) solver: reached maximal number of iterations",
                    satellite
                );
                return None;
            }

            e_k = m_k + keplerian.ecc * e_k_lst.sin();

            if (e_k - e_k_lst).abs() < 1e-10 {
                break;
            }

            i += 1;
            e_k_lst = e_k;
        }

        // true anomaly
        let (sin_e_k, cos_e_k) = e_k.sin_cos();
        let v_k = ((1.0 - keplerian.ecc.powi(2)).sqrt() * sin_e_k).atan2(cos_e_k - keplerian.ecc);

        let phi_k = v_k + keplerian.aop_rad; // latitude argument
        let (x2_sin_phi_k, x2_cos_phi_k) = (2.0 * phi_k).sin_cos();

        // latitude argument correction
        let du_k = keplerian.cus_rad * x2_sin_phi_k + keplerian.cuc_rad * x2_cos_phi_k;
        let u_k = phi_k + du_k;

        // orbital radius correction
        let dr_k = keplerian.crs_m * x2_sin_phi_k + keplerian.crc_m * x2_cos_phi_k;
        let r_k = keplerian.sma_m * (1.0 - keplerian.ecc * e_k.cos()) + dr_k;

        // inclination angle correction
        let di_k = keplerian.cis_rad * x2_sin_phi_k + keplerian.cic_rad * x2_cos_phi_k;

        // first derivatives
        let fd_omega_k = keplerian.omega_dot_rad_s - omega;

        let fd_e_k = n / (1.0 - keplerian.ecc * e_k.cos());
        let fd_phi_k = ((1.0 + keplerian.ecc) / (1.0 - keplerian.ecc)).sqrt()
            * ((v_k / 2.0).cos() / (e_k / 2.0).cos()).powi(2)
            * fd_e_k;

        let fd_u_k =
            (keplerian.cus_rad * x2_cos_phi_k - keplerian.cuc_rad * x2_sin_phi_k) * fd_phi_k * 2.0
                + fd_phi_k;

        let fd_r_k = keplerian.sma_m * keplerian.ecc * e_k.sin() * fd_e_k
            + 2.0 * (keplerian.crs_m * x2_cos_phi_k - keplerian.crc_m * x2_sin_phi_k) * fd_phi_k;

        let fd_i_k = keplerian.i_dot_rad_s
            + 2.0
                * (keplerian.cis_rad * x2_cos_phi_k - keplerian.cic_rad * x2_sin_phi_k)
                * fd_phi_k;

        // relativistic effect correction
        let dtr = dtr_f * keplerian.ecc * sqrt_sma_m * e_k.sin();
        let fd_dtr = dtr_f * keplerian.ecc * sqrt_sma_m * e_k.cos() * fd_e_k;

        // ascending node longitude correction (RAAN ?)
        let omega_k = if satellite.is_beidou_geo() {
            // BeiDou [IGSO]
            keplerian.longan_rad + keplerian.omega_dot_rad_s * dt_seconds
                - omega * keplerian.epoch.duration.to_seconds()
        } else {
            // GPS, Galileo, BeiDou [MEO]
            keplerian.longan_rad + (keplerian.omega_dot_rad_s - omega) * dt_seconds
                - omega * keplerian.epoch.duration.to_seconds()
        };

        // corrected inclination angle
        let i_k = keplerian.inc_rad + di_k + keplerian.i_dot_rad_s * dt_seconds;

        // position in orbital plane
        let (x, y) = (r_k * u_k.cos(), r_k * u_k.sin());
        let r_sv = (x, y, 0.0);

        debug!(
            "({}) dt={}s - omega_k={} - i_k={} - r_sv=({}, {})",
            satellite, dt_seconds, omega_k, i_k, x, y
        );

        Some(Solver {
            satellite,
            dt_seconds,
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
