use hifitime::{prelude::Unit, NANOSECONDS_PER_SECOND};

use thiserror::Error;

use crate::{
    navigation::{
        ephemeris::{
            protos::{rad_to_semicircles, semicircles_to_rad},
            Ephemeris, GpsQzssl1l2l5Health, OrbitItem,
        },
        NavFrame, NavFrameType, NavKey, NavMessageType, Record as NavRecord,
    },
    prelude::{Comments, Constellation, Epoch, Header, Record, Rinex, TimeScale, SV},
    ProductionAttributes,
};

use std::collections::HashMap;

use gnss_protos::{
    constants::SECONDS_PER_WEEK,
    gps::{
        GpsQzssFrame, GpsQzssFrame1, GpsQzssFrame2, GpsQzssFrame3, GpsQzssFrameId, GpsQzssHow,
        GpsQzssMessage, GpsQzssSubframe, GpsQzssTelemetry, GPS_FRAME_DURATION_S,
        GPS_SUBFRAME_DURATION_S, GPS_TOC_TOE_LSB_S, GPS_WEEK_ROLLOVER,
    },
};

#[cfg(feature = "log")]
use log::error;

/// Half of the GPS Week Rollover, for internal calculations.
const GPS_HALF_WEEK_ROLLOVER: i64 = GPS_WEEK_ROLLOVER as i64 / 2;

/// RINEX describes an unknown transmission time with 0.9999E9.
/// Any |t_tm| greater than this threshold cannot be a valid transmission time.
const T_TM_UNKNOWN_THRESHOLD_S: f64 = 1.0E8;

/// Nominal URA values (in meters) for each URA index (IS-GPS-200 20.3.3.3.1.3).
/// Index 15 means "no accuracy prediction available": we use the natural
/// extension of the ICD formula (2^(N-2)), which lies beyond the 6144m bound.
const NOMINAL_URA_M: [f64; 16] = [
    2.0, 2.8, 4.0, 5.7, 8.0, 11.3, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0, 1024.0, 2048.0, 4096.0,
    8192.0,
];

/// Builds the GPST [Epoch] located at `seconds` within `week`.
fn gpst_epoch(week: u32, seconds: u64) -> Epoch {
    Epoch::from_time_of_week(week, seconds * NANOSECONDS_PER_SECOND, TimeScale::GPST)
}

fn unwrapped_week_number(t_gpst: Epoch, wn: u16) -> u32 {
    let reference_week = t_gpst.to_time_of_week().0 as i64;
    let wn = (wn as i64) % GPS_WEEK_ROLLOVER as i64;

    // candidate within the same 1024 week cycle as the reference
    let mut week = reference_week - reference_week.rem_euclid(GPS_WEEK_ROLLOVER as i64) + wn;

    // pick closest candidate
    let delta = week - reference_week;

    if delta > GPS_HALF_WEEK_ROLLOVER {
        week -= GPS_WEEK_ROLLOVER as i64;
    } else if delta < -GPS_HALF_WEEK_ROLLOVER {
        week += GPS_WEEK_ROLLOVER as i64;
    }

    // weeks prior GPST origin do not exist
    if week < 0 {
        week += GPS_WEEK_ROLLOVER as i64;
    }

    week as u32
}

/// ToC and ToE are only broadcast as seconds within the week.
/// Returns the GPST [Epoch] closest to `t_tx` (transmission time) that
/// lies at `seconds` within its week. Ephemeris reference epochs are
/// always a few hours away from their transmission time, this resolves
/// the week ambiguity.
fn closest_epoch(seconds: u32, t_tx: Epoch) -> Epoch {
    let week = t_tx.to_time_of_week().0;

    [week.saturating_sub(1), week, week + 1]
        .iter()
        .map(|week| gpst_epoch(*week, seconds as u64))
        .min_by_key(|epoch| (*epoch - t_tx).total_nanoseconds().abs())
        .unwrap() // infaillible: 3 candidates
}

/// Returns true if both [Epoch]s are within half a week of each other,
/// which is required to recover the week number of ToC and ToE when decoding.
fn within_half_week(lhs: Epoch, rhs: Epoch) -> bool {
    (lhs - rhs).abs() <= (SECONDS_PER_WEEK / 2) as f64 * Unit::Second
}

/// Converts a RINEX duration (in seconds) to an integer number of seconds,
/// only if it is an exact multiple of `lsb_s` within the week.
fn quantized_seconds_of_week(seconds: f64, lsb_s: u32) -> Option<u32> {
    if seconds.fract() != 0.0 || seconds < 0.0 || seconds >= SECONDS_PER_WEEK as f64 {
        return None;
    }

    let seconds = seconds as u32;

    if seconds % lsb_s != 0 {
        return None;
    }

    Some(seconds)
}

