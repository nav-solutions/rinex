use hifitime::prelude::Unit;
use thiserror::Error;

use crate::{
    navigation::{ephemeris::GpsQzssl1l2l5Health, Ephemeris, OrbitItem},
    prelude::{Constellation, Epoch, TimeScale, SV},
};

use std::collections::HashMap;

use gnss_protos::{GpsQzssFrame1, GpsQzssFrame2, GpsQzssFrame3};

fn rad_to_semicircles(rad: f64) -> f64 {
    rad / core::f64::consts::PI
}

fn semicircles_to_rad(semic: f64) -> f64 {
    semic * core::f64::consts::PI
}

/// Returns unwrapped week number using any close-by GPST epoch
/// and wrapped week number counter
fn unwrapped_week_number(t_gpst: Epoch, wn: u16) -> u32 {
    let current_week = t_gpst.to_time_of_week().0;
    let delta = current_week - wn as u32;
    let rollover = (delta as f64 / 1024.0).round() as u32;
    wn as u32 + rollover * 1024
}

#[derive(Error, Debug)]
pub enum Error {
    /// EPH-1 and EPH-2 IODE codes do not match: invalid message burst
    #[error("EPH-1/EPH-2 IODE mismatch")]
    IodeMismatch,

    /// EPH-x IODE and EPH-3 IODC do not match: invalid message burst
    #[error("EPH-3 IODC mismatch")]
    IodcMismatch,

    /// Not a GPS or QZSS satellite
    #[error("Invalid Satellite")]
    InvalidSatellite,

    /// ToE Time of Issue of Ephemeris internal calculations failed
    #[error("Failed to compute ToE")]
    ToECalculations,
}

/// [GpsQzssSubframes] burst that we can encode from any [Ephemeris]
/// produced by any GPS/QZSS satellite. This API proposes the mirror operation,
/// converting any [Ephemeris] message to such a burst.
#[derive(Debug, Copy, Clone, Default)]
pub struct GpsQzssSubframes {
    /// EPH-1 first message (1st/5 in rotation)
    pub eph_1: GpsQzssFrame1,

    /// EPH-2 second message (2nd/5 in rotation)
    pub eph_2: GpsQzssFrame2,

    /// EPH-3 third message (3rd/5 in rotation)
    pub eph_3: GpsQzssFrame3,
}

