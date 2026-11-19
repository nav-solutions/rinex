//! GPS/QZSS bitstream and Ephemeris serdes operations
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
        GpsQzssFrame, GpsQzssFrame1, GpsQzssFrame2, GpsQzssFrame3, GpsQzssFrame4, GpsQzssFrame5,
        GpsQzssFrameId, GpsQzssHow, GpsQzssMessage, GpsQzssSubframe, GpsQzssTelemetry,
        GPS_FRAME_DURATION_S, GPS_QZSS_ROTATION_ITEMS, GPS_SUBFRAME_DURATION_S, GPS_TOC_TOE_LSB_S,
        GPS_WEEK_ROLLOVER,
    },
};

#[cfg(doc)]
use gnss_protos::gps::{GpsQzssAlmanach, GpsQzssAlmanachHealth, GpsQzssIonosphereUtc};

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

/// 30s GPS/QZSS burst formed by [GPS_QZSS_ROTATION_ITEMS] consecutive
/// [GpsQzssFrame]s, following the correct frame progression
/// according to the GPS/QZSS protocol. These are often times referred
/// to as "pages" in the overall message. We have 25 bursts (or pages)
/// in the complete message describing the entire constellation,
/// which takes 12'30".
/// We propose this wrapper to simplify the returned types that may
/// be returned from RINEX files.
#[derive(Debug, Copy, Clone, Default)]
pub struct GpsQzss30sMessage([GpsQzssFrame; GPS_QZSS_ROTATION_ITEMS]);

impl GpsQzss30sMessage {
    /// Access the underlying EPH-1 1/5 part of the 30s burst.
    pub fn eph_1(&self) -> GpsQzssFrame {
        self.0[0]
    }

    /// Access the underlying EPH-2 2/5 part of the 30s burst.
    pub fn eph_2(&self) -> GpsQzssFrame {
        self.0[1]
    }

    /// Access the underlying EPH-3 3/5 part of the 30s burst.
    pub fn eph_3(&self) -> GpsQzssFrame {
        self.0[2]
    }

    /// Access the underlying ALM-4 4/5 part of the 30s burst.
    pub fn alm_4(&self) -> GpsQzssFrame {
        self.0[3]
    }

    /// Access the underlying ALM-5 5/5 conclusion of the 30s burst.
    pub fn alm_5(&self) -> GpsQzssFrame {
        self.0[4]
    }

    /// Convenient method to extract the first 3/5 part of the 30s cycle,
    /// which can always be serdes from a RINEX [Ephemeris].
    pub fn to_ephemeris_message_item(&self, satellite: SV, epoch: Epoch) -> GpsQzssEphemerisItem {
        GpsQzssEphemerisItem {
            satellite,
            epoch,
            eph123: GpsQzssEphemerisMessage {
                eph_1: self.eph_1(),
                eph_2: self.eph_2(),
                eph_3: self.eph_3(),
            },
        }
    }
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

/// Similarly to [GpsQzssEphCompositionOpts] (refer to related docs),
/// except that you also control the Telemetry fields to apply to
/// ALM-4 and ALM-5 at the conclusion of the 30s burst.
#[derive(Debug, Copy, Clone, Default)]
pub struct GpsQzssCompositionOpts {
    /// Satellite signal source to be emulated or synthesized.
    /// Only GPS or QZSS satellites will be accepted here.
    pub satellite: SV,

    /// HOW alert flag that will apply to the entire 30s cycle.
    pub alert: bool,

    /// HOW A/S flag, that will apply to the entire 30s cycle.
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

    /// [GpsQzssTelemetry] to be attached to the ALM-3 frame (4/5).
    /// Accurate control over each telemetry is most interesting for advanced
    /// users who can take advantage of that to encode the private bits of the bitstream.
    pub tlm_4: GpsQzssTelemetry,

    /// [GpsQzssTelemetry] to be attached to the ALM-5 frame (5/5).
    /// Accurate control over each telemetry is most interesting for advanced
    /// users who can take advantage of that to encode the private bits of the bitstream.
    pub tlm_5: GpsQzssTelemetry,
}

/// [GpsQzssAlm4CompositionOpts] allows you to compose
/// ALM-4 accurately (4/5 in the 30s cycle)
#[derive(Debug, Copy, Clone)]
pub enum GpsQzssAlm4CompositionOpts {
    /// Pass [GpsQzssFrame4] directly.
    /// We will use this data word to follow EPH-3 and preceeding ALM-5
    /// in the 30s cycle we are about to forge.
    /// This is convenient if you are capable of forging specific & exact
    /// part of the bitstream.
    AsIs(GpsQzssFrame4),

