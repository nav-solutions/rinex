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
use log::{debug, error};

impl Rinex {
    /// Reconstructs a [GpsQzss30Message] burst at desired point in time
    /// of the provided temporal arc.
    ///
    /// When reconstructing the burst, the first 18s composed of EPH-1, 2 and 3
    /// can be fully reconstructed. ALM-4 and ALM-5 depends where we are in the
    /// 12'30" GPS alamanch period, and depending on the information that needs
    /// to be reconstructed, it is not 100% feasible and this method will zero pad.
    ///
    /// For each burst we are about to synthesize, we give the user options to
    /// customize and compose the burst accurately. For example, you can
    /// provide ALM-4 and 5 yourself and they will be used.
    /// We are also able to deduce Satellite Almanach from the Ephemeris frame,
    /// so several pages will not be fully zero.
    /// Page 18 describes the GPST offset to UTC timescale, which is also
    /// available in RINEX and can be fully reconstructed.
    ///
    /// For each [GpsQzssFrame] being reconstructed, you have access to every single
    /// bit, which allows advanced users to compose very precise bursts.
    ///
    /// If you intend to forge a realistic signal, you should define a temporal
    /// arc which is aligned with GPS periods.
    /// 
    /// If you intend to simulate a GPS/QZSS signal you still have to CDMA modulate
    /// the output burst.
    ///
    /// ## Input
    ///
    /// - Any valid NAV [Rinex] file
    ///
    /// - temporal arc, as [TimeSeries] which should be aligned with GPS period
    /// for realistic synthesis
    ///
    /// ## Output
    ///
    /// ```
    /// use rinex::{
    ///     prelude::Rinex,
    ///     navigation::{
    ///         GpsQzssEphCompositionOpts,
    ///     },
    /// };
    ///
    /// use hifitime::prelude::{
    ///     TimeSeries,
    ///     Unit,
    /// };
    ///
    /// use gnss_protos::gps::GPS_SUBFRAME_DURATION_S;
    ///  
    /// // load GPS (only, v2)
    /// let rinex = Rinex::from_gzip_file("data/NAV/V2/cbw10010.21n.gz")
    ///     .unwrap();
    ///
    /// // in this simple example, we will focus on G15 and G24
    /// // and compose slightly different bits for each of them
    /// let (mut g15_alert, g15_as) = (false, true);
    /// let (mut others_alert, others_as) = (true, false);
    ///
    /// // First point does not have to be aligned with the file,
    /// // we can compose a burst for every instant where a valid ephemeris
    /// // can be selected
    /// let t0 = rinex.first_epoch() - 2.0 * Duration::seconds(60);
    ///
    /// // Arc duration: 2 minuts, we get one subframe every 6s,
    /// // avoids generating too many points.
    /// let t0 = rinex.first_epoch() - 2.0 * Duration::seconds(60);
    /// 
    /// // This arc is therefore not aligned with GPS period,
    /// // synthesized signal is not 100% realistic.
    /// // It still uses the correct +6s interval between each subframes,
    /// // which is correct.
    /// let temporal_arc = TimeSeries::inclusive(
    ///     t0,
    ///     t0 + 120.0 * Unit::Second,
    ///     6.0 * Unit::Second,
    ///
    /// );
    ///
    /// // Attempt to reconstruct a 30s burst at every instant of this arc
    /// for message_30s in rinex.to_gps_qzss_messages(
    ///     temporal_arc,
    ///     |(satellite, t_tx)| {
    ///         // for any burst we are about to synthesize,
    ///         // you can compose and customize every single bits,
    ///         // which is particularly useful for reserved bits.
    ///         let opts = GpsQzssCompositionOpts {
    ///             satellite,
    ///             epoch: t_tx,
    ///             alert: {
    ///                 if satellite == g15 {
    ///                     g15_alert // simple pattern
    ///                 } else {
    ///                     others_alert // simple pattern 
    ///                 }
    ///             },
    ///             anti_spoofing: {
    ///                 if satellite == g15 {
    ///                     g15_as // simple pattern
    ///                 } else {
    ///                     others_as // simple pattern 
    ///                 }
    ///             },
    ///
    ///             // TLM-1 attached to EPH-1
    ///             tlm_1: Default::default(),
    ///
    ///             // TLM-2 attached to EPH-2
    ///             tlm_2: Default::default(),
    ///
    ///             // TLM-3 attached to EPH-3
    ///             tlm_3: Default::default(),
    /// 
    ///             // TLM-4 attached to ALM-4
    ///             tlm_4: Default::default(),
    ///
    ///             // ALM-4 composition options.
    ///             // You can provide it yourself by doing this.
    ///             // In this simple example, we will insert zeros
    ///             // at ALM-4.
    ///             alm_4: GpsQzssAlm4CompositionOpts::AsIs(Default::default()),
    ///
    ///             // TLM-5 attached to ALM-5
    ///             tlm_5: Default::default(),
    ///
    ///             // ALM-5 composition options.
    ///             // You can have our synthesizer try to reconstruct
    ///             // ALM-5 pages correctly. It will work for some of the
    ///             // bits but not 100% and that depends where
    ///             // we are right now in the GPS period.
    ///             alm_5: None,
    ///         };
    ///     },
    /// ) {
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
    pub fn to_gps_qzss_messages<'a, C>(
        &'a self,
        temporal_arc: TimeSeries,
        mut composer: C,
    ) -> Box<dyn Iterator<Item = Option<GpsQzss30sMessage>> + 'a>
    where
        C: FnMut((SV, Epoch)) -> GpsQzssCompositionOpts + 'a,
    {
        let gps_satellites = self
            .sv_iter()
            .filter(|sv| {
                sv.constellation == Constellation::GPS || sv.constellation == Constellation::QZSS
            })
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
                            #[cfg(feature = "log")] {
                                error!("failed to reconstruct GPS/QZSS burst: {} ({}@{})", _e, sv, t);
                            }
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
