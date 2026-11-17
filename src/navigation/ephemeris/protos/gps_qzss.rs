use hifitime::prelude::{Duration, Unit};

use thiserror::Error;

use crate::{
    navigation::ephemeris::{
        protos::semicircles_to_rad, Ephemeris, GpsQzssl1l2l5Health, OrbitItem,
    },
    prelude::{Constellation, Epoch, TimeScale, SV},
};

use std::collections::HashMap;

use gnss_protos::{
    GpsQzssFrame, GpsQzssFrame1, GpsQzssFrame2, GpsQzssFrame3, GpsQzssFrameId, GpsQzssHow,
    GpsQzssSubframe, GpsQzssTelemetry,
};

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
    /// EPH-1, EPH-2 and EPH-3 are not consecutive in time, that is,
    /// most likely not part of the same message.
    /// Each meessag should be evenly spaced in time with 6s interval.
    #[error("non-consecutive eph-1,2,3 subframes")]
    NonConsecutiveEph123,

    #[error("invalid tx-epoch: > half week away")]
    InvalidEpoch,

    /// We expect correct rotation starting with EPH-1
    #[error("not a valid EPH-1 subframe")]
    InvalidEph1,

    /// We expect correct rotation, EPH-2 following EPH-1
    #[error("not a valid EPH-2 subframe")]
    InvalidEph2,

    /// We expect correct rotation, EPH-3 following EPH-2
    #[error("not a valid EPH-3 subframe")]
    InvalidEph3,

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

    /// RINEX file did not describe t_tm correctly (9.999E9 special value);
    /// It is impossible to format a valid message
    #[error("transmission time undefined in RINEX file")]
    MissingTtm,

    /// RINEX file did not describe toe correctly.
    /// It is impossible to format a valid message
    #[error("ToE undefined in RINEX file")]
    MissingToE,
}

/// [GpsQzssEphemerisMessage] represents the first 3/5 of the message
/// rotation of any given satellite, and can be formed from any
/// RINEX [Ephemeris].
#[derive(Debug, Copy, Clone, Default)]
pub struct GpsQzssEphemerisMessage {
    /// EPH-1: 1/5 in message rotation, ready to encode
    pub eph_1: GpsQzssFrame,

    /// EPH-2: 2/5 in message rotation, ready to encode
    pub eph_2: GpsQzssFrame,

    /// EPH-3: 3/5 in message rotation, ready to encode
    pub eph_3: GpsQzssFrame,
}

/// [GpsQzssEphemerisTelemetries] allows you to compose each
/// [GpsQzssTelemetry] to be attached to each individual subframe
/// composing the ephemeris (first 3/5) of the message rotation.
/// In particular, this is most useful to advanced users that can take
/// advantage of this API to compose the TLM and private bits of the bitstream.
pub struct GpsQzssEphemerisTelemetries {
    /// TLM-1 to be associated to EPH-1
    pub tlm_1: GpsQzssTelemetry,

    /// TLM-2 to be associated to EPH-2
    pub tlm_2: GpsQzssTelemetry,

    /// TLM-3 to be associated to EPH-3
    pub tlm_3: GpsQzssTelemetry,
}

/// [GpsQzssMessage] sequence that consists in 5 [GpsQzssFrame]s
/// streamed in strict order, by any satellite.
/// The first 3/5 parts can be formed from any RINEX [Ephemeris].
/// The last 4/5 and 5/5 (Almanach) frames can be [...]
pub struct GpsQzssMessage {
    /// Ephemeris 3/5 of the message rotation
    pub eph: GpsQzssEphemerisMessage,

    /// ALM-4: 4/5 in message rotation, ready to encode
    pub alm_4: GpsQzssFrame,

    /// ALM-5: 5/5 in message rotation, ready to encode
    pub alm_5: GpsQzssFrame,
}

