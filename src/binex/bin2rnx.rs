//! BINEX to RINEX deserialization
use std::io::Read;

use crate::{
    prelude::{Duration, Epoch, Rinex},
    production::{Postponing, SnapshotMode},
};

use binex::prelude::{Decoder, EphemerisFrame, Message, Record, SolutionsFrame, StreamElement};

#[cfg(feature = "log")]
use log::{error, info};

/// BIN2RNX is a RINEX producer from a BINEX stream.
/// It can serialize the streamed messages and collect them as RINEX.
/// The production behavior is defined by [SnapshotMode]
pub struct BIN2RNX<'a, R: Read> {
    /// True when collecting is feasible
    pub active: bool,

    /// Collected size, for postponing mechanism
    size: usize,

    /// Snapshot mode
    pub snapshot_mode: SnapshotMode,

    /// Postponing option
    pub postponing: Postponing,

    /// Deploy time
    deploy_t: Epoch,

    /// BINEX [Decoder]
    decoder: Decoder<'a, R>,

    /// Pending NAV [Rinex]
    nav_rinex: Rinex,

    /// Pending OBS [Rinex]
    obs_rinex: Rinex,
}

impl<'a, R: Read> Iterator for BIN2RNX<'a, R> {
    type Item = Option<String>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.decoder.next() {
            Some(Ok(StreamElement::OpenSource(msg))) => {
                if self.active {
                    match msg.record {
                        Record::EphemerisFrame(fr) => {
                            //let nav = self.nav_rinex.record.as_mut_nav().unwrap();
                            match fr {
                                EphemerisFrame::GAL(_) => {},
                                EphemerisFrame::GLO(_) => {},
                                EphemerisFrame::GPS(_) => {},
                                EphemerisFrame::SBAS(_) => {},
                                EphemerisFrame::GPSRaw(_raw) => {},
                            }
                        },
                        Record::MonumentGeo(geo) => for _ in geo.frames.iter() {},
                        Record::Solutions(pvt) => {
                            for fr in pvt.frames.iter() {
                                match fr {
                                    SolutionsFrame::AntennaEcefPosition(_ecef) => {},
                                    SolutionsFrame::AntennaGeoPosition(_geo) => {},
                                    SolutionsFrame::Comment(_comment) => {},
                                    SolutionsFrame::TemporalSolution(_time) => {},
                                    SolutionsFrame::TimeSystem(_time) => {},
                                    SolutionsFrame::AntennaEcefVelocity(_ecef) => {},
                                    SolutionsFrame::AntennaGeoVelocity(_geo) => {},
                                    SolutionsFrame::Extra(_extra) => {},
                                }
                            }
                        },
                    }
                } else {
                    self.postponed(&msg);
                }
            },
            #[cfg(feature = "log")]
            Some(Ok(StreamElement::ClosedSource(msg))) => {
                error!(
                    "received closed source message: cannot interprate {:?}",
                    msg.closed_meta
                )
            },
            #[cfg(not(feature = "log"))]
            Some(Ok(StreamElement::ClosedSource(_))) => {},
            #[cfg(feature = "log")]
            Some(Err(e)) => {
                error!("binex decoding error: {:?}", e);
            },
            #[cfg(not(feature = "log"))]
            Some(Err(_)) => {},
            _ => {},
        }

        None
    }
}

impl<'a, R: Read> BIN2RNX<'a, R> {
    /// Creates a new [BIN2RNX] working from [Read]able interface.
    /// It will stream Tokens as long as the interface is alive.
    ///
    /// NB:
    /// - [BIN2RNX] needs the system time to be determined for the postponing
    /// mechanism. If determination fails, this method will panic.
    /// We propose [Self::new_system_time] if you want to manually
    /// define "now".
    /// - since RINEX is fully open source, only open source BINEX messages
    /// may be picked up and collected: closed source elements are discarded.
    ///
    /// ## Inputs
    /// - crinex: set to true if you want to use the CRINEX compression
    /// algorithm when collecting Observation RINEX.
    /// - production rate control as [SnapshotMode]
    /// - [Postponing] option
    /// - read: [Read]able interface
    pub fn new(crinex: bool, snapshot_mode: SnapshotMode, postponing: Postponing, read: R) -> Self {
        Self::new_system_time(
            crinex,
            Epoch::now().unwrap_or_else(|e| panic!("system time determination failed with {}", e)),
            snapshot_mode,
            postponing,
            read,
        )
    }

    /// Infaillible [BIN2RNX] creation, use this if you have no means to access system time.
    /// Define it yourself with "now". Refer to [Self::new] for more information.
    ///
    /// ## Inputs
    /// - crinex: set to true if you want to use the CRINEX compression
    /// algorithm when collecting Observation RINEX.
    pub fn new_system_time(
        crinex: bool,
        now: Epoch,
        snapshot_mode: SnapshotMode,
        postponing: Postponing,
        read: R,
    ) -> Self {
        Self {
            size: 0,
            postponing,
            snapshot_mode,
            deploy_t: now,
            nav_rinex: Rinex::basic_nav(),
            obs_rinex: if crinex {
                Rinex::basic_crinex()
            } else {
                Rinex::basic_obs()
            },
            decoder: Decoder::new(read),
            active: postponing == Postponing::None,
        }
    }