impl Ephemeris {
    /// Builds an [Ephemeris] frame from [GpsQzssSubframes] sequence.
    /// The sequence must be extracted from the first 3 parts of the 5 message rotation of a given
    /// satellite. This API only deals with Ephemeris frames 1, 2 & 3,
    /// and it does not tolerate incorrect message association.
    ///
    /// ## Input
    ///
    /// - tow: ToW elapsed time within current GPS week (in seconds)
    /// should be extracted from GpsQzssHow Hand Over data Word.
    pub fn from_gps_qzss_subframes(
        satellite: SV,
        toc: Epoch,
        subframes: GpsQzssSubframes,
    ) -> Result<Self, Error> {
        let mut orbits = HashMap::<String, OrbitItem>::new();

        let (eph_1, eph_2, eph_3) = (subframes.eph_1, subframes.eph_2, subframes.eph_3);

        let iodc = eph_1.iodc();
        let (iode_1, iode_2) = (eph_2.iode, eph_3.iode);

        if iode_1 != iode_2 {
            return Err(Error::IodeMismatch);
        }

        if (iodc & 0x0f) as u8 != iode_1 {
            return Err(Error::IodcMismatch);
        }

        let toc_gpst = toc.to_time_scale(TimeScale::GPST);
        let toc_week_seconds = toc_gpst.to_time_of_week().1 / 1_000_000_000;

        let unwrapped_week = unwrapped_week_number(toc_gpst, eph_1.week);

        orbits.insert("week".to_string(), OrbitItem::U32(unwrapped_week));

        if eph_1.l2_p_data_flag {
            orbits.insert("l2Codes".to_string(), OrbitItem::F64(1.0));
        }

        let ura_raw = eph_1.ura as f64;

        orbits.insert("t_tm".to_string(), OrbitItem::F64(toc_week_seconds as f64));

        orbits.insert("tgd".to_string(), OrbitItem::F64(eph_1.tgd));
        orbits.insert("accuracy".to_string(), OrbitItem::F64(ura_raw));
        orbits.insert("iodc".to_string(), OrbitItem::F64(eph_1.iodc() as f64));

        let dn = semicircles_to_rad(eph_2.dn);
        let m0 = semicircles_to_rad(eph_2.m0);

        orbits.insert("e".to_string(), OrbitItem::F64(eph_2.e));
        orbits.insert("m0".to_string(), OrbitItem::F64(m0));
        orbits.insert("crs".to_string(), OrbitItem::F64(eph_2.crs));
        orbits.insert("cuc".to_string(), OrbitItem::F64(eph_2.cuc));
        orbits.insert("cus".to_string(), OrbitItem::F64(eph_2.cus));
        // orbits.insert("aodo".to_string(), OrbitItem::U8(eph_2.aodo));
        orbits.insert("deltaN".to_string(), OrbitItem::F64(dn));
        orbits.insert("sqrta".to_string(), OrbitItem::F64(eph_2.sqrt_a));
        orbits.insert("iode".to_string(), OrbitItem::F64(eph_2.iode as f64));

        let i0_rad = semicircles_to_rad(eph_3.i0);
        let omega_rad = semicircles_to_rad(eph_3.omega);
        let omega0_rad = semicircles_to_rad(eph_3.omega0);
        let omega_dot_rad_s = semicircles_to_rad(eph_3.omega_dot);

        orbits.insert("crc".to_string(), OrbitItem::F64(eph_3.crc));
        orbits.insert("cic".to_string(), OrbitItem::F64(eph_3.cic));
        orbits.insert("cis".to_string(), OrbitItem::F64(eph_3.cis));
        orbits.insert("i0".to_string(), OrbitItem::F64(i0_rad));
        orbits.insert("idot".to_string(), OrbitItem::F64(eph_3.idot));
        orbits.insert("omega".to_string(), OrbitItem::F64(omega_rad));
        orbits.insert("omega0".to_string(), OrbitItem::F64(omega0_rad));
        orbits.insert("omegaDot".to_string(), OrbitItem::F64(omega_dot_rad_s));

        let health = Into::<u8>::into(eph_1.health);
        let health = GpsQzssl1l2l5Health(health as u32);

        orbits.insert("health".to_string(), OrbitItem::GpsQzssl1l2l5Health(health));

        Ok(Self {
            clock_bias: eph_1.af0,
            clock_drift: eph_1.af1,
            clock_drift_rate: eph_1.af2,
            orbits,
        })
    }

