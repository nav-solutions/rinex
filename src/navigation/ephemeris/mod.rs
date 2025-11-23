mod formatting;
mod parsing;

pub mod orbits;

/// Ephemeris NAV flags definitions & support
pub mod flags;

use orbits::OrbitItem;
use thiserror::Error;

use flags::{
    bds::{BdsHealth, BdsSatH1},
    glonass::{GlonassHealth, GlonassHealth2},
    gps::GpsQzssl1cHealth,
};

#[cfg(feature = "log")]
use log::error;

#[cfg(feature = "nav")]
#[cfg_attr(docsrs, doc(cfg(feature = "nav")))]
pub mod kepler;

#[cfg(feature = "nav")]
use crate::prelude::nav::Almanac;

#[cfg(feature = "ublox")]
mod ublox;

#[cfg(feature = "nav")]
use anise::{
    astro::AzElRange,
    errors::{AlmanacError, AlmanacResult},
    math::{Vector3, Vector6},
    prelude::{Frame, Orbit},
};

use std::collections::HashMap;

use crate::prelude::{Constellation, Duration, Epoch, TimeScale, SV};

#[derive(Error, Debug)]
pub enum EphemerisError {
    /// Invalid Ephemeris operation.
    /// For example, requesting ToE for Glonass or Geosat is not not possible.
    #[error("invalid ephemeris operation")]
    BadOperation,

    /// Constellation not supported.
    #[error("{0}: ephemeris not supported")]
    NotSupported(Constellation),

    /// Solver diverged: did not converge in specified amount of cycles.
    #[error("kepler solver did not converge")]
    Diverged,

    /// One or several data fields missing from record: cannot proceed.
    #[error("missing data")]
    MissingData,

    #[error("BDS-IGSO not supported yet")]
    BeidouIgsoNotSupported,

    #[error("({0}:{1}): failed to select an ephemeris frame")]
    FrameSelectionError(Epoch, SV),

    #[cfg(feature = "nav")]
    #[error("almanac error: {0}")]
    AlmanacError(#[from] AlmanacError),
}

/// Ephemeris Navigation message. May be found in all RINEX revisions.
/// Describes the content of the radio message at publication time.
/// Usually published at midnight and regularly updated with respect
/// to [Ephemeris] validity period.
///
/// Any [Ephemeris] comes with the description of the on-board clock,
/// but other data fields are [Constellation] and RINEX version dependent.
/// We store them as dictionary of [OrbitItem]s. This dictionary
/// is parsed based on our built-in JSON descriptor, it proposes methods
/// to access raw data or higher level methods for types that we can interpret.
/// Refer to [OrbitItem] for more information.
///
/// RINEX V3 example:
/// ```
/// use rinex::{
///     prelude::Rinex,
///     navigation::{NavFrameType, NavMessageType},
/// };
///
/// let rinex = Rinex::from_gzip_file("data/NAV/V3/BRDC00GOP_R_20210010000_01D_MN.rnx.gz")
///     .unwrap();
///
/// // You can always unwrap inner structures manually and access everything.
/// // But we propose higher level iteration methods to make things easier:
/// for (key, ephemeris) in rinex.nav_ephemeris_frames_iter() {
///     
///     let toc = key.epoch;
///     let sv_broadcaster = key.sv;
///     let sv_timescale = key.sv.constellation.timescale();
///
///     // we support most GNSS [Timescale]s completely.
///     // But incomplete support prohibits most Ephemeris exploitation.
///     if sv_timescale.is_none() {
///         continue;
///     }
///
///     let sv_timescale = sv_timescale.unwrap();
///
///     // until RINEXv3 (included) you can only find this type of frame
///     assert_eq!(key.frmtype, NavFrameType::Ephemeris);
///
///     // until RINEXv3 (included) you can only find this kind of message
///     assert_eq!(key.msgtype, NavMessageType::LNAV);
///
///     assert_eq!(toc.time_scale, sv_timescale); // always true in NAV RINEX
///
///     // Ephemeris serves many purposes and applications, so
///     // it has a lot to offer.
///
///     // ToE is most important when considering a frame.
///     // When missing (blanked), the frame should be discarded.
///     if let Some(toe) = ephemeris.toe(sv_broadcaster) {
///     
///     }
///
///     if let Some(tgd) = ephemeris.total_group_delay() {
///         // TGD was found & interpreted as duration
///         let tgd = tgd.total_nanoseconds();
///     }
///
///     // SV Health highest interpretation level: as simple boolean
///     if !ephemeris.satellite_is_healthy() {
///         // should most likely be ignored in navigation processing
///     }
///
///     // finer health interpretation is constellation dependent.
///     // Refer to RINEX standards and related constellation ICD.
///     if let Some(health) = ephemeris.orbits.get("health") {
///         if let Some(gps_qzss_l1l2l5) = health.as_gps_qzss_l1l2l5_health_flag() {
///             assert!(gps_qzss_l1l2l5.healthy());
///         }
///     }
///
///     // other example: l2p flag in GPS messages
///     if let Some(l2p) = ephemeris.orbits.get("l2p") {
///         let flag = l2p.as_gps_l2p_flag().unwrap();
///         assert!(flag); // P2(Y) streams LNAV message
///     }
///
///     // on "nav" feature (heavy) we integrate the kepler solver
///     // that can resolve the coordinates of the SV using this very frame.
///     // You still have to manage your ephemeris frames correctly.
///     // This is just an example.
///     if let Some(orbital_state) = ephemeris.kepler2position(sv_broadcaster, toc) {
///         // continue with [Orbit] processing
///     }
/// }
/// ```
///
/// Working with other RINEX revisions does not change anything
/// when dealing with this type, unless maybe the data fields you may
/// find the dictionary. For example, RINEX v4 describes beta-testing
/// health flags for BDS vehicles:
///
/// ```
/// use rinex::{
///     prelude::Rinex,
///     navigation::{NavFrameType, NavMessageType, bds::BdsHealth},
/// };
///
/// let rinex = Rinex::from_gzip_file("data/NAV/V4/BRD400DLR_S_20230710000_01D_MN.rnx.gz")
///     .unwrap();
///
/// // You can always unwrap inner structures manually and access everything.
/// // But we propose higher level iteration methods to make things easier:
/// for (key, ephemeris) in rinex.nav_ephemeris_frames_iter() {
///
///     if let Some(health) = ephemeris.orbits.get("health") {
///         // health flag found & possibly interpreted
///         // this for example, only applies to modern BDS messages
///         if let Some(flag) = health.as_bds_health_flag() {
///             if flag == BdsHealth::UnhealthyTesting {
///             }
///         }
///     }
/// }    
/// ```
#[derive(Default, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Ephemeris {
    /// Clock bias (in seconds)
    pub clock_bias: f64,