/// Returns the fit interval (in hours) as written in RINEX,
/// from the SF2 fit interval flag (IS-GPS-200 20.3.4.4, IS-QZSS-PNT).
fn fit_interval_hours(constellation: Constellation, flag: bool, iodc: u16) -> f64 {
    match (constellation, flag) {
        (Constellation::QZSS, false) => 2.0,
        // IS-QZSS only states "greater than 2 hours"
        (Constellation::QZSS, true) => 4.0,
        (_, false) => 4.0,
        (_, true) => match iodc {
            240..=247 => 8.0,
            248..=255 | 496 => 14.0,
            497..=503 | 1021..=1023 => 26.0,
            _ => 6.0,
        },
    }
}

/// Returns the SF2 fit interval flag, from RINEX fit interval (in hours).
fn fit_interval_flag(constellation: Constellation, fit_interval_h: f64) -> bool {
    match constellation {
        Constellation::QZSS => fit_interval_h > 2.0,
        _ => fit_interval_h > 4.0,
    }
}

/// [Error]s during the GPS/QZSS bitstream composition (serialization)
/// or extraction (deserialization).
#[derive(Error, Debug)]
pub enum Error {
    /// EPH-1, EPH-2 and EPH-3 are not consecutive in time, that is,
    /// most likely not part of the same message.
    /// Each meessag should be evenly spaced in time with 6s interval.
    #[error("non-consecutive eph-1,2,3 subframes")]
    NonConsecutiveEph123,

    /// EPH-1 does not start a GPS frame (frames start every 30s since
    /// the beginning of the week).
    #[error("eph-1 is not aligned to a frame boundary")]
    MisalignedFrame,

    /// ToC or ToE is more than half a week away from the transmission time:
    /// the week number could not be recovered when decoding.
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

    /// EPH-2 and EPH-3 IODE codes do not match: invalid message burst
    #[error("EPH-2/EPH-3 IODE mismatch")]
    IodeMismatch,

    /// IODE does not match the 8 LSB of the IODC: invalid message burst
    #[error("IODC/IODE mismatch")]
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

    /// ToC is not an integer number of seconds, multiple of 16s.
    #[error("ToC cannot be encoded (must be a multiple of 16s)")]
    InvalidToC,

    /// ToE is not an integer number of seconds, multiple of 16s.
    #[error("ToE cannot be encoded (must be a multiple of 16s)")]
    InvalidToE,
}

/// This structure is used when serilizing or deserializing part of a
/// GPS/QZSS bitstream. More precisely, the first 3/5 of the 30s
/// message rotation of any GPS or QZSS satellite.
#[derive(Debug, Copy, Clone, Default)]
pub struct GpsQzssEphemerisItem {
    /// Satellite from which the decoder has decoded the attached
    /// [GpsQzssFrame]. Only GPS or QZSS satellites will be processed,
    /// others will be silently dropped and would apply to different
    /// part of this API.
    pub satellite: SV,

    /// Instant, _any_ timescale may apply here in particular UTC
    /// and not necessarily GPST or QZSST. This instant is usually
    /// the time of receiption when decoding, or the encoding time
    /// when streaming out. Any instant which is +/- 512
    /// week away from the instant of physical transmission will work correctly.
    pub epoch: Epoch,

    /// First 3/5 items of the 30s rotation of associated GPS or QZSS satellite.
    /// We only process the Ephemeris frames. Message telemetry is dropped
    /// and is not encoded in the RINEX file.
    pub eph123: GpsQzssEphemerisMessage,
}

/// [GpsQzssEphCompositionItem] is used when serializing RINEX [Ephemeris] into
/// the first 3/5 of the 30s GPS/QZSS message rotation.
///
/// By accessing each [GpsQzssTelemetry] here, you can compose 100% exact
/// bit stream, which is particularly interesting to advaned users who can
/// compose the reserved and military bits.
#[derive(Debug, Copy, Clone, Default)]
pub struct GpsQzssEphCompositionOpts {
    /// Satellite signal source to be emulated or synthesized.
    /// Only GPS or QZSS satellites will be accepted here.
    pub satellite: SV,

    /// HOW alert flag that will apply to EPH-1, 2 and 3 similarly
    /// for this 30s cycle.
    pub alert: bool,

    /// HOW A/S flag, that will apply to EPH-1, 2 and 3 similarly
    /// for this 30s cycle.
    pub anti_spoofing: bool,

    /// Reference epoch, usually the "Time of Clock" (ToC) attached to each
    /// entry in the RINEX record. Any instant which lies with +/- 512 week
    /// from the transmission instant you intend to use will work correctly here.
    /// Any timescale may apply here because we internally convert to GPST properly.
    pub epoch: Epoch,

    /// [GpsQzssTelemetry] to be attached to the EPH-1 frame (1/5).
    /// Accurate control over each telemetry is most interesting for advanced
    /// users who can take advantage of that to encode the private bits of the bitstream.
    pub tlm_1: GpsQzssTelemetry,

