use thiserror::Error;

mod formatting;
pub mod orbits;
mod parsing;

/// Ephemeris NAV flags definitions & support
pub mod flags;

use orbits::OrbitItem;

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

use crate::prelude::{Constellation, Duration, Epoch, TimeScale, SV};

use anise::errors::AlmanacError;

use std::collections::HashMap;

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
///     if let Some(tgd) = ephemeris.tgd() {
///         // TGD was found & interpreted as duration
///         let tgd = tgd.total_nanoseconds();
///     }
///
///     // SV Health highest interpretation level: as simple boolean
///     if !ephemeris.sv_healthy() {
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

    /// Orbits are revision and constellation dependent,
    /// sorted by key and content, described in navigation::database
    pub orbits: HashMap<String, OrbitItem>,
}

/// Ephemeris processing specific errors.
#[derive(Debug, Error)]
pub enum EphemerisError {
    /// Timescale must be fully supported
    #[error("{0} is not supported")]
    NotSupported(Constellation),

    /// Missing some mandatory data fields,
    /// unable to perform this calculation.
    #[error("missing data")]
    MissingData,

    /// Keplerian solver failed to converge
    /// within reasonnable iterations
    #[error("kepler solved did not converge")]
    Diverged,

    /// Bad/invalid operation. Happens when the
    /// the user attempts incorrect operations
    /// on GEO/Glonass/SBAS vehicles.
    #[error("invalid ephemeris operation")]
    BadOperation,

    /// BDS-IGSO processing not supported yet
    #[allow(non_snake_case)]
    #[error("BDS-IGSO not supported yet")]
    BeidouIgsoNotSupported,

    /// Alamanac error
    #[error("almanac error: {0}")]
    AlmanacError(#[from] AlmanacError),

    /// Failed to select an [Ephemeris] frame
    #[error("{0}({1}): failed to select an ephemeris frame")]
    FrameSelectionError(Epoch, SV),
}

impl Ephemeris {
    /// Returns [SV] onboard clock (bias, drift and drift rate)
    /// in seconds, s/s and s/s^2.
    pub fn sv_clock_bias_drift_change_s(&self) -> (f64, f64, f64) {
        (self.clock_bias, self.clock_drift, self.clock_drift_rate)
    }

    /// Returns any orbital field, from their name and interpreted as [f64],
    /// which is always feasible as RINEX uses [f64] in its description.
    /// Refer to <https://github.com/rtk-rs/rinex/tree/main/db/NAV/orbits.json> to see how the
    /// RINEX fields have been classified.
    pub fn get_orbit_field_f64(&self, field: &str) -> Option<f64> {
        let value = self.orbits.get(field)?;
        Some(value.as_f64())
    }

    /// Manually assign a RINEX field. The field name
    /// should be preserved and follow the database
    /// definitions found here <https://github.com/rtk-rs/rinex/tree/main/db/NAV/orbits.json>
    pub(crate) fn set_orbit_field_f64(&mut self, field: &str, value: f64) {
        self.orbits
            .insert(field.to_string(), OrbitItem::from(value));
    }

    /// Retrieve the indexed week counter, as [u32].
    /// This does not apply to [Constellation::Glonass]
    /// and GEO/SBAS satellites.
    pub(crate) fn get_week(&self) -> Option<u32> {
        self.get_orbit_field_f64("week")
            .and_then(|value| Some(value.round() as u32))
    }

    /// Returns the TGD expressed as a [Duration], if it exists.
    pub fn tgd(&self) -> Option<Duration> {
        let tgd_s = self.get_orbit_field_f64("tgd")?;
        Some(Duration::from_seconds(tgd_s))
    }