    /// Creates a new [BIN2RNX] that will collect a [Rinex] once a day at midnight,
    /// with deployment possibly postponed.
    ///
    /// ## Inputs
    /// - crinex: set to true if you want to use the CRINEX compression
    /// algorithm when collecting Observation RINEX.
    /// - [Postponing] option
    /// - read: [Read]able interface
    pub fn new_daily(crinex: bool, postponing: Postponing, read: R) -> Self {
        Self::new(crinex, SnapshotMode::DailyMidnight, postponing, read)
    }

    /// Creates a new [BIN2RNX] that will collect a [Rinex] twice a day at midnight and noon,
    /// with deployment possibly postponed.
    ///
    /// ## Inputs
    /// - crinex: set to true if you want to use the CRINEX compression
    /// algorithm when collecting Observation RINEX.
    /// - [Postponing] option
    /// - read: [Read]able interface
    pub fn new_midnight_noon(crinex: bool, postponing: Postponing, read: R) -> Self {
        Self::new(crinex, SnapshotMode::DailyMidnightNoon, postponing, read)
    }

    /// Creates a new [BIN2RNX] that will collect a [Rinex] hourly
    /// with deployment possibly postponed.
    ///
    /// ## Inputs
    /// - crinex: set to true if you want to use the CRINEX compression
    /// algorithm when collecting Observation RINEX.
    /// - [Postponing] option
    /// - read: [Read]able interface
    pub fn new_hourly(crinex: bool, postponing: Postponing, read: R) -> Self {
        Self::new(crinex, SnapshotMode::Hourly, postponing, read)
    }

    /// Creates a new [BIN2RNX] that will collect a [Rinex] periodically,
    /// with deployment possibly postponed.
    /// ## Inputs
    /// - crinex: set to true if you want to use the CRINEX compression
    /// algorithm when collecting Observation RINEX.
    /// - period: production period, as [Duration]
    /// - [Postponing] option
    /// - read: [Read]able interface
    pub fn new_periodic(crinex: bool, period: Duration, postponing: Postponing, read: R) -> Self {
        Self::new(crinex, SnapshotMode::Periodic(period), postponing, read)
    }

    fn postponed(&mut self, msg: &Message) {
        match self.postponing {
            Postponing::SystemTime(t) => self.system_time_postponing(t),
            Postponing::Size(size) => self.bytewise_postponing(msg.encoding_size(), size),
            Postponing::Messages(size) => self.protocol_postponing(size),
            _ => unreachable!("no postponing!"),
        }
    }

    /// Holds production until system time as reached specific instant
    fn system_time_postponing(&mut self, t: Epoch) {
        let now =
            Epoch::now().unwrap_or_else(|e| panic!("system time determination failure: {}", e));

        if now > t {
            // todo log message
            self.active = true;
            self.deploy_t = now;
        }
    }

    /// Collect "size" bytes until production is allowed
    fn bytewise_postponing(&mut self, msg_size: usize, size: usize) {
        self.size += msg_size;
        if self.size >= size {
            #[cfg(feature = "log")]
            info!("bin2rnx now deployed: production is pending");
            let now =
                Epoch::now().unwrap_or_else(|e| panic!("system time determination failure: {}", e));
            self.active = true;
            self.deploy_t = now;
        } else {
            #[cfg(feature = "log")]
            info!("binex postponing..");
        }
    }

    /// Collect "size" messages until production is allowed
    fn protocol_postponing(&mut self, size: usize) {
        match self.decoder.next() {
            Some(Ok(StreamElement::OpenSource(_))) => {
                self.size += 1;
                #[cfg(feature = "log")]
                info!("binex postponing {}/{} messages", self.size, size);
            },
            #[cfg(feature = "log")]
            Some(Ok(StreamElement::ClosedSource(msg))) => {
                error!(
                    "received closed source message: cannot interprate {:?}",
                    msg.closed_meta
                )
            },
            #[cfg(not(feature = "log"))]
            Some(Ok(StreamElement::ClosedSource(_))) => {},
            #[cfg(feature = "log")]
            Some(Err(e)) => {
                error!("binex decoding error: {:?}", e);
            },
            #[cfg(not(feature = "log"))]
            Some(Err(_)) => {},
            _ => {},
        }
        if self.size >= size {
            let now =
                Epoch::now().unwrap_or_else(|e| panic!("system time determination failure: {}", e));
            self.active = true;
            self.deploy_t = now;
            #[cfg(feature = "log")]
            info!("bin2rnx now deployed: production is pending");
        }
    }

    /// Obtain reference to collected Observation RINEX
    pub fn obs_rinex(&self) -> &Rinex {
        &self.obs_rinex
    }

    /// Obtain reference to collected Navigation RINEX
    pub fn nav_rinex(&self) -> &Rinex {
        &self.nav_rinex
    }
}