    /// [GpsQzssTelemetry] to be attached to the EPH-2 frame (2/5).
    /// Accurate control over each telemetry is most interesting for advanced
    /// users who can take advantage of that to encode the private bits of the bitstream.
    pub tlm_2: GpsQzssTelemetry,

    /// [GpsQzssTelemetry] to be attached to the EPH-3 frame (3/5).
    /// Accurate control over each telemetry is most interesting for advanced
    /// users who can take advantage of that to encode the private bits of the bitstream.
    pub tlm_3: GpsQzssTelemetry,
}

/// [GpsQzssEphemerisMessage] represents the first 3/5 of the 30s message
/// rotation of any given satellite. The first 3/5 of the 30s rotation can always
/// be created from or converted into a valid RINEX [Ephemeris], this API works both ways.
///
/// If you intend to create the full 30s cycle, you will need to insert
/// the ALM-4 and ALM-5 frames after EPH-3. A simple application that does
/// not intend to create a fully realistic signal could padd the cycle with zeros.
#[derive(Debug, Copy, Clone, Default)]
pub struct GpsQzssEphemerisMessage {
    /// EPH-1 frame: 1/5 in 30s rotation, as decoded or ready to encode.
    pub eph_1: GpsQzssFrame,

    /// EPH-2 frame: 2/5 in 30s rotation, as decoded or ready to encode.
    pub eph_2: GpsQzssFrame,

    /// EPH-3 frame: 3/5 in 30s rotation, as decoded or ready to encode.
    pub eph_3: GpsQzssFrame,
}

