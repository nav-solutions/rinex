//! GPS/QZSS bitstream to RINEX serdes operations

use crate::{
    navigation::{
        ephemeris::protos::gps_qzss::Error, Ephemeris, GpsQzss30sMessage, GpsQzssCompositionOpts,
        Record as NavRecord,
    },
    prelude::{Constellation, Header, Record, Rinex, Version, SV},
    production::ProductionAttributes,
};

use itertools::Itertools;

use hifitime::prelude::{Epoch, TimeSeries};

use gnss_protos::gps::GpsQzssFrame;

#[cfg(feature = "log")]
use log::debug;

impl Rinex {
    /// Collects a NAV [Rinex] file from multiple continuous [GpsQzssFrameItem] streamers.
    /// This allows you to collect a NAV RINEX from several [GpsQzssFrame] decoders, working in
    /// parallel, one per satellite and bitstream being received.
    ///
    /// The main difference with [Rinex::from_gps_qzss_eph_messages] is that this one processes
    /// the entire continuous [GpsQzssFrame] stream, as naturally streamed over the air by
    /// any sallite, while the latter was focused on the EPH-1,2,3 part of the stream for each
    /// satellite. Using this method, the analysis of the entire message cycle takes ALM4 and ALM5
    /// into account, concluding the entire 30s message cycle. By grabing specific pages of the
    /// ALM4 and ALM5 rotations, we can deduce more information and create a more precise RINEX
    /// file.
    ///
    /// In particular:
    ///
    /// - description of the timescale against UTC timescales. When working with RINEX V4
    /// this is regularly updated during the course of the day. When working with older revisions,
    /// it is only described in the [Header]. In this case, we will use the first description
    /// encountered in the binary stream, this information will be described in the [Header]
    /// and is supposed to apply for 24 hours. This creates biases when post-processing that file.
    ///
    /// - when working with NAV RINEX V4 revision, we can dump STO frames for each ALM4-5 burst,
    /// which allows to describe the evolution of the timescale with other timescales more
    /// precisely.
    ///
    /// - The satellite Almanach is not saved in RINEX, which remains an incomplete file format to
    /// describe Almanachs. Therefore, this information (all those pages) are dropped and
    /// disregarded by this method. One reason being, real-time navigation solely relies on
    /// satellite Ephemeris, and they are also more accurate than satellite Almanach.
    ///
    /// ## Input
    ///
    /// - mutable [Header], which allows you to compose an accurate RINEX, for example
    /// describing the name of the operator and hardware being used for that recording.
    /// Simple use [Header::default] when unsure to create a basic file. The most important
    /// field being the [Version] number, which will define the following behavior and resulting
    /// file quality.
    ///
    /// - possible [ProductionAttributes] which allow you to name your text file
    /// and customise your text file more accurately, which is particularly useful to professional
    /// agencies. Pass [None] here, and we will use our smart guesser, which will provide most of
    /// the information based on actual data content.
    ///
    /// - iterator: sequence of [GpsQzssFrameItem], where you must describe the decoding instant
    /// of the attached frame, and the [SV] satellite signal source on which the correlator
    /// is working on. That means, at some point in your workflow, you have to merge each
    /// individual demodulator into a single stream provided to this method. Which seems doable
    /// and acceptable. This method does not care about the rotation of the message.
    /// It is possible to stream physically incorect data streams.
    /// All this method cares about, is gathering and dump the correct information
    /// everytime it encounters a given part of the radio stream.
    ///
    /// ## Output
    ///
    /// - This method is infaillible. It may create an empty file if the input stream is empty (no
    /// data source) and if you fail to provide at least one EPH-1,2,3 ALM-4,5 rotation.
    /// The Almanach temporal state will be missing if your streamer does not provide enough data.
    /// It takes 12'30s of recording to record the entire constellation Almanach.
    /// Reconstructs a 30s GPS radio bursts at every point
    /// in time of the requested temporal arc, using the best suited
    /// Ephemeris frame for each GPS satellite. This burst represents 30s of bit stream
    /// intended to be modulated at the GPS bit rate.
    ///
    /// To select the best suited ephemeris frame, proper ephemeris frame is elected.
    ///
    /// The temporal arc should describe each instants you intended to transmit a burst.
    /// It should follow the GPS period if you intend to forge a realistic signal.
    /// This method does not verify that, the ephemeris election applies at other points
    /// in time, so it is possible to forge "physically" "incorrect" bit streams using this method.
    ///
    /// Any time a correct ephemeris frame has been elected, we can forge 18s of radio signal.
    /// For each burst, we let the user compose each bits of each part of the burst.
    /// Which allows advanced users to precisely control the bit stream being synthesized.
    /// If you tolerate zeros in your bitstream, you can compose using the [Default] trait.
    ///
    /// by concluding each burst with ALM-4 and ALM-5, either user defined, or reconstructed
    /// from the actual RINEX data the best we can.
    ///
    /// For every point in the temporal arc, this method searches for the best suited
    /// Ephemeris frame for each available satellites applying the correct ephemeris
    /// election method. From that ephemeris frame:
    ///
    /// - the first 18s of the burst, made of EPH-1, 2 and 3 will always be correct.
    /// - the last 12s, ALM-4 and ALM-5 either have to be provided or can be partially
    /// reconstructed from the record, depending where we are in the GPS period.
    ///
    /// For each burst we are about to forge, we allow the user to compose all reserved
    /// bits and telemetry fields, which allows to produce very accurate bitstreams.
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
    /// // load GPS (only, v2)
    /// let rinex = Rinex::from_gzip_file("data/NAV/V2/cbw10010.21n.gz")
    ///     .unwrap();
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
    #[cfg(feature = "nav")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nav")))]
    pub fn to_gps_messages<'a, C>(
        &'a self,
        temporal_arc: TimeSeries,
        mut composer: C,
    ) -> Box<dyn Iterator<Item = Option<GpsQzss30sMessage>> + 'a>
    where
        C: FnMut((SV, Epoch)) -> GpsQzssCompositionOpts + 'a,
    {
        let gps_satellites = self
            .sv_iter()
            .filter(|sv| sv.constellation == Constellation::GPS)
            .collect::<Vec<_>>();

        Box::new(
            temporal_arc
                .into_iter()
                .cartesian_product(gps_satellites.into_iter())
                .map(move |(t, sv)| {
                    let (toc, toe, eph) = self.nav_ephemeris_selection(sv, t)?;

                    #[cfg(feature = "log")]
                    {
                        debug!("{} ({}) - selected ephemeris: {}", t, sv, toc);
                    }

                    let opts = composer((sv, t));

                    match eph.to_gps_qzss_message(opts) {
                        Ok(msg) => Some(msg),
                        Err(_e) => {
                            //                     #[cfg(feature = "log")] {
                            //                         error!("failed to convert to GPS/QZSS EPH1-2-3 message: {} ({}@{})", _e, key.sv, key.epoch);
                            //                     }
                            None
                        },
                    }
                }),
        )
    }

    /// Creates a complete Navigation RINEX from a stream of [GpsQzss30sMessage]s.
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
    pub fn from_gps_qzss_messages(
        &self,
        mut header: Header,
        attributes: Option<ProductionAttributes>,
        iterator: impl Iterator<Item = GpsQzss30sMessage>,
    ) -> Result<Self, Error> {
        let major = header.version.major;

        let mut record = NavRecord::default();

        // for item in iterator {
        //     match item.satellite.constellation {
        //         Constellation::GPS | Constellation::QZSS => {
        //             // proceed
        //             match Ephemeris::from_gps_qzss_eph_message(item) {
        //                 Ok((epoch, ephemeris)) => {
        //                     let key = NavKey {
        //                         epoch: item.epoch,
        //                         sv: item.satellite,
        //                         msgtype: NavMessageType::LNAV,
        //                         frmtype: NavFrameType::Ephemeris,
        //                         subtype: None,              // not applicable here
        //                         galileo_data_sources: None, // not applicable here
        //                         sto_identity: None,         // not applicable here
        //                         transmission_time: None,    // TODO
        //                     };
        //                     record.insert(key, NavFrame::EPH(ephemeris));
        //                 },
        //                 Err(_e) => {
        //                     // #[cfg(feature = "log")]
        //                     // error!("failed to decode ephemeris from {:?} ({}@{}): {}", item.eph123, item.satellite, item.epoch.clone(), _e);
        //                 },
        //             }
        //         },
        //         _ => {}, // drop: invalid API usage
        //     }
        // }

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