    /// Clock drift (s.s⁻¹)
    pub clock_drift: f64,

    /// Clock drift rate (s.s⁻²)).   
    pub clock_drift_rate: f64,

    /// Data fields depend on the [Constellation] and the RINEX revision
    /// we are dealing with. This structure stores all fields and value
    /// as described by our database, which is an image of the RINEX specs.
    pub orbits: HashMap<String, OrbitItem>,
}

impl Ephemeris {
    /// Grab the satellite clock bias (s), drift (s.s⁻¹) and
    /// drift rate (s.s⁻²)), which is attached to all [Ephemeris].
    pub fn clock_bias_drift_driftrate(&self) -> (f64, f64, f64) {
        (self.clock_bias, self.clock_drift, self.clock_drift_rate)
    }

    /// Returns requested data field from the orbital record.
    /// Returns [EphemerisError::MissingData] on missing data field.
    /// Cast to [f64] is always feasible, whatever the inner interpretation.
    pub(crate) fn get_orbit_field_f64(&self, field: &str) -> Result<f64, EphemerisError> {
        let value = self.orbits.get(field).ok_or(EphemerisError::MissingData)?;

        Ok(value.as_f64())
    }

    /// Add a new orbital parameters, encoded as f64.
    pub(crate) fn set_orbit_f64(&mut self, field: &str, value: f64) {
        self.orbits
            .insert(field.to_string(), OrbitItem::from(value));
    }

    /// Returns number of days elapsed since timescale initialization
    /// and [Ephemeris] reference time.
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn week_number(&self) -> Result<u32, EphemerisError> {
        let week = self.orbits.get("week").ok_or(EphemerisError::MissingData)?;
        Ok(week.as_u32())
    }

    /// Returns number of seconds since sunday midnight
    /// and [Ephemeris] reference time.
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn week_seconds(&self) -> Result<f64, EphemerisError> {
        self.get_orbit_field_f64("toe")
    }

    /// Grab the Total Group Delay (TGD) value, expressed as [Duration].
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn total_group_delay(&self) -> Result<Duration, EphemerisError> {
        let seconds = self.get_orbit_field_f64("tgd")?;
        Ok(Duration::from_seconds(seconds))
    }