impl Ephemeris {
    /// Builds an [Ephemeris] frame from the first 3/5 part of a GPS/QZSS
    /// bitstream message rotation.
    ///
    /// This API guards against invalid GPS/QZSS message associations,
    /// it is illegal to group messages that do not belong to the same
    /// satellite & valid rotation (consecutive HOW, matching IODC/IODE).
    ///
    /// ## Time management
    /// - the transmission time is the beginning of EPH-1 (HOW(EPH-1) - 6s),
    /// which is the beginning of the frame. It is written as `t_tm`,
    /// in seconds relative to the week of ToE (possibly negative).
    /// - ToC and ToE are broadcast as seconds within the week:
    /// their week is the one placing them closest to the transmission time.
    /// - the RINEX `week` is the week of ToE (not the broadcast week number).
    ///
    /// ## Input
    /// - t_rx: usually [Epoch] of message reception.
    /// In practice, any [Epoch] that is less than 512 weeks from
    /// the actual transmission time of the following message will work fine here.
    /// Any [TimeScale] supported, including [TimeScale::UTC].
    ///
    /// - satellite: GPS or QZSS [SV]
    ///
    /// ## Returns
    /// - (ToC) Time of Clock, expressed in [TimeScale::GPST], and [Ephemeris] frame
    pub fn from_gps_qzss_eph_message(item: GpsQzssEphemerisItem) -> Result<(Epoch, Self), Error> {
        match item.satellite.constellation {
            Constellation::GPS | Constellation::QZSS => {},
            _ => return Err(Error::InvalidSatellite),
        }

        let mut orbits = HashMap::<String, OrbitItem>::new();

        let (eph_1, eph_2, eph_3) = (item.eph123.eph_1, item.eph123.eph_2, item.eph123.eph_3);

        // HOW.tow is expressed in seconds:
        // start of the following subframe.
        if eph_2.how.tow != eph_1.how.tow + GPS_SUBFRAME_DURATION_S {
            return Err(Error::NonConsecutiveEph123);
        }

        if eph_3.how.tow != eph_1.how.tow + 2 * GPS_SUBFRAME_DURATION_S {
            return Err(Error::NonConsecutiveEph123);
        }

        let (subf1, subf2, subf3) = (
            eph_1.subframe.as_eph1().ok_or(Error::InvalidEph1)?,
            eph_2.subframe.as_eph2().ok_or(Error::InvalidEph2)?,
            eph_3.subframe.as_eph3().ok_or(Error::InvalidEph3)?,
        );

        let iodc = subf1.iodc();

        if subf2.iode != subf3.iode {
            return Err(Error::IodeMismatch);
        }

        if (iodc & 0xff) as u8 != subf2.iode {
            return Err(Error::IodcMismatch);
        }

        // Transmission time: EPH-1 starts 6s before the HOW(EPH-1) instant,
        // and EPH-1 always starts a 30s frame.
        if eph_1.how.tow < GPS_SUBFRAME_DURATION_S {
            return Err(Error::MisalignedFrame);
        }

        let t_sf1_s = (eph_1.how.tow - GPS_SUBFRAME_DURATION_S) as u64;

        if t_sf1_s % GPS_FRAME_DURATION_S as u64 != 0 {
            return Err(Error::MisalignedFrame);
        }

        let t_rx_gpst = item.epoch.to_time_scale(TimeScale::GPST);
        let tx_week = unwrapped_week_number(t_rx_gpst, subf1.week);
        let t_tx = gpst_epoch(tx_week, t_sf1_s);

        // ToC & ToE (seconds of week) attached to the closest week
        let toc = closest_epoch(subf1.toc, t_tx);
        let toe = closest_epoch(subf2.toe, t_tx);

        let toe_week = toe.to_time_of_week().0;
        let t_tm_s = (t_tx - gpst_epoch(toe_week, 0)).to_unit(Unit::Second);

        orbits.insert("week".to_string(), OrbitItem::U32(toe_week));
        orbits.insert("toe".to_string(), OrbitItem::F64(subf2.toe as f64));
        orbits.insert("t_tm".to_string(), OrbitItem::F64(t_tm_s));

        // SF1
        orbits.insert(
            "l2Codes".to_string(),
            OrbitItem::F64(subf1.ca_or_p_l2 as f64),
        );
        orbits.insert(
            "l2p".to_string(),
            OrbitItem::Gpsl2pFlag(subf1.l2_p_data_flag),
        );

        orbits.insert(
            "accuracy".to_string(),
            OrbitItem::F64(NOMINAL_URA_M[(subf1.ura & 0x0f) as usize]),
        );

        let health = Into::<u8>::into(subf1.health);

        orbits.insert(
            "health".to_string(),
            OrbitItem::GpsQzssl1l2l5Health(GpsQzssl1l2l5Health(health as u32)),
        );

        orbits.insert("tgd".to_string(), OrbitItem::F64(subf1.tgd));
        orbits.insert("iodc".to_string(), OrbitItem::F64(iodc as f64));

        // SF2
        orbits.insert("iode".to_string(), OrbitItem::F64(subf2.iode as f64));
        orbits.insert("crs".to_string(), OrbitItem::F64(subf2.crs));
        orbits.insert(
            "deltaN".to_string(),
            OrbitItem::F64(semicircles_to_rad(subf2.dn)),
        );
        orbits.insert(
            "m0".to_string(),
            OrbitItem::F64(semicircles_to_rad(subf2.m0)),
        );
        orbits.insert("cuc".to_string(), OrbitItem::F64(subf2.cuc));
        orbits.insert("e".to_string(), OrbitItem::F64(subf2.e));
        orbits.insert("cus".to_string(), OrbitItem::F64(subf2.cus));
        orbits.insert("sqrta".to_string(), OrbitItem::F64(subf2.sqrt_a));

        orbits.insert(
            "fitInt".to_string(),
            OrbitItem::F64(fit_interval_hours(
                item.satellite.constellation,
                subf2.fit_int_flag,
                iodc,
            )),
        );

        // TODO AoDo is not described in RINEX
        // orbits.insert("aodo".to_string(), OrbitItem::U8(subf2.aodo));

        // SF3
        orbits.insert("cic".to_string(), OrbitItem::F64(subf3.cic));
        orbits.insert(
            "omega0".to_string(),
            OrbitItem::F64(semicircles_to_rad(subf3.omega0)),
        );
        orbits.insert("cis".to_string(), OrbitItem::F64(subf3.cis));
        orbits.insert(
            "i0".to_string(),
            OrbitItem::F64(semicircles_to_rad(subf3.i0)),
        );
        orbits.insert("crc".to_string(), OrbitItem::F64(subf3.crc));
        orbits.insert(
            "omega".to_string(),
            OrbitItem::F64(semicircles_to_rad(subf3.omega)),
        );
        orbits.insert(
            "omegaDot".to_string(),
            OrbitItem::F64(semicircles_to_rad(subf3.omega_dot)),
        );
        orbits.insert(
            "idot".to_string(),
            OrbitItem::F64(semicircles_to_rad(subf3.idot)),
        );

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

    /// Dumps this [Ephemeris] frame as [GpsQzssEphemerisMessage] sequence,
    /// that is, the first 3/5 subframes of the message rotation.
    ///
    /// When composing, we define the ToW for each EPH-1, 2 and 3 correctly,
    /// as per the placement in time and GPS protocol specifications.
    ///
    /// You can then stream the resulting bit stream over the air or wire,
    /// it will still require proper CDMA modulation to match the physical
    /// signal. And you will also have to insert the ALM-4 and ALM-5 after EPH-3
    /// if you intend to create a physically realistic signal, or you can pad with zeros.
    ///
    /// ## Time management
    ///
    /// - the transmission time is `t_tm`, relative to the week of ToE (RINEX `week`),
    /// aligned to the beginning of its 30s frame. This makes the encoding insensitive to the
    /// `t_tm` convention of the RINEX producer (start of EPH-1, HOW instant...).
    ///
    /// - SF1 week number is the transmission week (modulo 1024), which may differ from the
    /// week of ToE.
    ///
    /// - HOW(EPH-n) is the beginning of the following subframe: frame start + 6n seconds.
    /// Timing is accurate and matches the GPS protocol.
    ///
    /// - SF1.toc and SF2.toe are seconds of their own week: they must be multiples of 16s.
    ///
    /// ## Input
    ///
    /// - opts: [GpsQzssEphCompositionOpts]
    /// that allow to compose 3/5 of the 30s cycle very precisely.
    ///
    /// ## Output
    ///
    /// - 3/5 of the message rotation, as [GpsQzssEphemerisMessage],
    /// ready to be encoded and transmitted.
    ///
    /// ```
    /// use rinex::prelude::{Constellation, Rinex};
    /// use rinex::navigation::{Ephemeris, GpsQzssEphemerisTelemetries};
    /// use gnss_protos::GpsQzssTelemetry;
    ///
    /// let rinex = Rinex::from_gzip_file("data/NAV/V2/cbw10010.21n.gz")
    ///     .unwrap();
    ///
    /// let tlm = GpsQzssTelemetry {
    ///     message: 0,
    ///     integrity: false,
    ///     reserved_bit: false,
    /// };
    ///
    /// for (key, ephemeris) in rinex.nav_ephemeris_frames_iter() {
    ///     let telemetries = GpsQzssEphemerisTelemetries {
    ///         tlm_1: tlm,
    ///         tlm_2: tlm,
    ///         tlm_3: tlm,
    ///     };
    ///
    ///     let message = ephemeris
    ///         .to_gps_qzss_eph_message(key.sv, key.epoch, false, false, telemetries)
    ///         .unwrap();
    ///
    ///     // reciprocal (RINEX [Ephemeris] frame, and its ToC)
    ///     let (toc, decoded) = Ephemeris::from_gps_qzss_eph_message(key.sv, key.epoch, false,
    ///         false, message)
    ///     .unwrap();
    ///
    ///     assert_eq!(toc, key.epoch);
    /// }
    /// ```
    pub fn to_gps_qzss_eph_message(
        &self,
        opts: GpsQzssEphCompositionOpts,
    ) -> Result<GpsQzssEphemerisMessage, Error> {
        match opts.satellite.constellation {
            Constellation::GPS | Constellation::QZSS => {},
            _ => return Err(Error::InvalidSatellite),
        }

        // ToC: seconds within its own GPST week
        let toc_gpst = opts.epoch.to_time_scale(TimeScale::GPST);
        let toc_nanos = toc_gpst.to_time_of_week().1;

        if toc_nanos % NANOSECONDS_PER_SECOND != 0 {
            return Err(Error::InvalidToC);
        }

        let toc_s = quantized_seconds_of_week(
            (toc_nanos / NANOSECONDS_PER_SECOND) as f64,
            GPS_TOC_TOE_LSB_S,
        )
        .ok_or(Error::InvalidToC)?;

        // ToE: seconds within the RINEX week
        let toe_week = self.get_week().ok_or(Error::MissingToE)?;
        let toe_s = self.get_orbit_f64("toe").ok_or(Error::MissingToE)?;
        let toe_s = quantized_seconds_of_week(toe_s, GPS_TOC_TOE_LSB_S).ok_or(Error::InvalidToE)?;
        let toe_gpst = gpst_epoch(toe_week, toe_s as u64);

        // Transmission time: t_tm is relative to the week of ToE,
        // it may be negative or exceed one week.
        let t_tm = self
            .get_orbit_f64("t_tm")
            .filter(|t_tm| t_tm.abs() < T_TM_UNKNOWN_THRESHOLD_S)
            .ok_or(Error::MissingTtm)?;

        let t_tx = gpst_epoch(toe_week, 0) + t_tm.floor() * Unit::Second;
        let (tx_week, tx_nanos) = t_tx.to_time_of_week();

        // align to the beginning of the frame (start of EPH-1)
        let tx_s = tx_nanos / NANOSECONDS_PER_SECOND;
        let frame_start_s = tx_s - tx_s % GPS_FRAME_DURATION_S as u64;
        let frame_start = gpst_epoch(tx_week, frame_start_s);

        // ToC/ToE must be recoverable from the transmission time
        if !within_half_week(toc_gpst, frame_start) || !within_half_week(toe_gpst, frame_start) {
            return Err(Error::InvalidEpoch);
        }

        // HOW: beginning of the following subframe, in seconds.
        // Frame start + 18s < 604800 for EPH-1,2,3: no week rollover possible.
        let how_tow = |n: u32| frame_start_s as u32 + n * GPS_SUBFRAME_DURATION_S;

        // Message identifiers
        let iodc = self.get_orbit_f64("iodc").unwrap_or_default() as u16;
        let iode = self.get_orbit_f64("iode").unwrap_or_default() as u8;

        if (iodc & 0xff) as u8 != iode {
            return Err(Error::IodcMismatch);
        }

        let l2_codes = self.get_orbit_f64("l2Codes").unwrap_or_default();
        let l2p_flag = self.get_orbit_f64("l2p").unwrap_or_default() > 0.0;
        let health = self.get_orbit_f64("health").unwrap_or_default();
        let tgd_s = self.get_orbit_f64("tgd").unwrap_or_default();
        let ura_m = self.get_orbit_f64("accuracy").unwrap_or_default();

        let mut subframe1 = GpsQzssFrame1::default()
            .with_week((tx_week % GPS_WEEK_ROLLOVER as u32) as u16)
            .with_iodc(iodc)
            .with_ca_or_p_l2_mask(l2_codes as u8)
            .with_health((health as u8).into())
            .with_time_of_clock_seconds(toc_s)
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
        let sqrt_a = self.get_orbit_f64("sqrta").unwrap_or_default();
        let mean_motion_diff_rad_s = self.get_orbit_f64("deltaN").unwrap_or_default();
        let fit_interval_h = self.get_orbit_f64("fitInt").unwrap_or_default();

        let mut subframe2 = GpsQzssFrame2::default()
            .with_aodo(0) // TODO: AoDo is not described in RINEX
            .with_toe_seconds(toe_s)
            .with_mean_anomaly_radians(m0_rad)
            .with_crs_meters(crs_m)
            .with_iode(iode)
            .with_cuc_radians(cuc_rad)
            .with_cus_radians(cus_rad)
            .with_eccentricity(ecc)
            .with_square_root_semi_major_axis(sqrt_a)
            .with_mean_motion_difference_radians(mean_motion_diff_rad_s);

        if fit_interval_flag(opts.satellite.constellation, fit_interval_h) {
            subframe2 = subframe2.with_fit_interval_flag();
        }

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
            .with_iode(iode)
            // IDOT is broadcast in semicircles/s
            .with_inclination_rate_semicircles_s(rad_to_semicircles(idot_rad_s))
            .with_longitude_ascending_node_radians(omega0_rad)
            .with_omega_radians(omega_rad)
            .with_omega_dot_rad_s(omega_dot_rad_s);

        let how = |n: u32, frame_id: GpsQzssFrameId| GpsQzssHow {
            frame_id,
            tow: how_tow(n),
            alert: opts.alert,
            anti_spoofing: opts.anti_spoofing,
        };

        Ok(GpsQzssEphemerisMessage {
            eph_1: GpsQzssFrame {
                how: how(1, GpsQzssFrameId::Ephemeris1),
                telemetry: opts.tlm_1,
                subframe: GpsQzssSubframe::Ephemeris1(subframe1),
            },
            eph_2: GpsQzssFrame {
                how: how(2, GpsQzssFrameId::Ephemeris2),
                telemetry: opts.tlm_2,
                subframe: GpsQzssSubframe::Ephemeris2(subframe2),
            },
            eph_3: GpsQzssFrame {
                how: how(3, GpsQzssFrameId::Ephemeris3),
                telemetry: opts.tlm_3,
                subframe: GpsQzssSubframe::Ephemeris3(subframe3),
            },
        })
    }
}

impl Rinex {
    // /// Dumps all satellite Ephemeris described in this RINEX file
    // /// to GPS bit stream ready to encode. This stream is composed
    // /// of a sequence of EPH-1, -2 and -3 frames which represents 3/5
    // /// of the 30s message rotation.

