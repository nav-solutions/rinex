#[cfg(feature = "nav")]
#[cfg_attr(docsrs, doc(cfg(feature = "nav")))]
pub mod feature; // feature dependent, high level methods

#[cfg(all(feature = "nav", feature = "processing"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "nav", feature = "processing"))))]
pub mod time; // feature dependent high level methods

#[cfg(feature = "ut1")]
#[cfg_attr(docsrs, doc(cfg(feature = "ut1")))]
pub mod ut1; // feature dependent high level methods

use crate::{
    navigation::{
        EarthOrientation, Ephemeris, NavFrame, NavFrameType, NavKey, NavMessageType, TimeOffset,
    },
    prelude::{Epoch, Rinex, RinexType, SV},
};

use std::collections::btree_map::Keys;

use super::IonosphereModel;

impl Rinex {
    /// Returns true if this [Rinex] is [RinexType::NavigationData].
    pub fn is_navigation_rinex(&self) -> bool {
        self.header.rinex_type == RinexType::NavigationData
    }

    /// [NavKey]s [Iterator]
    pub fn navigation_keys(&self) -> Keys<'_, NavKey, NavFrame> {
        if let Some(rec) = self.record.as_nav() {
            rec.keys()
        } else {
            panic!("bad rinex type")
        }
    }

    /// [NavMessageType] [Iterator]
    pub fn nav_message_types_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (Epoch, SV, NavMessageType)> + '_> {
        if let Some(rec) = self.record.as_nav() {
            Box::new(rec.iter().map(|(k, _)| (k.epoch, k.sv, k.msgtype)))
        } else {
            Box::new([].into_iter())
        }
    }

    /// [Ephemeris] frames [Iterator]
    pub fn nav_ephemeris_frames_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (&NavKey, &Ephemeris)> + '_> {
        if let Some(rec) = self.record.as_nav() {
            Box::new(rec.iter().filter_map(|(k, v)| {
                if k.frmtype == NavFrameType::Ephemeris {
                    let fr = v.as_ephemeris().unwrap();
                    Some((k, fr))
                } else {
                    None
                }
            }))
        } else {
            Box::new([].into_iter())
        }
    }

    /// [EarthOrientation] frames [Iterator].
    /// This type of frames exists in NAV V4 only. which may only exist
    pub fn nav_earth_orientation_frames_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (&NavKey, &EarthOrientation)> + '_> {
        if let Some(rec) = self.record.as_nav() {
            Box::new(rec.iter().filter_map(|(k, v)| {
                if k.frmtype == NavFrameType::EarthOrientation {
                    let fr = v.as_earth_orientation().unwrap();
                    Some((k, fr))
                } else {
                    None
                }
            }))
        } else {
            Box::new([].into_iter())
        }
    }

    /// [IonosphereModel] frames [Iterator].
    /// This type of frames exists in NAV V4 only.
    pub fn nav_ionosphere_models_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (&NavKey, &IonosphereModel)> + '_> {
        if let Some(rec) = self.record.as_nav() {
            Box::new(rec.iter().filter_map(|(k, v)| {
                if k.frmtype == NavFrameType::IonosphereModel {
                    let fr = v.as_ionosphere_model().unwrap();
                    Some((k, fr))
                } else {
                    None
                }
            }))
        } else {
            Box::new([].into_iter())
        }
    }

    /// [SystemTime] frames [Iterator].
    /// This type of frames exists in NAV V4 only.
    pub fn nav_system_time_frames_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (&NavKey, &TimeOffset)> + '_> {
        if let Some(rec) = self.record.as_nav() {
            Box::new(rec.iter().filter_map(|(k, v)| {
                if k.frmtype == NavFrameType::SystemTimeOffset {
                    let fr = v.as_system_time().unwrap();
                    Some((k, fr))
                } else {
                    None
                }
            }))
        } else {
            Box::new([].into_iter())
        }
    }

    /// [SV] clock state [Iterator].
    /// ## Inputs
    /// - self: Navigation [Rinex]
    /// ## Output
    /// - offset (s), drift (s.s⁻¹), drift rate (s.s⁻²)  triplet iterator
    pub fn nav_sv_clock_iter(&self) -> Box<dyn Iterator<Item = (NavKey, (f64, f64, f64))> + '_> {
        Box::new(
            self.nav_ephemeris_frames_iter()
                .map(|(k, eph)| (*k, eph.sv_clock())),
        )
    }

    // /*
    //  * [IonMessage] Iterator
    //  */
    // fn ionod_correction_models(
    //     &self,
    // ) -> Box<dyn Iterator<Item = (Epoch, (NavMsgType, SV, IonMessage))> + '_> {
    //     /*
    //      * Answers both OLD and MODERN RINEX requirements
    //      * In RINEX2/3, midnight UTC is the publication datetime
    //      */
    //     let t0 = self.first_epoch().unwrap(); // will fail on invalid RINEX
    //     let t0 = Epoch::from_utc_days(t0.to_utc_days().round());
    //     Box::new(
    //         self.header
    //             .ionod_corrections
    //             .iter()
    //             .map(move |(c, ion)| (t0, (NavMsgType::LNAV, SV::new(*c, 1), *ion)))
    //             .chain(self.navigation().flat_map(|(t, frames)| {
    //                 frames.iter().filter_map(move |fr| {
    //                     let (msg, sv, ion) = fr.as_ion()?;
    //                     Some((*t, (msg, sv, *ion)))
    //                 })
    //             })),
    //     )
    // }

    // /// Returns Ionospheric delay compensation, to apply at "t" desired Epoch
    // /// and desired location. NB: we only support Klobuchar models at the moment,
    // /// as we don't know how to convert other models (feel free to contribute).
    // /// "t" must be within a 24 hour time frame of the oldest model.
    // /// When working with RINEX2/3, the model is published at midnight
    // /// and you should expect discontinuities when a new model is being published.
    // pub fn ionod_correction(
    //     &self,
    //     t: Epoch,
    //     sv_elevation: f64,
    //     sv_azimuth: f64,
    //     user_lat_ddeg: f64,
    //     user_lon_ddeg: f64,
    //     carrier: Carrier,
    // ) -> Option<f64> {
    //     // determine nearest in time
    //     let (_, (model_sv, model)) = self
    //         .ionod_correction_models()
    //         .filter_map(|(t_i, (_, sv_i, msg_i))| {
    //             // TODO
    //             // calculations currently limited to KB model: implement others
    //             let _ = msg_i.as_klobuchar()?;
    //             // At most 1 day from publication time
    //             if t_i <= t && (t - t_i) < 24.0 * Unit::Hour {
    //                 Some((t_i, (sv_i, msg_i)))
    //             } else {
    //                 None
    //             }
    //         })
    //         .min_by_key(|(t_i, _)| (t - *t_i))?;

    //     // TODO
    //     // calculations currently limited to KB model: implement others
    //     let kb = model.as_klobuchar().unwrap();
    //     let h_km = match model_sv.constellation {
    //         Constellation::BeiDou => 375.0,
    //         // we only expect BDS or GPS here,
    //         // wrongly formed RINEX will cause innacurate results
    //         Constellation::GPS | _ => 350.0,
    //     };
    //     Some(kb.meters_delay(
    //         t,
    //         sv_elevation,
    //         sv_azimuth,
    //         h_km,
    //         user_lat_ddeg,
    //         user_lon_ddeg,
    //         carrier,
    //     ))
    // }
}