    /// Returns true if this [Ephemeris] (radio message snapshot) declares
    /// the attached satellite as suitable for navigation.
    pub fn satellite_is_healthy(&self) -> bool {
        let health = self.orbits.get("health");

        if health.is_none() {
            return false;
        }

        let health = health.unwrap();

        if let Some(flag) = health.as_gps_qzss_l1l2l5_health_flag() {
            flag.healthy()
        } else if let Some(flag) = health.as_gps_qzss_l1c_health_flag() {
            !flag.intersects(GpsQzssl1cHealth::UNHEALTHY)
        } else if let Some(flag) = health.as_glonass_health_flag() {
            // TODO: Status mask .. ?
            if let Some(flag2) = self
                .orbits
                .get("health2")
                .and_then(|item| Some(item.as_glonass_health2_flag().unwrap()))
            {
                !flag.intersects(GlonassHealth::UNHEALTHY)
                    && flag2.intersects(GlonassHealth2::HEALTHY_ALMANAC)
            } else {
                !flag.intersects(GlonassHealth::UNHEALTHY)
            }
        } else if let Some(flag) = health.as_geo_health_flag() {
            // TODO !
            false
        } else if let Some(flag) = health.as_bds_sat_h1_flag() {
            !flag.intersects(BdsSatH1::UNHEALTHY)
        } else if let Some(flag) = health.as_bds_health_flag() {
            flag == BdsHealth::Healthy
        } else {
            false
        }
    }

    /// Returns true if this [Ephemeris] (radio message snapshot) declares
    /// the attached satellite as being tested (not suitable for navigation).
    pub fn satellite_under_test(&self) -> bool {
        let health = self.orbits.get("health");

        if health.is_none() {
            return false;
        }

        let health = health.unwrap();

        // only exists for modern BDS at the moment
        if let Some(flag) = health.as_bds_health_flag() {
            flag == BdsHealth::UnhealthyTesting
        } else {
            false
        }
    }

    /// GEO and Glonass sat [Ephemeris] specific: returns the
    /// reference position and velocity vector, both expressed in kilometers.
    /// It is not possible to navigate (integrate) this position if both
    /// the position and dynamics are not provided.
    #[cfg(feature = "nav")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nav")))]
    pub fn geo_glonass_reference_pos_vel_km(&self) -> Result<Vector6, EphemerisError> {
        let (x_m, y_m, z_m) = (
            self.get_orbit_field_f64("posX")?,
            self.get_orbit_field_f64("posY")?,
            self.get_orbit_field_f64("posZ")?,
        );

        let (velx_m, vely_m, velz_m) = (
            self.get_orbit_field_f64("velX")?,
            self.get_orbit_field_f64("velY")?,
            self.get_orbit_field_f64("velZ")?,
        );

        Ok(Vector6::new(
            x_m * 1e-3,
            y_m * 1e-3,
            z_m * 1e-3,
            velx_m * 1e-3,
            vely_m * 1e-3,
            velz_m * 1e-3,
        ))
    }

    /// GEO and Glonass sat [Ephemeris] specific: returns the reference acceleration vector, in km.s⁻².
    #[cfg(feature = "nav")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nav")))]
    pub fn geo_glonass_reference_accel_km(&self) -> Result<Vector3, EphemerisError> {
        let (x_m, y_m, z_m) = (
            self.get_orbit_field_f64("accelX")?,
            self.get_orbit_field_f64("accelY")?,
            self.get_orbit_field_f64("accelZ")?,
        );

        Ok(Vector3::new(x_m * 1e-3, y_m * 1e-3, z_m * 1e-3))
    }

    /// Returns the semi-major axis expressed in meters at reference time.
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn semi_major_axis_m(&self) -> Result<f64, EphemerisError> {
        let sqrt_a = self.get_orbit_field_f64("sqrta")?;
        Ok(sqrt_a.powf(2.0))
    }

    /// Returns the orbital eccentricity at reference time.
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn eccentricity(&self) -> Result<f64, EphemerisError> {
        self.get_orbit_field_f64("e")
    }

    /// Returns the longitude of the ascending node at reference time, in radians.
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn longitude_ascending_node_rad(&self) -> Result<f64, EphemerisError> {
        self.get_orbit_field_f64("omega0")
    }

    /// Returns the mean anomaly at reference time, in radians.
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn mean_anomaly_rad(&self) -> Result<f64, EphemerisError> {
        self.get_orbit_field_f64("m0")
    }