/// [GpsQzssMessageTelemetries] allows you to compose each
/// [GpsQzssTelemetry] of a complete message rotation.
/// This is most useful to advanced users that can take
/// advantage of this API to compose the TLM and private bits of the bitstream.
pub struct GpsQzssTelemetries {
    /// 3x [GpsQzssTelemetry] to be associated with the first 3/5 subframes
    pub eph_tlm: GpsQzssEphemerisTelemetries,

    /// ALM-4 [GpsQzssTelemetry] associated with subframe 4/5
    pub tlm_4: GpsQzssTelemetry,

    /// ALM-5 [GpsQzssTelemetry] associated with subframe 5/5
    pub tlm_5: GpsQzssTelemetry,
}

impl Ephemeris {
    /// Builds an [Ephemeris] frame first 3/5 part of a GPS/QZSS
    /// bitstream messge rotation. The first 3/5 ephemeris messages
    /// can help form any RINEX [Ephemeris] frames.
    ///
    /// This API guards against invalid GPS/QZSS message associations,
    /// it is illegal to group messages that do not belong to the same
    /// satellite & valid rotation.
    ///
    /// ## Input
    /// - t_rx: usually [Epoch] of message reception.
    /// In practice, any [Epoch] that is less than 512 weeks from
    /// the actual transmission time of the following message will work fine here.
    /// Any [TimeScale] supported, including [TimeScale::UTC].
    ///
    /// - satellite: SV
    ///
    /// ## Returns
    /// - (ToC) Time of Clock and [Ephemeris] frame
    pub fn from_gps_qzss_eph_message(
        t_rx: Epoch,
        satellite: SV,
        msg: GpsQzssEphemerisMessage,
    ) -> Result<(Epoch, Self), Error> {
        let mut orbits = HashMap::<String, OrbitItem>::new();

        let (eph_1, eph_2, eph_3) = (msg.eph_1, msg.eph_2, msg.eph_3);

        if eph_2.how.tow != eph_1.how.tow + 1 {
            return Err(Error::NonConsecutiveEph123);
        }

        if eph_3.how.tow != eph_1.how.tow + 2 {
            return Err(Error::NonConsecutiveEph123);
        }

        let (subf1, subf2, subf3) = (
            eph_1.subframe.as_eph1().ok_or(Error::InvalidEph1)?,
            eph_2.subframe.as_eph2().ok_or(Error::InvalidEph2)?,
            eph_3.subframe.as_eph3().ok_or(Error::InvalidEph3)?,
        );

        let iodc = subf1.iodc();

        let (iode_1, iode_2) = (subf2.iode, subf3.iode);

        if iode_1 != iode_2 {
            return Err(Error::IodeMismatch);
        }

        if (iodc & 0x0f) as u8 != iode_1 {
            return Err(Error::IodcMismatch);
        }

        let t_rx_gpst = t_rx.to_time_scale(TimeScale::GPST);

        let unwrapped_week = unwrapped_week_number(t_rx_gpst, subf1.week);

        let t_sf1 = (eph_1.how.tow * 6 - 6) as u64;
        let t_tx = Epoch::from_time_of_week(unwrapped_week, t_sf1 * 1_000_000_000, TimeScale::GPST);

        let (toc_s, toe_s) = (subf1.toc * 16, subf2.toe * 16);
        let (toc_nanos, toe_nanos) = (toc_s as u64 * 1_000_000_000, toe_s as u64 * 1_000_000_000);

        let (toc_1, toc_2, toc_3) = (
            Epoch::from_time_of_week(unwrapped_week - 1, toc_nanos, TimeScale::GPST),
            Epoch::from_time_of_week(unwrapped_week, toc_nanos, TimeScale::GPST),
            Epoch::from_time_of_week(unwrapped_week + 1, toc_nanos, TimeScale::GPST),
        );

        let (toe_1, toe_2, toe_3) = (
            Epoch::from_time_of_week(unwrapped_week - 1, toe_nanos, TimeScale::GPST),
            Epoch::from_time_of_week(unwrapped_week, toe_nanos, TimeScale::GPST),
            Epoch::from_time_of_week(unwrapped_week + 1, toe_nanos, TimeScale::GPST),
        );

        let (dtoc_1, dtoc_2, dtoc_3) = (
            (t_tx - toc_1).abs(),
            (t_tx - toc_2).abs(),
            (t_tx - toc_3).abs(),
        );

        let (dtoe_1, dtoe_2, dtoe_3) = (
            (t_tx - toe_1).abs(),
            (t_tx - toe_2).abs(),
            (t_tx - toe_3).abs(),
        );

        let (mut toc, mut toe) = (toc_1, toe_1);

        if dtoc_2 < dtoc_3 && dtoc_2 < dtoc_1 {
            toc = toc_2;
        } else if dtoc_3 < dtoc_1 && dtoc_3 < dtoc_2 {
            toc = toc_3;
        }

        if dtoe_2 < dtoe_3 && dtoe_2 < dtoe_1 {
            toe = toe_2;
        } else if dtoe_3 < dtoe_1 && dtoe_3 < dtoe_2 {
            toe = toe_3;
        }

        orbits.insert("week".to_string(), OrbitItem::U32(unwrapped_week));

        let t_tm = t_tx.duration.to_unit(Unit::Second) - toe.duration.to_unit(Unit::Second);

        orbits.insert("t_tm".to_string(), OrbitItem::F64(t_tm));

        if subf1.l2_p_data_flag {
            orbits.insert("l2Codes".to_string(), OrbitItem::F64(1.0));
        }

        let ura_raw = subf1.ura as f64;

        orbits.insert("tgd".to_string(), OrbitItem::F64(subf1.tgd));
        orbits.insert("accuracy".to_string(), OrbitItem::F64(ura_raw));
        orbits.insert("iodc".to_string(), OrbitItem::F64(subf1.iodc() as f64));

        let dn = semicircles_to_rad(subf2.dn);
        let m0 = semicircles_to_rad(subf2.m0);

        orbits.insert("e".to_string(), OrbitItem::F64(subf2.e));
        orbits.insert("m0".to_string(), OrbitItem::F64(m0));
        orbits.insert("crs".to_string(), OrbitItem::F64(subf2.crs));
        orbits.insert("cuc".to_string(), OrbitItem::F64(subf2.cuc));
        orbits.insert("cus".to_string(), OrbitItem::F64(subf2.cus));

        // TODO AoDo in RINEX?
        // orbits.insert("aodo".to_string(), OrbitItem::U8(subf2.aodo));

        orbits.insert("deltaN".to_string(), OrbitItem::F64(dn));
        orbits.insert("sqrta".to_string(), OrbitItem::F64(subf2.sqrt_a));
        orbits.insert("iode".to_string(), OrbitItem::F64(subf2.iode as f64));

        let i0_rad = semicircles_to_rad(subf3.i0);
        let omega_rad = semicircles_to_rad(subf3.omega);
        let omega0_rad = semicircles_to_rad(subf3.omega0);
        let omega_dot_rad_s = semicircles_to_rad(subf3.omega_dot);

        orbits.insert("crc".to_string(), OrbitItem::F64(subf3.crc));
        orbits.insert("cic".to_string(), OrbitItem::F64(subf3.cic));
        orbits.insert("cis".to_string(), OrbitItem::F64(subf3.cis));
        orbits.insert("i0".to_string(), OrbitItem::F64(i0_rad));
        orbits.insert("idot".to_string(), OrbitItem::F64(subf3.idot));
        orbits.insert("omega".to_string(), OrbitItem::F64(omega_rad));
        orbits.insert("omega0".to_string(), OrbitItem::F64(omega0_rad));
        orbits.insert("omegaDot".to_string(), OrbitItem::F64(omega_dot_rad_s));

        let health = Into::<u8>::into(subf1.health);
        let health = GpsQzssl1l2l5Health(health as u32);

        orbits.insert("health".to_string(), OrbitItem::GpsQzssl1l2l5Health(health));

        Ok((
            toc,
            Self {
                clock_bias: subf1.af0,
                clock_drift: subf1.af1,
                clock_drift_rate: subf1.af2,
                orbits,
            },
        ))
    }