    /// Returns true if this [Ephemeris] declares its SV as suitable for navigation.
    pub fn sv_is_healthy(&self) -> bool {
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

    /// Returns true if this [Ephemeris] message declares this satellite in testing mode.
    pub fn sv_in_testing(&self) -> bool {
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

    /// Returns glonass frequency channel, in case this is a Glonass [Ephemeris] message,
    /// with described channel.
    pub fn glonass_freq_channel(&self) -> Option<i8> {
        self.get_orbit_field_f64("channel")
            .and_then(|value| Some(value.round() as i8))
    }

    /// Return Time of [Ephemeris] (ToE) expressed as [Epoch].
    /// This does not apply to GEO / SBAS and Glonass [Ephemeris],
    /// and so should never be invoked in such a pipeline.
    pub fn toe(&self, sv: SV) -> Result<Epoch, EphemerisError> {
        if sv.constellation.is_sbas() {
            return Err(EphemerisError::BadOperation);
        }

        if sv.constellation == Constellation::Glonass {
            return Err(EphemerisError::BadOperation);
        }

        // TODO: in CNAV V4 TOC is said to be TOE... double check that
        let (week, seconds) = (
            self.get_week().ok_or(EphemerisError::MissingData)?,
            self.get_orbit_field_f64("toe")
                .ok_or(EphemerisError::MissingData)?,
        );

        let nanos = (seconds * 1.0E9).round() as u64;

        match sv.constellation {
            Constellation::GPS | Constellation::Galileo => {
                // Constellation::QZSS => {
                Ok(Epoch::from_time_of_week(week, nanos, TimeScale::GPST))
            },
            Constellation::QZSS => Ok(Epoch::from_time_of_week(week, nanos, TimeScale::QZSST)),
            Constellation::BeiDou => Ok(Epoch::from_time_of_week(week, nanos, TimeScale::BDT)),
            Constellation::Glonass => {
                unreachable!("glonass constellation handled elsewhere")
            },
            constellation => Err(EphemerisError::NotSupported(constellation)),
        }
    }

    /// Returns Adot parameter from a CNAV ephemeris
    pub(crate) fn a_dot(&self) -> Option<f64> {
        self.get_orbit_field_f64("a_dot")
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

    /// Calculates Clock correction for [SV] at [Epoch] based on [Self]
    /// and ToC [Epoch] of publication of [Self] from the free running clock.
    ///
    /// ## Inputs
    /// - sv: satellite identity as [SV]
    /// - toc: Time of Clock as [Epoch], expressed in any [Timescale].
    /// - epoch: [Epoch], expressed in any [Timescale].
    /// - max_iter: max iterations in the solver
    ///
    /// ## Output
    /// - clock correction, expressed as [Duration] in corresponding
    /// [Timescale]
    pub fn clock_correction(
        &self,
        sv: SV,
        toc: Epoch,
        epoch: Epoch,
        max_iter: usize,
    ) -> Result<Duration, EphemerisError> {
        let sv_ts = match sv.constellation.timescale() {
            Some(timescale) => timescale,
            None => {
                return Err(EphemerisError::NotSupported(sv.constellation));
            },
        };

        // transpose if need be
        let t_sv = epoch.to_time_scale(sv_ts);
        let toc_sv = toc.to_time_scale(sv_ts);

        let (a0, a1, a2) = self.sv_clock_bias_drift_change_s();

        let mut dt = (t_sv - toc_sv).to_seconds();

        for _ in 0..max_iter {
            dt -= a0 + a1 * dt + a2 * dt.powi(2);
        }

        Ok(Duration::from_seconds(a0 + a1 * dt + a2 * dt.powi(2)))
    }

    /// Returns True if this [Ephemeris] frame is valid for specified epoch.
    ///
    /// ## Inputs
    /// - sv: satellite identity as [SV]
    /// - toc: Time of Clock as [Epoch]
    /// - epoch: [Epoch]
    ///
    /// ## Returns
    /// - true when suitable
    pub fn is_valid(&self, sv: SV, toc: Epoch, epoch: Epoch) -> bool {
        let max_dtoe = match Self::validity_duration(sv.constellation) {
            Some(max_dtoe) => max_dtoe,
            None => {
                #[cfg(feature = "log")]
                error!("{}({}) - constellation not supported", epoch, sv);
                return false;
            },
        };

        if sv.constellation.is_sbas() || sv.constellation == Constellation::Glonass {
            (epoch - toc).abs() < max_dtoe
        } else {
            match self.toe(sv) {
                Ok(toe) => (epoch - toe).abs() < max_dtoe,
                Err(e) => {
                    #[cfg(feature = "log")]
                    error!("{}({}) - toe-error: {}", epoch, sv, e);
                    false
                },
            }
        }
    }

    /// Returns [Ephemeris] validity period for this [Constellation].
    pub fn validity_duration(c: Constellation) -> Option<Duration> {
        match c {
            Constellation::GPS | Constellation::QZSS => Some(Duration::from_seconds(7200.0)),
            Constellation::Galileo => Some(Duration::from_seconds(10800.0)),
            Constellation::BeiDou => Some(Duration::from_seconds(21600.0)),
            Constellation::IRNSS => Some(Duration::from_seconds(7200.0)),
            Constellation::Glonass => Some(Duration::from_seconds(1800.0)),
            c => {
                if c.is_sbas() {
                    // For GEO / SBAS, we tolerate a 24hr validity period.
                    // As it seems that some RINEXv3 published a single state per day.
                    // Obviously, the higher the update rate, the better.
                    Some(Duration::from_days(1.0))
                } else {
                    None
                }
            },
        }
    }
}
