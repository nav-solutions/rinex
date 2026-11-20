//! GPS/QZSS bitstream and Ephemeris serdes operations
use hifitime::{
    prelude::{TimeSeries, Unit},
    NANOSECONDS_PER_SECOND,
};

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

use itertools::Itertools;

#[cfg(doc)]
use gnss_protos::gps::{GpsQzssAlmanach, GpsQzssAlmanachHealth, GpsQzssIonosphereUtc};

#[cfg(feature = "log")]
use log::{debug, error};

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
pub struct GpsQzss30sMessage {
    /// Satellite for which this 30s burst was reconstructed.
    /// This information will be required by your signal modulator.
    pub satellite: SV,

    /// Placement in time, usually intended transmission time.
    pub epoch: Epoch,

    /// Decoded or reconstructed [GpsQzssFrame]s burst. We provide
    /// helper functions to deal with them.
    pub frames: [GpsQzssFrame; GPS_QZSS_ROTATION_ITEMS],
}

impl GpsQzss30sMessage {
    /// Access the underlying EPH-1 1/5 part of the 30s burst.
    pub fn eph_1(&self) -> GpsQzssFrame {
        self.frames[0]
    }

    /// Access the underlying EPH-2 2/5 part of the 30s burst.
    pub fn eph_2(&self) -> GpsQzssFrame {
        self.frames[1]
    }

    /// Access the underlying EPH-3 3/5 part of the 30s burst.
    pub fn eph_3(&self) -> GpsQzssFrame {
        self.frames[2]
    }

    /// Access the underlying ALM-4 4/5 part of the 30s burst.
    pub fn alm_4(&self) -> GpsQzssFrame {
        self.frames[3]
    }

    /// Access the underlying ALM-5 5/5 conclusion of the 30s burst.
    pub fn alm_5(&self) -> GpsQzssFrame {
        self.frames[4]
    }
}

/// [GpsQzssCompositionOpts] allows you to control accuractly each bits
/// of the bitstream we are about to forge. In particular,
/// each alert and A/S bits HOW headers, each [GpsQzssTelemetry]
/// to be associated to each subframe. And, when that applies,
/// telemetries for ALM-4 and ALM-5 and design choices to possibly
/// have the RINEX parser guess the ALM-4 and ALM-5 information,
/// which is not feasible 100% of the time, depending where the
/// attached Epoch lies in the 12'30" GPS period.
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

    /// Epoch of composition, which is usually the intended transmitted time.
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

    /// Possibly define ALM-4 yourself, otherwise,
    /// let the burst synthesizer try to reconstruct this information.
    /// Depending where we are in the GPS epoch, that may not be feasible.
    pub alm_4: Option<GpsQzssFrame4>,

    /// [GpsQzssTelemetry] to be attached to the ALM-5 frame (5/5).
    /// Accurate control over each telemetry is most interesting for advanced
    /// users who can take advantage of that to encode the private bits of the bitstream.
    pub tlm_5: GpsQzssTelemetry,

    /// Possibly define ALM-4 yourself, otherwise,
    /// let the burst synthesizer try to reconstruct this information.
    /// Depending where we are in the GPS epoch, that may not be feasible.
    pub alm_5: Option<GpsQzssFrame5>,
}