    /// Dumps this [Ephemeris] frame as [GpsQzssSubframe] sequence.
    ///
    /// The sequence describes the first 3 frames of the 5 frame message rotation
    /// of a given satellite.  To correctly encode each message, you will have to
    /// wrap each subframe as GpsQzssFrame which can then be streamed to binary.
    /// In particular:
    ///
    /// - you control ToW for each message, which should reflect the correct data transmission rate
    /// if you are aiming towards realistic bitstream synthesis
    ///
    /// - our framework gives you access to TLM Message number and basically every single bit
    /// of the binary message. In particular, this allows advanced users and private operators to
    /// encode the non-public part of the message.
    ///
    /// Once the 3/5 subframes have been wrapped, you can stream 3/5th of the message rotation.
    /// You can complete the rotation with zeros if you don't know how to encode Almanach
    /// information. This library may offer options to faciliate as well, but it is not available
    /// yet.
    ///
    /// ## Input
    ///
    /// - satellite: must be GPS or QZSS, required for internal calculations
    /// prior formatting
    ///
    /// - week: week number at transmission time (as u16)
    ///
    /// - iodc: IODC message identifier. This API verifies correctness of both identifiers.
    /// information. With great power come great responsabilities.
    ///
    /// - iode: IODE message identifier. This API verifies correctness of both identifiers.
    ///
    /// ## Output
    /// - 3/5 of the message rotation, described as [GpsQzssSubframes] without
    /// "header" information that you still have to complete.
    ///
    /// ```
    /// let ephemeris = Ephemeris::default();
    ///
    /// let g01 = SV(Constellation::GPS, 1);
    /// let toc = Epoch::from_gregorian_utc_at_midnight(2000, 1, 1);
    ///
    /// // Message combination must be valid
    /// let iodc = 0x3f;
    /// let iode = 0x3f;
    ///
    /// let message_burst = ephemeris.to_gps_qzss_subframes(toc, g01, iode, iodc)
    ///     .unwrap();
    ///
    /// // bit true reciprocity
    /// let reciprocal = Ephemeris::from_gps_qzss_messages(message_burst)
    ///     .unwrap();
    ///
    /// assert_eq!(reciprocal, ephemeris);
    /// ```
    pub fn to_gps_qzss_subframes(&self, satellite: SV) -> Result<GpsQzssSubframes, Error> {
        match satellite.constellation {
            Constellation::GPS | Constellation::QZSS => {},
            _ => return Err(Error::InvalidSatellite),
        }

        let week = self.get_orbit_f64("week").unwrap_or_default();
        // TODO manque logique unwrap date avec ToC
        // let week = (week as u16) & 0x3ff;
        let week = week as u16;

        let l2p_code = self.get_orbit_f64("l2Codes").unwrap_or_default();
        let l2p_flag = self.get_orbit_f64("l2p").unwrap_or_default() > 0.0;

        let health = self.get_orbit_f64("health").unwrap_or_default();
        let tgd_s = self.get_orbit_f64("tgd").unwrap_or_default();

        let ura_m = self.get_orbit_f64("accuracy").unwrap_or_default();

        let iodc = self.get_orbit_f64("iodc").unwrap_or_default();

        let mut subframe1 = GpsQzssFrame1::default()
            .with_week(week)
            .with_iodc(iodc as u16)
            .with_ca_or_p_l2_mask(l2p_code as u8)
            .with_health((health as u8).into())
            .with_clock_offset_seconds(self.clock_bias)
            .with_clock_drift_seconds_s(self.clock_drift)
            .with_user_range_accuracy_m(ura_m)
            .with_clock_drift_rate_seconds_s2(self.clock_drift_rate)
            .with_total_group_delay_seconds(tgd_s);

        if l2p_flag {
            subframe1 = subframe1.with_l2p_flag();
        }

        let toe_s = self
            .toe(satellite)
            .ok_or(Error::ToECalculations)?
            .duration
            .to_unit(Unit::Second);

        let ecc = self.get_orbit_f64("e").unwrap_or_default();
        let m0_rad = self.get_orbit_f64("m0").unwrap_or_default();
        let crs_m = self.get_orbit_f64("crs").unwrap_or_default();
        let cus_rad = self.get_orbit_f64("cus").unwrap_or_default();
        let cuc_rad = self.get_orbit_f64("cuc").unwrap_or_default();
        let semi_major_axis_m = self.get_orbit_f64("sqrta").unwrap_or_default().powi(2);
        let mean_motion_diff_rad_s = self.get_orbit_f64("deltaN").unwrap_or_default();

        let iode = self.get_orbit_f64("iode").unwrap_or_default();

        // TODO AoDo in RINEX ??
        let aodo = self.get_orbit_f64("aodo").unwrap_or_default();

        let subframe2 = GpsQzssFrame2::default()
            .with_aodo(0) // TODO aodo as u8)
            .with_toe_seconds(toe_s as u32)
            .with_mean_anomaly_radians(m0_rad)
            .with_crs_meters(crs_m)
            .with_iode(iode as u8)
            .with_cuc_radians(cuc_rad)
            .with_cus_radians(cus_rad)
            .with_eccentricity(ecc)
            .with_semi_major_axis_meters(semi_major_axis_m)
            .with_mean_motion_difference_radians(mean_motion_diff_rad_s);

        // TODO subframe2 .with_fit_interval

        let i0_rad = self.get_orbit_f64("i0").unwrap_or_default();
        let idot_rad_s = self.get_orbit_f64("idot").unwrap_or_default();

        let crc_m = self.get_orbit_f64("crc").unwrap_or_default();
        let cic_rad = self.get_orbit_f64("cic").unwrap_or_default();
        let cis_rad = self.get_orbit_f64("cis").unwrap_or_default();

        let omega0_rad = self.get_orbit_f64("omega0").unwrap_or_default();
        let omega_rad = self.get_orbit_f64("omega").unwrap_or_default();
        let omega_dot_rad_s = self.get_orbit_f64("omegaDot").unwrap_or_default();

        let subframe3 = GpsQzssFrame3::default()
            .with_cic_radians(cic_rad)
            .with_cis_radians(cis_rad)
            .with_crc_meters(crc_m)
            .with_inclination_radians(i0_rad)
            .with_iode(iode as u8)
            .with_inclination_rate_radians_s(idot_rad_s)
            .with_longitude_ascending_node_radians(omega0_rad)
            .with_omega_radians(omega_rad)
            .with_omega_dot_rad_s(omega_dot_rad_s);

        Ok(GpsQzssSubframes {
            eph_1: subframe1,
            eph_2: subframe2,
            eph_3: subframe3,
        })
    }
}