    // /// This method reconstructs all [GpsQzssMessage] 25-page rotations for the entire
    // /// GPS constellation, described in this entire Navigation RINEX record.
    // /// Each [GpsQzssMessage] is then ready to encode as stream of bits and possibly
    // /// sent over the air.
    // ///
    // /// This method aims at reconstructing the binary bit stream very accurately,
    // /// but that is not 100% feasible, because some of the bit stream information was lost
    // /// at the time the RINEX record was encoded.
    //
    // /// This method is very convenient for people working at the Almanach level.
    // /// Yet (general public) RINEX files are not the proper mean to save and share
    // /// Almanachs. For one reason, the usual 2h "sampling" rate is enough to describe
    // /// the evolution of the Constellation precisely, but is also very far from the
    // /// actualy bit stream over the air (12'30" period that one [GpsQzssMessage] covers).
    // /// If you "happen" to have a "fast" RINEX nav record (for example, sampler that saved
    // /// every single rotation), this method will be able to reconstruct every single indivual
    // /// frame. Otherwise, this is not feasible. We will grab each information for each SV (starting
    // /// at G01) and we will reconstruct every possible [GpsQzssMessage] for each G01 entry.
    // /// You will inevitably get gaps in the successive [GpsQzssMessage] serie.
    // ///
    // /// Information that we reconstruct exactly:
    // /// - Ionosphere model parameters. This is either described in the file header,
    // /// or regularly updated in modern RINEX records. In RINEX V2 we will copy
    // /// the model parameters in _every single_ GPS page. In RINEX V4 the
    // /// model parameters are regularly updated and this method reconstructs the binary
    // /// stream more precisely.
    // ///
    // /// Information that cannot be fully reconstructed:
    // /// - SV Almanach: (PRN dependent) almanach reconstruction is not 100% bit true.
    // /// SV almanach parameters are keplerian parameters for each SV, without
    // /// harmonic terms with lower precision compared to SV ephemeris.
    // /// - This method is not an orbit fitter & designer. It is a quick & easy to use tool
    // /// to recreate a somewhat realistic [GpsQzssMessage] bitstream. Therefore, the quality
    // /// of the reconstructed Almanach should be good enough, but not ideal.
    // ///
    // /// Information that is inevitably lost at RINEX encoding:
    // /// - RESERVED and military bits.
    // /// - Special Message (SF4 p17 22 ASCII bytes) are not encoded in RINEX records (military?).
    // /// - RESERVED SF4 pages 1, 6, 11, 12, 14, 15, 16, 19-24 are not encoded in RINEX records.
    // /// - This method will insert zeros for all these bits in the bit stream.
    // ///
    // /// ## Input
    // ///
    // /// - Navigation RINEX record. Other formats will return an empty Iterator.
    // ///
    // /// ## Output
    // ///
    // /// - [GpsQzssMessage] Iterator, ideally starting at Eph-1 G01.
    // /// Since our structure is automatically sorted, the output strem reflects just that,
    // /// and is possible "truncated" with missing part of the 25-pages, if the record
    // /// does not start with G01. Note that this is totally sane and will happen,
    // /// because the RINEX record is formatted at some location in space and time,
    // /// from direct sky observations.
    // pub fn to_gps_message_rotations(&self) -> Box<dyn Iterator<Item = GpsQzssMessage> + '_> {
    //     Self::to_gps_qzss_message_rotations(self, Constellation::GPS)
    // }
    //
    // /// Refer to [Rinex::to_gps_message_rotation] for more information.
    // /// Same behavior, applied to QZSS (Jap) satellites.
    // pub fn to_qzss_message_rotations(&self) -> Box<dyn Iterator<Item = GpsQzssMessage> + '_> {
    //     Self::to_gps_qzss_message_rotations(self, Constellation::QZSS)
    // }