    /// This option will force our burst synthesizer
    /// to compute the most accurate Almanach parameters
    /// we can, for the attached satellite.
    /// Basically, the solver will fit [GpsQzssAlmanach] for this satellite.
    SatelliteAlmanach,

    /// Use this option to force our burst synthesizer
    /// to grab the Ionospheric model parameters
    /// and GPST/UTC current state, from closest RINEX data point
    /// when working with RINEX v4, or from file header when
    /// working with RINEX v2/v3.
    /// Basically, this will forge the most realistic [GpsQzssIonosphereUtc] we can.
    /// You can still provide the 14 reserved bits concluding Word10.
    /// This is automated ALM4 Page 18 termination.
    IonosphereUtc(Option<u16>),
}

/// [GpsQzssAlm4CompositionOpts] allows you to compose
/// ALM-5 accurately (5/5 in the 30s cycle)
#[derive(Debug, Copy, Clone)]
pub enum GpsQzssAlm5CompositionOpts {
    /// Pass [GpsQzssFrame5] directly.
    /// We will use this data word to follow ALM-4 and conclude the 30s
    /// cycle we are about to forge.
    /// This is convenient if you are capable of forging specific & exact
    /// part of the bitstream.
    AsIs(GpsQzssFrame5),

    /// This option will force our burst synthesizer
    /// to compute the most accurate Almanach parameters
    /// we can, for the attached satellite.
    /// Basically, the solver will fit [GpsQzssAlmanach] for this satellite.
    SatelliteAlmanach,

    /// This option forces our burst synthesize to grab the latest
    /// health flags for SV1-24, compute the WNa and ToA.
    /// You can still provide the 22 reserved bits concluding Word10.
    /// This is ALM5 page 25 automated termination.
    AlmanacHealth(Option<u32>),
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
    ///
    /// - t_rx: usually [Epoch] of message reception.
    /// In practice, any [Epoch] that is less than 512 weeks from
    /// the actual transmission time of the following message will work fine here.
    /// Any [TimeScale] supported, including [TimeScale::UTC].
    ///
    /// - satellite: GPS or QZSS [SV]
    ///
    /// ## Output
    ///
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
    /// that is, the first 3/5 of the 30s message cycle.
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
    ///
    /// use rinex::navigation::{
    ///     Ephemeris,
    ///     GpsQzssEphCompositionOpts,
    ///     GpsQzssEphemerisItem,
    /// };
    ///
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
    ///
    ///     let opts = GpsQzssEphCompositionOpts {
    ///         // this should will be required by your CDMA modulator.
    ///         // Also this is the "identity" of the signal source
    ///         satellite: key.sv,
    ///
    ///         // _any_ instant may apply here, as long as it lies within +/- 512
    ///         // weeks from the transmission time. Here we have the ToC
    ///         // for each RINEX entry, which is very convenient and easy to use
    ///         epoch: key.epoch,
    ///
    ///         // Alert bit associated to each HOW data word of this burst
    ///         alert: false,
    ///
    ///         // A/S bit associated to each HOW data word of this burst
    ///         anti_spoofing: false,
    ///
    ///         // TLM to be associated to EPH-1
    ///         tlm_1: GpsQzssTelemetry {
    ///             message: 1,
    ///             integrity: false, // whatever you want
    ///             reserved_bit: false, // whatever you want
    ///         },
    ///
    ///         // TLM to be associated to EPH-2
    ///         tlm_2: GpsQzssTelemetry {
    ///             message: 2,
    ///             integrity: false, // whatever you want
    ///             reserved_bit: false, // whatever you want
    ///         },
    ///
    ///         // TLM to be associated to EPH-3
    ///         tlm_3: GpsQzssTelemetry {
    ///             message: 3,
    ///             integrity: false, // whatever you want
    ///             reserved_bit: false, // whatever you want
    ///         },
    ///     };
    ///
    ///     let eph123 = ephemeris
    ///         .to_gps_qzss_eph_message(opts)
    ///         .unwrap();
    ///
    ///     let item = GpsQzssEphemerisItem {
    ///         eph123,
    ///         satellite: key.sv,
    ///         epoch: key.epoch,
    ///     };
    ///
    ///     // reciprocal (RINEX [Ephemeris] frame, and its ToC)
    ///     let (toc, decoded) = Ephemeris::from_gps_qzss_eph_message(item)
    ///         .unwrap();
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

