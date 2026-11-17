use thiserror::Error;
use hifitime::prelude::{Unit};

use crate::{
    navigation::{ephemeris::GpsQzssl1cHealth, Ephemeris, OrbitItem},
    prelude::{Constellation, SV},
};

use std::collections::HashMap;

use gnss_protos::{
    GpsQzssFrame1, GpsQzssFrame2, GpsQzssFrame3, 
};


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
    /// satellite. This API does not tolerate incorrect message collection.
    pub fn from_gps_qzss_subframes(
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

        // orbits.insert("i0".to_string(), eph_1.i0_rad);
        orbits.insert("week".to_string(), OrbitItem::U32(eph_1.week.into()));
        orbits.insert(
            "l2p".to_string(),
            OrbitItem::Gpsl2pFlag(eph_1.l2_p_data_flag),
        );
        orbits.insert("tgd".to_string(), OrbitItem::F64(eph_1.tgd));
        orbits.insert("ura".to_string(), OrbitItem::U8(eph_1.ura));

        let l1c_health = Into::<u8>::into(eph_1.health);
        let l1c_health = GpsQzssl1cHealth::from_bits(l1c_health as u32).unwrap_or_default();

        orbits.insert(
            "health".to_string(),
            OrbitItem::GpsQzssl1cHealth(l1c_health),
        );

        Ok(Self {
            clock_bias: eph_1.af0,
            clock_drift: eph_1.af1,
            clock_drift_rate: eph_1.af2,
            orbits,
        })
    }

    /// Dumps this [Ephemeris] frame as [GpsQzssSubframe] sequence.
    /// The sequence describes the 3 subframes of the 3/5 first part of the message rotation
    /// of a given satellite. To correctly encode the 3/5 first part of the message rotation,
    /// you need to wrap the subframes into the correct proto structure. In particular:
    ///
    /// - you control ToW for each message, which should reflect the correct data transmission rate
    /// if you are aiming towards realistic bitstream synthesis
    ///
    /// - our framework gives you access to TLM Message number and basically every single bit
    /// of the binary message. In particular, this allows advanced users and private operators to
    /// encode the non-public part of the message.
    ///
    /// Once the 3/5 subframes have been wrapped to [GpsQzssFrame] you have 3/5 of the message that
    /// is compatible with streaming. If you want to complete the last 2/5 of the message rotation,
    /// you need to create the two almanach subframes, very similarly but describing different
    /// physical information. This library may offer options to facilitate this step as well,
    /// but that does not exist at the moment. if you are limiting to the first 3/5 part of the
    /// message, you can stream zeros to conclude the rotation of this vehicle (no almanach
    /// information).
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
    pub fn to_gps_qzss_messages(
        &self,
        satellite: SV,
    ) -> Result<GpsQzssSubframes, Error> {

        match satellite.constellation {
            Constellation::GPS | Constellation::QZSS => {},
            _ => return Err(Error::InvalidSatellite),
        }

        let week = self.get_orbit_f64("week").unwrap_or_default();
        let week = (week as u16) & 0x3ff;

        let l2p_mask = self.get_orbit_f64("l2p").unwrap_or_default();
        let health = self.get_orbit_f64("health").unwrap_or_default();
        let tgd_nanos = self.get_orbit_f64("tgd").unwrap_or_default();

        let ura_m = self.get_orbit_f64("accuracy").unwrap_or_default();

        let iodc = self.get_orbit_f64("iodc").unwrap_or_default();

        let subframe1 = GpsQzssFrame1::default()
            .with_week(week)
            .with_iodc(iodc as u16)
            .with_ca_or_p_l2_mask(l2p_mask as u8)
            .with_health((health as u8).into())
            .with_clock_offset_seconds(self.clock_bias)
            .with_clock_drift_seconds_s(self.clock_drift)
            .with_user_range_accuracy_m(ura_m)
            .with_clock_drift_rate_seconds_s2(self.clock_drift_rate)
            .with_total_group_delay_nanos(tgd_nanos);

        // TODO .with_l2p_flag

        let toe_s = self.toe(satellite)
            .ok_or(Error::ToECalculations)?
            .duration.to_unit(Unit::Second);

        let i0_rad = self.get_orbit_f64("i0").unwrap_or_default();
        let crs_m = self.get_orbit_f64("crs").unwrap_or_default();
        let cus_rad = self.get_orbit_f64("cus").unwrap_or_default();
        let semi_major_axis_m = self.get_orbit_f64("sqrt_a").unwrap_or_default().powi(2);
        let mean_motion_diff_rad = self.get_orbit_f64("delta_n").unwrap_or_default();

        let iode = self.get_orbit_f64("iode").unwrap_or_default();

        let subframe2 = GpsQzssFrame2::default()
            .with_toe_seconds(toe_s as u32)
            .with_mean_anomaly_radians(i0_rad)
            .with_crs_meters(crs_m)
            .with_iode(iode as u8)
            .with_eccentricity(self.get_orbit_f64("e").unwrap_or_default())
            .with_semi_major_axis_meters(semi_major_axis_m)
            .with_mean_motion_difference_radians(mean_motion_diff_rad)
            .with_cus_radians(cus_rad);

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