    // /// See [Rinex::to_gps_message_rotation] for more information.
    // fn to_gps_qzss_message_rotations(&self, filter: Constellation) -> Box<dyn Iterator<Item = GpsQzssMessage> + '_> {

    // }

    /// Serializes this Navigation RINEX into a serie of GPS/QZSS EPH-1, EPH-2 and EPH-3
    /// frames, which compose the first 3/5 of the 30s message rotation of each satellite.
    ///
    /// It is possible to extract the first 3/5 of the burst for every single [Ephemeris]
    /// entry. When composing the burst, we let you customize the [GpsQzssTelemetry] fields
    /// and other flags, which allows advanced users to precisely control the bit stream
    /// they will compose. To do that, all you have to do is to implement the provided
    /// function pointer, which will be invoked for every single EPH-1, 2, 3 burst
    /// that we are about to serialize. For each burst, you can attach the HOW alert bit,
    /// the HOW A/S bit and each [GpsQzssTelemetry] field. All of those can vary in time,
    /// we pass the satellite ID and the Time of Clock [Epoch] which should be enough
    /// for you to implement your custom logic. If you don't know how to compose
    /// those value, simply implement a function that returns [Default] values
    /// and all those bits will be zeros.
    ///
    /// The rotation being incomplete, if you intend to synthesize a realistic signal
    /// you will have to insert the ALM-4 and ALM-5 after each EPH-3 frame, or you can insert
    /// zeros.
    pub fn to_gps_qzss_eph_messages<'a, C>(
        &'a self,
        mut composer: C,
    ) -> Box<dyn Iterator<Item = GpsQzssEphemerisItem> + 'a>
    where
        C: FnMut((SV, Epoch)) -> GpsQzssEphCompositionOpts + 'a,
    {
        Box::new(self.nav_ephemeris_frames_iter().filter_map(move |(key, eph)| {
            match key.sv.constellation {
                Constellation::GPS | Constellation::QZSS => {
                    let opts = composer((key.sv, key.epoch));
                    match eph.to_gps_qzss_eph_message(opts) {
                        Ok(msg) => Some(GpsQzssEphemerisItem {
                            satellite: key.sv,
                            epoch: key.epoch,
                            eph123: msg,
                        }),
                        Err(_e) => {
                            #[cfg(feature = "log")] {
                                error!("failed to convert to GPS/QZSS EPH1-2-3 message: {} ({}@{})", _e, key.sv, key.epoch);
                            }
                            None
                        }
                    }
                },
                _ => None, // naturally dropped
            }
        }))
    }

    /// Creates a complete Navigation RINEX from a list of collected [GpsQzssFrame]s.
    /// Whether this describes a complete 12'30" rotation does not matter, but provided
    /// content obviously impacts the resulting RINEX.
    ///
    /// ## Input
    ///
    /// - header: provide the RINEX [Header] so you can compose
    /// the record as you will. In particular, the RINEX revision
    /// defines the formatting method and future behavior.
    /// For example you can define your receiver, and mark the amount
    /// of collected LNAV messages in header comments.
    ///
    /// - attributes: provude custom [ProductionAttributes] that will
    /// be attached to this RINEX file, mostly used when dumping into a text file.
    /// When not provided, this method will guess the best attributes that
    /// fit the actual data payload.
    ///
    /// ## Output
    ///
    /// - Navigation [Rinex] file.
    pub fn from_gps_qzss_eph_messages(
        &self,
        header: Header,
        attributes: Option<ProductionAttributes>,
        iterator: impl Iterator<Item = GpsQzssEphemerisItem>,
    ) -> Result<Self, Error> {
        let mut record = NavRecord::default();

        for item in iterator {
            match item.satellite.constellation {
                Constellation::GPS | Constellation::QZSS => {
                    // proceed
                    match Ephemeris::from_gps_qzss_eph_message(item) {
                        Ok((epoch, ephemeris)) => {
                            let key = NavKey {
                                epoch: item.epoch,
                                sv: item.satellite,
                                msgtype: NavMessageType::LNAV,
                                frmtype: NavFrameType::Ephemeris,
                                subtype: None,              // not applicable here
                                galileo_data_sources: None, // not applicable here
                                sto_identity: None,         // not applicable here
                                transmission_time: None,    // TODO
                            };
                            record.insert(key, NavFrame::EPH(ephemeris));
                        },
                        Err(_e) => {
                            // #[cfg(feature = "log")]
                            // error!("failed to decode ephemeris from {:?} ({}@{}): {}", item.eph123, item.satellite, item.epoch.clone(), _e);
                        },
                    }
                },
                _ => {}, // drop: invalid API usage
            }
        }

        let mut rinex = Rinex {
            header,
            comments: Default::default(),
            record: Record::NavRecord(record),
            production: ProductionAttributes::default(),
        };

        if let Some(attributes) = attributes {
            rinex.production = attributes;
        } else {
            rinex.production = rinex.guess_production_attributes();
        }

        Ok(rinex)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn week_unwrapping() {
        let t = |week: u32| gpst_epoch(week, 0);

        assert_eq!(unwrapped_week_number(t(2138), 90), 2138);
        assert_eq!(unwrapped_week_number(t(2047), 0), 2048);
        assert_eq!(unwrapped_week_number(t(2048), 1023), 2047);
        assert_eq!(unwrapped_week_number(t(100), 1000), 1000);
        assert_eq!(unwrapped_week_number(t(1500), 10), 1034);
    }

    #[test]
    fn closest_reference_epoch() {
        // transmitted saturday 22:00, ToC sunday 00:00 (next week)
        let t_tx = gpst_epoch(2399, 597_600);
        assert_eq!(closest_epoch(0, t_tx), gpst_epoch(2400, 0));

        // transmitted sunday 00:30, ToC saturday 23:59:44 (previous week)
        let t_tx = gpst_epoch(2400, 1_800);
        assert_eq!(closest_epoch(604_784, t_tx), gpst_epoch(2399, 604_784));

        // nominal
        let t_tx = gpst_epoch(2400, 428_370);
        assert_eq!(closest_epoch(431_984, t_tx), gpst_epoch(2400, 431_984));
    }

    #[test]
    fn nominal_ura_index_reciprocity() {
        for (index, ura_m) in NOMINAL_URA_M.iter().enumerate() {
            let frame = GpsQzssFrame1::default().with_user_range_accuracy_m(*ura_m);
            assert_eq!(frame.ura as usize, index, "URA index {}", index);
        }
    }

    #[test]
    fn fit_interval_reciprocity() {
        for constellation in [Constellation::GPS, Constellation::QZSS] {
            for flag in [false, true] {
                for iodc in [0, 240, 250, 496, 500, 1022] {
                    let hours = fit_interval_hours(constellation, flag, iodc);
                    assert_eq!(fit_interval_flag(constellation, hours), flag);
                }
            }
        }
    }
}