    /// Any GPS/QZSS RINEX [Ephemeris] frame can be streamed to a complete 30s GPS/QZSS bitstream,
    /// as long as you can provide the ALM4 and ALM5 concluding the 30s cycle.
    /// Refer to [Self::to_gps_qzss_eph_message] for more information.
    ///
    /// ## Input
    ///
    /// - eph123_opts: [GpsQzssCompositionOpts] options that apply to the first 3/5 part
    /// of the burst: EPH-1, EPH-2 and EPH-3.
    ///
    /// - alm4: externally defined [GpsQzssFrame] which must be
    /// a valid ALM-4 subframe to follow EPH-3 correctly.
    /// This method will reject other subframes for this type.
    /// If you don't know how to provide ALM4, simply pass None.
    /// When passing None, a "zero" ALM4 is described, with correct HOW.ToW.
    ///
    /// - alm5: externally defined [GpsQzssFrame] which must be
    /// a valid ALM-5 subframe to follow ALM-4 correctly.
    /// This method will reject other subframes for this type.
    /// If you don't know how to provide ALM5, simply pass None.
    /// When passing None, a "zero" ALM5 is described, with correct HOW.ToW.
    /// ```
    /// use rinex::{
    ///     prelude::{
    ///         Rinex, Constellation,
    ///     },
    ///     navigation::{
    ///         Ephemeris,
    ///         GpsQzssEphCompositionOpts,
    ///     },
    /// };
    ///
    /// use gnss_protos::gps::GPS_SUBFRAME_DURATION_S;
    ///
    /// let rinex =
    ///     Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();
    ///
    /// // Any GPS/QZSS RINEX Ephemeris can be converted to a complete 30s
    /// // radio burst, as long as you can provide ALM4 and ALM5.
    /// // In this example, we pretend to be a general public user who
    /// // is not capable to do so, we take advantage of the Default constructor
    /// // to create dummy ALM4 and ALM5 subframes lazily. For this operation
    /// // to work, we still have to create ALM4 and ALM5 subframes correctly,
    /// // which _are not_ the default subframes!
    /// for (key, eph) in rinex.nav_ephemeris_frames_iter() {
    ///     match key.sv.constellation {
    ///         Constellation::GPS | Constellation::QZSS => {
    ///             // applies similarly to GPS and QZSS (same protocol)
    ///
    ///             // lazy and general public use case:
    ///             // unable to describe most ALM4 or ALM5 pages.
    ///             // The end of the 30s burst will be zero padded.
    ///             let (alm4, alm5) = (None, None);
    ///
    ///             // it is important to define the correct satellite.
    ///             // This information should be shared with your signal modulator anyway.
    ///             // It is important to provide a reference Epoch here which lies
    ///             // within +/- 512 weeks from physical transmission time.
    ///             // Simply and lazily pass the RINEX ToC here.
    ///             let opts = GpsQzssEphCompositionOpts {
    ///                 satellite: key.sv,
    ///                 epoch: key.epoch,
    ///                 alert: false, // example
    ///                 anti_spoofing: true, // example
    ///                 tlm_1: Default::default(), // example
    ///                 tlm_2: Default::default(), // example
    ///                 tlm_3: Default::default(), // example
    ///             };
    ///
    ///             let radio_burst = eph.to_gps_qzss_message(opts, alm4, alm5)
    ///                 .unwrap();
    ///
    ///             // simple verification:
    ///             // even when stream is lazily zero padded,
    ///             // this API is smart enough to forge correct ToW progression.
    ///             // Which is physicall correct.
    ///             assert_eq!(
    ///                 radio_burst.eph_2().how.tow - radio_burst.eph_1().how.tow,
    ///                 GPS_SUBFRAME_DURATION_S,
    ///             );
    ///             assert_eq!(
    ///                 radio_burst.eph_3().how.tow - radio_burst.eph_2().how.tow,
    ///                 GPS_SUBFRAME_DURATION_S,
    ///             );
    ///             assert_eq!(
    ///                 radio_burst.alm_4().how.tow - radio_burst.eph_3().how.tow,
    ///                 GPS_SUBFRAME_DURATION_S,
    ///             );
    ///             assert_eq!(
    ///                 radio_burst.alm_5().how.tow - radio_burst.alm_4().how.tow,
    ///                 GPS_SUBFRAME_DURATION_S,
    ///             );
    ///             
    ///             // At this point you are ready to modulate & transmit over the air.
    ///             // In this example, we verify our reciprocity.
    ///             // Only the first 3/5 part of the burst applies here,
    ///             // almanach is dropped.
    ///             let eph123 = radio_burst.to_ephemeris_message_item(key.sv, key.epoch);
    ///
    ///             let (reconstructed_toc, reconstructed_eph) =
    ///                 Ephemeris::from_gps_qzss_eph_message(eph123)
    ///                 .unwrap();
    ///
    ///             // bit true reciprocity
    ///             assert_eq!(reconstructed_toc, key.epoch, "ToC reciprocal failed");
    ///             
    ///             // reconstructed ephemeris is "bit true" at the GPS/QZSS
    ///             // binary precision level, which is lower than f64 assert_eq.
    ///             // Our test toolkit verifies just that, but this macro
    ///             // is not avaible here. assert_eq! would "fail" at 1e-7 range.
    ///         },
    ///         _ => {}, // not applicable
    ///     }
    /// }
    /// ```
    pub fn to_gps_qzss_message(
        &self,
        eph123_opts: GpsQzssEphCompositionOpts,
        alm_4: Option<GpsQzssFrame>,
        alm_5: Option<GpsQzssFrame>,
    ) -> Result<GpsQzss30sMessage, Error> {
        match eph123_opts.satellite.constellation {
            Constellation::GPS | Constellation::QZSS => {},
            _ => return Err(Error::InvalidSatellite),
        }

        let eph123 = self.to_gps_qzss_eph_message(eph123_opts)?;

        let alm_4 = match alm_4 {
            Some(alm) => alm,
            None => {
                // for 100% correct ToW, even in this case
                let start_time_s = eph123.eph_1.frame_start_time_s();

                GpsQzssFrame {
                    telemetry: Default::default(),
                    how: GpsQzssHow {
                        frame_id: GpsQzssFrameId::Almanach4,
                        tow: start_time_s as u32 + 4 * GPS_SUBFRAME_DURATION_S,
                        alert: eph123_opts.alert,
                        anti_spoofing: eph123_opts.anti_spoofing,
                    },
                    subframe: GpsQzssSubframe::Almanach4(Default::default()),
                }
            },
        };

        let alm_5 = match alm_5 {
            Some(alm) => alm,
            None => {
                // for 100% correct ToW, even in this case
                let start_time_s = eph123.eph_1.frame_start_time_s();

                GpsQzssFrame {
                    telemetry: Default::default(),
                    how: GpsQzssHow {
                        frame_id: GpsQzssFrameId::Almanach5,
                        tow: start_time_s as u32 + 5 * GPS_SUBFRAME_DURATION_S,
                        alert: eph123_opts.alert,
                        anti_spoofing: eph123_opts.anti_spoofing,
                    },
                    subframe: GpsQzssSubframe::Almanach5(Default::default()),
                }
            },
        };

        Ok(GpsQzss30sMessage([
            eph123.eph_1,
            eph123.eph_2,
            eph123.eph_3,
            alm_4,
            alm_5,
        ]))
    }
}

impl Rinex {
    /// This method is very similar to [Ephemeris::to_gps_qzss_eph_message],
    /// excepts that it applies at the [Rinex] level, and takes advantage
    /// of the entire record set and file Header, to form an entire 30s GPS/QZSS burst.
    ///
    /// When using [Ephemeris::to_gps_qzss_eph_message], you are limited
    /// to synthesis of EPH-1, 2 and 3 which from the first 3/5 of the 30s burst.
    /// The ALM-4 and ALM-5 subframes concluding the 30s burst are missing
    /// inevitably. You can take advantage of this method to compose
    /// a realistic and complete 30s message burst, either by deducing
    /// some information from the File Header or Record itself,
    /// or by manually defining them using the composition options.
    ///
    /// Thanks to these composition options, this method allows you to synthesize
    /// a 30s burst that will fit correctly in any part of the 12'30" message rotation.
    ///
    /// Read [Alm4CompositionOpts] and [Alm5CompositionOpts] options carefully
    /// to understand how to conclude the burst correctly. In particular,
    /// some of the information transported by ALM4 and ALM5 cannot be deduced
    /// from a RINEX file. Depending on which part of the 12'30" you intend
    /// to synthesize, you should feed the information directly yourself,
    /// otherwise this method can only pad the bitstream with zeros.
    ///
    /// Like similar methods, it gives you full control to compose the radio burst
    /// very precisely, in particular Telemetry fields for advanced users.
    ///
    /// ## Input
    ///
    /// - Navigation [Rinex]. Returns an Error on other format (invalid operation).
    ///
    /// - composition opts, as [GpsQzssCompoitionOpts], to fine tune each bit
    /// of the data burst to be synthesized. This is most useful to advanced users.
    ///
    /// - alm4: ALM4 accurate design.
    /// This is directly related to the placement in time you intend for the 30s burst
    /// we are about to synthesize.
    ///
    /// - alm5: ALM5 accurate design.
    /// This is directly related to the placement in time you intend for the 30s burst
    /// we are about to synthesize.
    pub fn to_gps_qzss_messages(
        &self,
        opts: GpsQzssCompositionOpts,
        alm4: GpsQzssAlm4CompositionOpts,
        alm5: GpsQzssAlm5CompositionOpts,
    ) -> Result<GpsQzss30sMessage, Error> {
        match opts.satellite.constellation {
            Constellation::GPS | Constellation::QZSS => {},
            _ => {
                return Err(Error::InvalidSatellite);
            },
        }

        Ok(Default::default())
    }