impl Ephemeris {
    /// Builds a RINEX [Ephemeris] frame decoded [GpsQzss30sMessage].
    /// Applies to any GPS or QZSS satellites.
    ///
    /// This method guards against invalid GPS/QZSS message associations,
    /// it is illegal to group messages that do not belong to the same
    /// satellite & valid rotation (consecutive HOW, matching IODC/IODE).
    ///
    /// Note that the ALM-4 and ALM-5 are disregarded by this method.
    /// You can have a zero terminated 30s cycle and still be able to reconstruct
    /// valid valid RINEX [Ephemeris]. In otherwords, only the first 18s
    /// of the 30s GPS cycle are useful to reconstruct this object.
    ///
    /// ## Time management
    ///
    /// - the transmission time is the beginning of EPH-1 (HOW(EPH-1) - 6s),
    /// which is the beginning of the frame. It is written as `t_tm`,
    /// in seconds relative to the week of ToE (possibly negative).
    ///
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
    /// - (ToC) Time of Clock, expressed in [TimeScale::GPST]
    /// - and [Ephemeris] frame
    pub fn from_gps_qzss_message(msg: GpsQzss30sMessage) -> Result<(Epoch, Self), Error> {
        match msg.satellite.constellation {
            Constellation::GPS | Constellation::QZSS => {},
            _ => return Err(Error::InvalidSatellite),
        }

        let mut orbits = HashMap::<String, OrbitItem>::new();

        let (eph_1, eph_2, eph_3) = (msg.eph_1(), msg.eph_2(), msg.eph_3());

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

        let t_rx_gpst = msg.epoch.to_time_scale(TimeScale::GPST);
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
                msg.satellite.constellation,
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

    /// Converts this [Ephemeris] frame to [GpsQzss30sMessage] burst,
    /// ready to encode.
    ///
    /// NB: it is impossible to reconstruct ALM-4 and ALM-5 from this object,
    /// only the first 18s composed of EPH-1, 2, 3 subframes can be fully reconstructed.
    /// Therefore, you should compose ALM4 and ALM5 yourself, using the provided
    /// options. if you don't know how to do that, simply use the default constructor
    /// like in the following example.
    ///
    /// When composing, we define the ToW correctly to respect the GPS
    /// protocol. If you intend to define ALM-4 and ALM-5 externally yourself,
    /// we will disregard the provided ToW and encode it ourselves to garantee 100%
    /// correctness.
    ///
    /// If you are able to CDMA modulate this burst, you can then transmit over the air.
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
    /// - opts: [GpsQzssCompositionOpts]
    ///
    /// ## Output
    ///
    /// - EPH 1.2.3 ALM-4 and ALM-5 burst as [GpsQzss30sMessage]
    ///
    /// ```
    /// use rinex::prelude::{Constellation, Rinex};
    ///
    /// use rinex::navigation::{
    ///     Ephemeris,
    ///     GpsQzssCompositionOpts,
    /// };
    ///
    /// use gnss_protos::gps::{
    ///     GpsQzssTelemetry,
    ///     GPS_SUBFRAME_DURATION_S,
    /// };
    ///
    /// // load GPS (only, v2)
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
    ///     let opts = GpsQzssCompositionOpts {
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
    ///
    ///         // TLM to be associated to ALM-4
    ///         tlm_4: GpsQzssTelemetry {
    ///             message: 4,
    ///             integrity: false, // whatever you want
    ///             reserved_bit: false, // whatever you want
    ///         },
    ///
    ///         // ALM-4 cannot be deduced from RINEX Ephemeris,
    ///         // you are expected to provide it externally.
    ///         // If you don't know how to do this, simply insert zeros like this
    ///         alm_4: Default::default(),
    ///         
    ///         // TLM to be associated to ALM-5
    ///         tlm_5: GpsQzssTelemetry {
    ///             message: 5,
    ///             integrity: false, // whatever you want
    ///             reserved_bit: false, // whatever you want
    ///         },
    ///
    ///         // ALM-5 cannot be deduced from RINEX Ephemeris,
    ///         // you are expected to provide it externally.
    ///         // If you don't know how to do this, simply insert zeros like this
    ///         alm_5: Default::default(),
    ///
    ///     };
    ///
    ///     let msg = ephemeris
    ///         .to_gps_qzss_message(opts)
    ///         .unwrap();
    ///
    ///     // Verifies correctness in ToW progression (always true).
    ///     assert_eq!(
    ///         msg.eph_2().how.tow - msg.eph_1().how.tow,
    ///         GPS_SUBFRAME_DURATION_S,
    ///     );
    ///     assert_eq!(
    ///         msg.eph_3().how.tow - msg.eph_2().how.tow,
    ///         GPS_SUBFRAME_DURATION_S,
    ///     );
    ///     assert_eq!(
    ///         msg.alm_4().how.tow - msg.eph_3().how.tow,
    ///         GPS_SUBFRAME_DURATION_S,
    ///     );
    ///     assert_eq!(
    ///         msg.alm_5().how.tow - msg.alm_4().how.tow,
    ///         GPS_SUBFRAME_DURATION_S,
    ///     );
    ///
    ///     // reciprocal (RINEX [Ephemeris] frame, and its ToC)
    ///     let (toc, decoded) = Ephemeris::from_gps_qzss_message(msg)
    ///         .unwrap();
    ///
    ///     assert_eq!(toc, key.epoch);
    ///
    ///     // reconstructed ephemeris is "bit true" at the GPS/QZSS
    ///     // binary precision level, which is lower than f64 assert_eq.
    ///     // Our test toolkit verifies just that, but this macro
    ///     // is not avaible here. assert_eq! would "fail" at 1e-7 range.
    /// }
    /// ```
    pub fn to_gps_qzss_message(
        &self,
        opts: GpsQzssCompositionOpts,
    ) -> Result<GpsQzss30sMessage, Error> {
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

        let eph_1 = GpsQzssFrame {
            how: how(1, GpsQzssFrameId::Ephemeris1),
            telemetry: opts.tlm_1,
            subframe: GpsQzssSubframe::Ephemeris1(subframe1),
        };

        let eph_2 = GpsQzssFrame {
            how: how(2, GpsQzssFrameId::Ephemeris2),
            telemetry: opts.tlm_2,
            subframe: GpsQzssSubframe::Ephemeris2(subframe2),
        };

        let eph_3 = GpsQzssFrame {
            how: how(3, GpsQzssFrameId::Ephemeris3),
            telemetry: opts.tlm_3,
            subframe: GpsQzssSubframe::Ephemeris3(subframe3),
        };

        let alm_4 = match opts.alm_4 {
            Some(subf4) => {
                GpsQzssFrame {
                    how: how(4, GpsQzssFrameId::Almanach4),
                    telemetry: opts.tlm_4,
                    subframe: GpsQzssSubframe::Almanach4(subf4),
                }
            },
            None => {
                // not provided by user: insert zeros
                GpsQzssFrame {
                    how: how(4, GpsQzssFrameId::Almanach4),
                    telemetry: opts.tlm_4,
                    subframe: GpsQzssSubframe::Almanach4(Default::default()),
                }
            },
        };

        let alm_5 = match opts.alm_5 {
            Some(subf5) => {
                GpsQzssFrame {
                    how: how(5, GpsQzssFrameId::Almanach5),
                    telemetry: opts.tlm_5,
                    subframe: GpsQzssSubframe::Almanach5(subf5),
               }
            },
            None => {
                // not provided by user: insert zeros
                GpsQzssFrame {
                    how: how(5, GpsQzssFrameId::Almanach5),
                    telemetry: opts.tlm_5,
                    subframe: GpsQzssSubframe::Almanach5(Default::default()),
               }
            }
        };

        Ok(GpsQzss30sMessage {
            satellite: opts.satellite,
            epoch: opts.epoch,
            frames: [eph_1, eph_2, eph_3, alm_4, alm_5],
        })
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