    /// Returns the inclination at reference time in radians.
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn inclination_rad(&self) -> Result<f64, EphemerisError> {
        self.get_orbit_field_f64("i0")
    }

    /// Returns the argument of perigee at reference time, in radians.
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn argument_of_perigee_rad(&self) -> Result<f64, EphemerisError> {
        self.get_orbit_field_f64("omega")
    }

    /// Returns mean-motion difference at reference time in radians.
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn mean_motion_difference_rad(&self) -> Result<f64, EphemerisError> {
        self.get_orbit_field_f64("deltaN")
    }

    /// Returns the inclination rate-of-change at reference time, in radians.s⁻¹.
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn inclination_rate_of_change_rad_s(&self) -> Result<f64, EphemerisError> {
        self.get_orbit_field_f64("idot")
    }

    /// Returns the right ascension rate-of-change at reference time, in radians.s⁻¹.
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn right_ascension_rate_of_change_rad_s(&self) -> Result<f64, EphemerisError> {
        self.get_orbit_field_f64("omegaDot")
    }

    /// Returns the i-sine and i-cosine harmonic correction, in radians.
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn harmonic_correction_isin_icos(&self) -> Result<(f64, f64), EphemerisError> {
        let cic_rad = self.get_orbit_field_f64("cic")?;
        let cis_rad = self.get_orbit_field_f64("cis")?;
        Ok((cis_rad, cic_rad))
    }

    /// Returns the u-sine and u-cosine harmonic correction, in radians.
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn harmonic_correction_usin_ucos(&self) -> Result<(f64, f64), EphemerisError> {
        let cuc_rad = self.get_orbit_field_f64("cuc")?;
        let cus_rad = self.get_orbit_field_f64("cus")?;
        Ok((cus_rad, cuc_rad))
    }

    /// Returns the r-sine and r-cosine harmonic correction, in meters.
    /// Applies to all but GEO and Glonass sat [Ephemeris] frames.
    pub fn harmonic_correction_rsin_rcos(&self) -> Result<(f64, f64), EphemerisError> {
        let crc_m = self.get_orbit_field_f64("crc")?;
        let crs_m = self.get_orbit_field_f64("crs")?;
        Ok((crs_m, crc_m))
    }

    /// Glonass [Ephemeris] specific: returns the FDMA channel number.
    pub fn glonass_fdma_channel(&self) -> Result<i8, EphemerisError> {
        let value = self
            .orbits
            .get("channel")
            .ok_or(EphemerisError::MissingData)?;
        Ok(value.as_i8())
    }

    /// Return Time of [Ephemeris] (`ToE`) expressed as [Epoch].
    pub fn toe(&self, satellite: SV) -> Result<Epoch, EphemerisError> {
        if satellite.constellation.is_sbas() {
            return Err(EphemerisError::BadOperation);
        }

        if satellite.constellation == Constellation::Glonass {
            return Err(EphemerisError::BadOperation);
        }

        // TODO: in CNAV V4 TOC is said to be TOE... check that
        let (week, seconds) = (self.week_number()?, self.week_seconds()?);

        let nanos = (seconds * 1.0E9).round() as u64;

        match satellite.constellation {
            Constellation::GPS | Constellation::Galileo => {
                Ok(Epoch::from_time_of_week(week, nanos, TimeScale::GPST))
            },
            Constellation::QZSS => Ok(Epoch::from_time_of_week(week, nanos, TimeScale::QZSST)),
            Constellation::BeiDou => Ok(Epoch::from_time_of_week(week, nanos, TimeScale::BDT)),
            Constellation::Glonass => unreachable!("glonass constellation: handled elswhere"),
            constellation => Err(EphemerisError::NotSupported(constellation)),
        }
    }

    /// Returns the derivative correction to the semi-major axis,
    /// in meters.s⁻¹, as provided by V4 messages.
    /// Applies to V4(CNAV) GPS, QZSS, GAL and BDS only.
    pub fn cnav_adot_m_s(&self) -> Result<f64, EphemerisError> {
        self.get_orbit_field_f64("adot")
    }
}

impl Ephemeris {
    /// Creates new [Ephemeris] with desired [OrbitItem]
    pub fn with_orbit(&self, key: &str, orbit: OrbitItem) -> Self {
        let mut s = self.clone();
        s.orbits.insert(key.to_string(), orbit);
        s
    }

