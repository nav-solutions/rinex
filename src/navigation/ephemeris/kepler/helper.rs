#[cfg(feature = "log")]
use log::{error, warn};

use crate::{
    constants::{Constants, Omega},
    navigation::Ephemeris,
    prelude::{Constellation, Epoch, SV},
};

use nalgebra::{Matrix3, Rotation, Rotation3, SMatrix, Vector4};

use anise::math::Vector3;

/// [Helper] helps calcualte satellite orbital state from Keplerian elements.
#[derive(Debug, Clone, Copy)]
pub struct Helper {
    /// [SV] satellite identity
    pub sv: SV,

    /// The difference between the calculated time and the ephemeris reference time
    pub t_k: f64,

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

impl Helper {
    /// Returns MEO to ECEF [Rotation3] matrix
    fn meo_orbit_to_ecef_rotation_matrix(&self) -> Rotation<f64, 3> {
        // Positive angles mean counterclockwise rotation
        let rotation_x = Rotation3::from_axis_angle(&Vector3::x_axis(), self.i_k);
        let rotation_z = Rotation3::from_axis_angle(&Vector3::z_axis(), self.omega_k);
        rotation_z * rotation_x
    }

    /// Returns GEO to ECEF [Rotation3] matrix
    fn geo_orbit_to_ecef_rotation_matrix(&self) -> Rotation<f64, 3> {
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

    /// Calculate ecef position [km].
    pub fn ecef_position(&self) -> Vector3 {
        if self.sv.is_beidou_geo() {
            self.beidou_geo_ecef_position()
        } else {
            let (x, y, z) = self.r_sv;
            let orbit_xyz = Vector3::new(x, y, z);
            let ecef_xyz = self.meo_orbit_to_ecef_rotation_matrix() * orbit_xyz;
            ecef_xyz / 1000.0
        }
    }

    /// Returns ECEF velocity [Vector3] in km/s.
    pub fn ecef_velocity(&self) -> Vector3 {
        if self.sv.is_beidou_geo() {
            self.beidou_geo_ecef_velocity()
        } else {
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
    }

    /// Returns ECEF (position, velocity) [Vector3] in (km, km/s).
    pub fn ecef_pv(&self) -> (Vector3, Vector3) {
        (self.ecef_position(), self.ecef_velocity())
    }

    /// Returns ECEF position [Vector3] in km, for BeiDou GEO specifically
    pub fn beidou_geo_ecef_position(&self) -> Vector3 {
        let orbit_xyz = Vector3::new(self.r_sv.0, self.r_sv.1, 0.0);
        let rotation1 = self.meo_orbit_to_ecef_rotation_matrix();
        let rotation2 = self.geo_orbit_to_ecef_rotation_matrix();
        let ecef_xyz = rotation2 * rotation1 * orbit_xyz;
        ecef_xyz / 1000.0
    }

    /// Returns ECEF velocity [Vector3] in km/s, for BeiDou GEO specifically
    pub fn beidou_geo_ecef_velocity(&self) -> Vector3 {
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
        let pos = self.beidou_geo_ecef_position();
        let fd_pos = Vector3::new(fd_xgk, fd_ygk, fd_zgk);
        let vel = fd_rz * rx * pos + rz * rx * fd_pos;
        vel
    }

    /// Returns ECEF (position, velocity) [Vector3]s in (km, km/s), for BeiDou GEO specifically.
    pub fn beidou_geo_ecef_pv(&self) -> (Vector3, Vector3) {
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
        let pos = self.beidou_geo_ecef_position();
        let fd_pos = Vector3::new(fd_xgk, fd_ygk, fd_zgk);
        let vel = fd_rz * rx * pos + rz * rx * fd_pos;
        (pos, vel)
    }

    /// Returns ECEF position [Vector3] in km.
    pub fn position(&self) -> Option<Vector3> {
        match self.sv.constellation {
            Constellation::GPS | Constellation::Galileo => Some(self.ecef_position()),
            Constellation::BeiDou => {
                if self.sv.is_beidou_geo() {
                    Some(self.beidou_geo_ecef_position())
                } else {
                    Some(self.ecef_position())
                }
            },
            _ => {
                #[cfg(feature = "log")]
                warn!("{} is not supported", self.sv.constellation);
                None
            },
        }
    }

    /// Returns (position, velocity) [Vector3] duplet, in (km, km/s).
    pub fn position_velocity(&self) -> Option<(Vector3, Vector3)> {
        if self.sv.is_beidou_geo() {
            Some(self.beidou_geo_ecef_pv())
        } else {
            match self.sv.constellation {
                Constellation::GPS | Constellation::Galileo | Constellation::BeiDou => {
                    Some(self.ecef_pv())
                },
                _ => {
                    #[cfg(feature = "log")]
                    warn!("{} is not supported", self.sv.constellation);
                    None
                },
            }
        }
    }
}

impl Ephemeris {
    /// Try to form obtain a [Helper] for Keplerian equations solving.
    /// This will fail on Glonass and SBAS constellations.
    pub fn helper(&self, sv: SV, t: Epoch) -> Option<Helper> {
        // const
        let gm_m3_s2 = Constants::gm(sv);
        let omega = Constants::omega(sv);
        let dtr_f = Constants::dtr_f(sv);

        let t_k = self.t_k(sv, t)?;

        let mut kepler = self.kepler()?;
        let perturbations = self.perturbations()?;

        // considering the filed a_dot
        if let Some(a_dot) = self.a_dot() {
            kepler.a += a_dot * t_k;
        }

        let n0 = (gm_m3_s2 / kepler.a.powi(3)).sqrt(); // average angular velocity
        let n = n0 + perturbations.dn; // corrected mean angular velocity
        let m_k = kepler.m_0 + n * t_k; // average anomaly

        // Iterative calculation of e_k
        let mut e_k_lst: f64 = 0.0;
        let mut e_k;
        let mut i = 0;

        loop {
            e_k = m_k + kepler.e * e_k_lst.sin();
            if (e_k - e_k_lst).abs() < 1e-10 {
                break;
            }
            i += 1;
            e_k_lst = e_k;
        }

        if i >= Constants::MAX_KEPLER_ITER {
            #[cfg(feature = "log")]
            error!("{} kepler iteration overflow", sv);
        }

        // true anomaly
        let (sin_e_k, cos_e_k) = e_k.sin_cos();
        let v_k = ((1.0 - kepler.e.powi(2)).sqrt() * sin_e_k).atan2(cos_e_k - kepler.e);

        let phi_k = v_k + kepler.omega; // latitude argument
        let (x2_sin_phi_k, x2_cos_phi_k) = (2.0 * phi_k).sin_cos();

        // latitude argument correction
        let du_k = perturbations.cus * x2_sin_phi_k + perturbations.cuc * x2_cos_phi_k;
        let u_k = phi_k + du_k;

        // orbital radisu correction
        let dr_k = perturbations.crs * x2_sin_phi_k + perturbations.crc * x2_cos_phi_k;
        let r_k = kepler.a * (1.0 - kepler.e * e_k.cos()) + dr_k;

        // inclination angle correction
        let di_k = perturbations.cis * x2_sin_phi_k + perturbations.cic * x2_cos_phi_k;

        // first derivatives
        let fd_omega_k = perturbations.omega_dot - omega;

        let fd_e_k = n / (1.0 - kepler.e * e_k.cos());
        let fd_phi_k = ((1.0 + kepler.e) / (1.0 - kepler.e)).sqrt()
            * ((v_k / 2.0).cos() / (e_k / 2.0).cos()).powi(2)
            * fd_e_k;

        let fd_u_k =
            (perturbations.cus * x2_cos_phi_k - perturbations.cuc * x2_sin_phi_k) * fd_phi_k * 2.0
                + fd_phi_k;

        let fd_r_k = kepler.a * kepler.e * e_k.sin() * fd_e_k
            + 2.0
                * (perturbations.crs * x2_cos_phi_k - perturbations.crc * x2_sin_phi_k)
                * fd_phi_k;

        let fd_i_k = perturbations.i_dot
            + 2.0
                * (perturbations.cis * x2_cos_phi_k - perturbations.cic * x2_sin_phi_k)
                * fd_phi_k;

        // relativistic effect correction
        let dtr = dtr_f * kepler.e * kepler.a.sqrt() * e_k.sin();
        let fd_dtr = dtr_f * kepler.e * kepler.a.sqrt() * e_k.cos() * fd_e_k;

        // ascending node longitude correction (RAAN ?)
        let omega_k = if sv.is_beidou_geo() {
            // BeiDou [IGSO]
            kepler.omega_0 + perturbations.omega_dot * t_k - omega * kepler.toe
        } else {
            // GPS, Galileo, BeiDou [MEO]
            kepler.omega_0 + (perturbations.omega_dot - omega) * t_k - omega * kepler.toe
        };

        // corrected inclination angle
        let i_k = kepler.i_0 + di_k + perturbations.i_dot * t_k;

        // position in orbital plane
        let (x, y) = (r_k * u_k.cos(), r_k * u_k.sin());

        let r_sv = (x, y, 0.0);

        Some(Helper {
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
