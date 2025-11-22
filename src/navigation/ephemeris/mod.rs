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

#[cfg(feature = "nav")]
use crate::prelude::nav::Almanac;

#[cfg(feature = "ublox")]
mod ublox;

#[cfg(feature = "nav")]
use anise::{
    astro::AzElRange,
    errors::AlmanacResult,
    prelude::{Frame, Orbit},
};

use std::collections::HashMap;

use crate::prelude::{Constellation, Duration, Epoch, TimeScale, SV};

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

    /// Returns abstract orbital data as [f64], for this particular
    /// data field (if it exists in the RINEX specs and was provided by this message).
    pub(crate) fn get_orbit_f64(&self, field: &str) -> Option<f64> {
        let value = self.orbits.get(field)?;
        Some(value.as_f64())
    }

    /// Add a new orbital parameters, encoded as f64.
    pub(crate) fn set_orbit_f64(&mut self, field: &str, value: f64) {
        self.orbits
            .insert(field.to_string(), OrbitItem::from(value));
    }

    /// Grab the week number, which is described by all but
    /// Glonass and SBAS [Ephemeris].
    pub fn week_number(&self) -> Option<u32> {
        self.orbits
            .get("week")
            .and_then(|value| Some(value.as_u32()))
    }

    /// Grab the Total Group Delay (TGD) value, expressed as [Duration], which is expressed
    /// for all but Glonass and SBAS [Ephemeris].
    pub fn total_group_delay(&self) -> Option<Duration> {
        let tgd_s = self.get_orbit_f64("tgd")?;
        Some(Duration::from_seconds(tgd_s))
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

    /// Returns FDMA channel index (Glonass [Ephemeris] specific).
    pub fn glonass_freq_channel(&self) -> Option<i8> {
        if let Some(value) = self.orbits.get("channel") {
            Some(value.as_i8())
        } else {
            None
        }
    }

    /// Return Time of [Ephemeris] (`ToE`) expressed as [Epoch].
    pub fn toe(&self, satellite: SV) -> Option<Epoch> {
        // TODO: in CNAV V4 TOC is said to be TOE... ...
        let (week, seconds) = (self.week_number()?, self.get_orbit_f64("toe")?);
        let nanos = (seconds * 1.0E9).round() as u64;

        match satellite.constellation {
            Constellation::GPS | Constellation::QZSS | Constellation::Galileo => {
                Some(Epoch::from_time_of_week(week, nanos, TimeScale::GPST))
            },
            Constellation::BeiDou => Some(Epoch::from_time_of_week(week, nanos, TimeScale::BDT)),
            _ => {
                #[cfg(feature = "log")]
                error!("{} is not supported", satellite.constellation);
                None
            },
        }
    }

    /// Returns the derivative correction to the semi-major axis,
    /// in meters.s⁻¹, as provided by V4 messages.
    pub fn cnav_adot_m_s(&self) -> Option<f64> {
        self.get_orbit_f64("adot")
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
    pub fn clock_correction(
        &self,
        toc: Epoch,
        t: Epoch,
        sv: SV,
        max_iter: usize,
    ) -> Option<Duration> {
        let sv_ts = sv.constellation.timescale()?;

        let t_sv = t.to_time_scale(sv_ts);
        let toc_sv = toc.to_time_scale(sv_ts);

        if t_sv < toc_sv {
            #[cfg(feature = "log")]
            error!("t < t_oc: bad op!");
            None
        } else {
            let (a0, a1, a2) = (self.clock_bias, self.clock_drift, self.clock_drift_rate);
            let mut dt = (t_sv - toc_sv).to_seconds();
            for _ in 0..max_iter {
                dt -= a0 + a1 * dt + a2 * dt.powi(2);
            }
            Some(Duration::from_seconds(a0 + a1 * dt + a2 * dt.powi(2)))
        }
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

    /// Returns True if this [Ephemeris] frame is valid for specified epoch.
    /// NB: this only applies to MEO Ephemerides, not GEO Ephemerides,
    /// which should always be considered "valid".
    /// ## Input
    /// - sv: [SV] identity
    /// - epoch: test [Epoch]
    pub fn is_valid(&self, sv: SV, t: Epoch) -> bool {
        if let Some(toe) = self.toe(sv) {
            if let Some(max_dtoe) = Self::validity_duration(sv.constellation) {
                (t - toe).abs() < max_dtoe
            } else {
                #[cfg(feature = "log")]
                error!("{} - validity period", sv.constellation);
                false
            }
        } else {
            #[cfg(feature = "log")]
            error!("{} - ToE calculation", sv.constellation);
            false
        }
    }

    /// Ephemeris validity period for this [Constellation]
    pub fn validity_duration(c: Constellation) -> Option<Duration> {
        match c {
            Constellation::GPS | Constellation::QZSS => Some(Duration::from_seconds(7200.0)),
            Constellation::Galileo => Some(Duration::from_seconds(10800.0)),
            Constellation::BeiDou => Some(Duration::from_seconds(21600.0)),
            Constellation::IRNSS => Some(Duration::from_seconds(7200.0)),
            Constellation::Glonass => Some(Duration::from_seconds(1800.0)),
            c => {
                if c.is_sbas() {
                    // Tolerate one publication per day.
                    // Typical RINEX apps will load one set per 24 hr.
                    // GEO Orbits are very special, with a single entry per day.
                    // Therefore, in typical RINEX apps, we will have one entry for every day.
                    // GEO Ephemerides cannot be handled like other Ephemerides anyway, they require
                    // a complete different logic and calculations
                    Some(Duration::from_days(1.0))
                } else {
                    None
                }
            },
        }
    }
}