    /// Creates new [Ephemeris] with desired week counter
    pub fn with_week(&self, week: u32) -> Self {
        self.with_orbit("week", OrbitItem::from(week))
    }

    /// Calculates clock correction for given satellite at desired [Epoch]
    /// using this [Ephemeris] frame and associated ToC (Time of Clock).
    ///
    /// ## Input
    /// - satellite as [SV]
    /// - time of clock as [Epoch]
    /// - [Epoch]
    /// - number of iteration to be used by solver
    ///
    /// ## Output
    /// - clock correction as [Duration] in [TimeScale]
    pub fn clock_correction(
        &self,
        satellite: SV,
        toc: Epoch,
        epoch: Epoch,
        num_iter: usize,
    ) -> Result<Duration, EphemerisError> {
        let sv_ts = match satellite.constellation.timescale() {
            Some(timescale) => timescale,
            None => {
                return Err(EphemerisError::NotSupported(satellite.constellation));
            },
        };

        let t_sv = epoch.to_time_scale(sv_ts);
        let toc_sv = toc.to_time_scale(sv_ts);

        let (a0, a1, a2) = self.clock_bias_drift_driftrate();
        let mut dt = (t_sv - toc_sv).to_seconds();

        for _ in 0..num_iter {
            dt -= a0 + a1 * dt + a2 * dt.powi(2);
        }

        Ok(Duration::from_seconds(a0 + a1 * dt + a2 * dt.powi(2)))
    }

    /// (elevation, azimuth, range) determination helper,
    /// returned in the form of [AzElRange], for desired [SV] observed at RX coordinates,
    /// expressed in km in fixed body [Frame] centered on Earth.
    #[cfg(feature = "nav")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nav")))]
    pub fn elevation_azimuth_range(
        t: Epoch,
        almanac: &Almanac,
        fixed_body_frame: Frame,
        sv_position_km: (f64, f64, f64),
        rx_position_km: (f64, f64, f64),
    ) -> AlmanacResult<AzElRange> {
        let (rx_x_km, rx_y_km, rx_z_km) = rx_position_km;
        let (tx_x_km, tx_y_km, tx_z_km) = sv_position_km;

        let rx_orbit = Orbit::from_position(rx_x_km, rx_y_km, rx_z_km, t, fixed_body_frame);
        let tx_orbit = Orbit::from_position(tx_x_km, tx_y_km, tx_z_km, t, fixed_body_frame);

        almanac.azimuth_elevation_range_sez(rx_orbit, tx_orbit, None, None)
    }

    /// Returns true if this [Ephemeris] frame is valid for specified epoch.
    ///
    /// ## Input
    /// - satellite: as [SV]
    /// - toc: time of clock as [Epoch]
    /// - epoch: [Epoch]
    ///
    /// ## Output
    /// - true when suitable
    pub fn is_valid(&self, satellite: SV, toc: Epoch, epoch: Epoch) -> bool {
        let max_dtoe = match Self::validity_duration(satellite.constellation) {
            Some(max_dtoe) => max_dtoe,
            None => {
                #[cfg(feature = "log")]
                error!("{}({:x}) - constellation not supported", epoch, satellite);
                return false;
            },
        };

        if satellite.constellation.is_sbas() || satellite.constellation == Constellation::Glonass {
            (epoch - toc).abs() < max_dtoe
        } else {
            match self.toe(satellite) {
                Ok(toe) => (epoch - toe).abs() < max_dtoe,
                #[cfg(feature = "log")]
                Err(e) => {
                    error!("{}({}): {}", epoch, satellite, e);
                    return false;
                },
                #[cfg(not(feature = "log"))]
                Err(_) => {
                    return false;
                },
            }
        }
    }

    /// Returns [Ephemeris] frame validity period for associated [Constellation].
    pub fn validity_duration(constellation: Constellation) -> Option<Duration> {
        match constellation {
            Constellation::GPS | Constellation::QZSS => Some(Duration::from_seconds(7200.0)),
            Constellation::Galileo => Some(Duration::from_seconds(10800.0)),
            Constellation::BeiDou => Some(Duration::from_seconds(21600.0)),
            Constellation::IRNSS => Some(Duration::from_seconds(7200.0)),
            Constellation::Glonass => Some(Duration::from_seconds(1800.0)),
            c => {
                if c.is_sbas() {
                    // for GEO sat we tolerate a 24h timeframe,
                    // this allows one sample per 24h data files.
                    Some(Duration::from_days(1.25))
                } else {
                    None
                }
            },
        }
    }
}