    /// Dumps this [Ephemeris] frame as [GpsQzssEphemerisMessage] sequence.
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
    /// - tx_epoch: [Epoch], usually in future, where the TX of this message burst is planned.
    /// This [Epoch] must lie within +/- half a week from the current frame, otherwise
    /// this is an invalid setup that will lead to an error.
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
    pub fn to_gps_qzss_eph_message(
        &self,
        satellite: SV,
        toc: Epoch,
        alert: bool,
        anti_spoofing: bool,
        telemetries: GpsQzssEphemerisTelemetries,
    ) -> Result<GpsQzssEphemerisMessage, Error> {
        match satellite.constellation {
            Constellation::GPS | Constellation::QZSS => {},
            _ => return Err(Error::InvalidSatellite),
        }

        let toe_gpst = self.toe(satellite).ok_or(Error::ToECalculations)?;

        // reconstructing transmission time
        let toe_week = toe_gpst.to_time_of_week().0;

        // TODO: check t_tm = 0.99999E9 undefined!
        let t_tm = self.get_orbit_f64("t_tm").ok_or(Error::MissingTtm)?;

        let mut t_tx = Epoch::from_time_of_week(toe_week, 0, TimeScale::GPST);
        t_tx += t_tm * Unit::Second;

        let (week, tow) = t_tx.to_time_of_week();
        let week = week % 1024;

        let tow = Duration::from_seconds((tow / 1_000_000_000) as f64).floor(30.0 * Unit::Second);
        let tow_s = tow.to_unit(Unit::Second).round() as u32;

        // SF1.toc is toc in seconds of week /16
        let toc_s = toc.to_time_of_week().1 / 1_000_000_000 / 16;

        // SF2.toe is toe /16
        let toe_s = self.get_orbit_f64("toe").ok_or(Error::MissingToE)? as u32;
        let toe_s = toe_s / 16;

        let l2p_code = self.get_orbit_f64("l2Codes").unwrap_or_default();
        let l2p_flag = self.get_orbit_f64("l2p").unwrap_or_default() > 0.0;

        let health = self.get_orbit_f64("health").unwrap_or_default();
        let tgd_s = self.get_orbit_f64("tgd").unwrap_or_default();

        let ura_m = self.get_orbit_f64("accuracy").unwrap_or_default();

        let iodc = self.get_orbit_f64("iodc").unwrap_or_default();

        let mut subframe1 = GpsQzssFrame1::default()
            .with_week(week as u16)
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

        Ok(GpsQzssEphemerisMessage {
            eph_1: GpsQzssFrame {
                how: {
                    GpsQzssHow {
                        alert,
                        anti_spoofing,
                        tow: (tow_s + 1 * 6) / 6,
                        frame_id: GpsQzssFrameId::Ephemeris1,
                    }
                },
                telemetry: telemetries.tlm_1,
                subframe: GpsQzssSubframe::Ephemeris1(subframe1),
            },
            eph_2: GpsQzssFrame {
                how: {
                    GpsQzssHow {
                        alert,
                        anti_spoofing,
                        tow: (tow_s + 2 * 6) / 6,
                        frame_id: GpsQzssFrameId::Ephemeris2,
                    }
                },
                telemetry: telemetries.tlm_2,
                subframe: GpsQzssSubframe::Ephemeris2(subframe2),
            },
            eph_3: GpsQzssFrame {
                how: {
                    GpsQzssHow {
                        alert,
                        anti_spoofing,
                        tow: (tow_s + 3 * 6) / 6,
                        frame_id: GpsQzssFrameId::Ephemeris3,
                    }
                },
                telemetry: telemetries.tlm_3,
                subframe: GpsQzssSubframe::Ephemeris3(subframe3),
            },
        })
    }
}