    /// Serializes this Navigation RINEX into a serie of GPS/QZSS EPH-1, EPH-2 and EPH-3
    /// frames, which compose the first 3/5 of the 30s message sent by every satellite.
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
    /// zeros. Basically, this method forges 18s of the 30s cycle.
    /// ```
    /// use rinex::{
    ///     prelude::Rinex,
    ///     navigation::{
    ///         GpsQzssEphCompositionOpts,
    ///     },
    /// };
    ///
    /// use gnss_protos::gps::GPS_SUBFRAME_DURATION_S;
    ///  
    /// let rinex = Rinex::from_gzip_file("data/NAV/V2/cbw10010.21n.gz").unwrap();
    ///
    /// // simple example
    /// let (mut alert, mut anti_spoofing) = (false, false);
    ///
    /// // Dump this RINEX to GPS bitstream.
    /// // QZSS will not exist because this is a GPS V2 Only file.
    /// // If you intend to forge a physically correct bitstream, you should
    /// // insert ALM4 and ALM5 subframes after each EPH-3.
    /// // This method will basically forge 18s of message, it still lacks 12s to complete the 30s
    /// // cycle.
    /// for msg in rinex.to_gps_qzss_eph_messages(|(satellite, toc)| {
    ///     // Define your composition method to customize each EPH-1,2,3 message.
    ///     // Every single RINEX Ephemeris data point is about to be
    ///     // converted to EPH-1,2,3 burst. You can fine tune each bit
    ///     // in each frame. For that, we pass the satellite and ToC,
    ///     // and we expect you to fulfill the remaining bits.
    ///     // In this example, we toggle the Alert and A/S bits
    ///     // every other epoch, and we fill the telemetry with zeros.
    ///     let opts = GpsQzssEphCompositionOpts {
    ///         satellite, // simple pass through
    ///         epoch: toc, // simple pass through
    ///         anti_spoofing,
    ///         alert,
    ///         tlm_1: Default::default(), // zeros
    ///         tlm_2: Default::default(), // zeros
    ///         tlm_3: Default::default(), // zeros
    ///     };
    ///
    ///     alert = !alert; // toggle every other burst, regardless of SV
    ///     anti_spoofing = !anti_spoofing; // toggles every other burst, regardless of SV
    ///     
    ///     // return composition choice for this very burst
    ///     opts
    /// }) {
    ///
    ///     // your signal modulator will most likely need this information
    ///     let (sat, epoch_toc) = (msg.satellite, msg.epoch);
    ///
    ///     // verify GPS/QZSS ToW correctness.
    ///     // Remember, this method only forges 18s of the 30s cycle.
    ///     assert_eq!(
    ///         msg.eph123.eph_2.how.tow - msg.eph123.eph_1.how.tow,
    ///         GPS_SUBFRAME_DURATION_S,
    ///     );
    ///     assert_eq!(
    ///         msg.eph123.eph_3.how.tow - msg.eph123.eph_2.how.tow,
    ///         GPS_SUBFRAME_DURATION_S,
    ///     );
    /// }
    /// ```
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
