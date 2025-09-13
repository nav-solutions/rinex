use crate::{
    navigation::{Ephemeris, NavKey},
    prelude::{Constellation, Rinex},
};

use rtcm_rs::msg::message::Message;

pub struct Streamer<'a> {
    /// Iterator
    ephemeris_iter: Box<dyn Iterator<Item = (&'a NavKey, &'a Ephemeris)> + 'a>,
}

impl<'a> Streamer<'a> {
    /// Builds a new [Streamer] dedicated to NAV RINEX streaming.
    pub fn new(rinex: &'a Rinex) -> Self {
        Self {
            ephemeris_iter: rinex.nav_ephemeris_frames_iter(),
        }
    }
}

impl<'a> Iterator for Streamer<'a> {
    type Item = Message;

    /// Try to serialize a new RTCM [Message] from this [Streamer].
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (key, eph) = self.ephemeris_iter.next()?;

            match key.sv.constellation {
                Constellation::GPS => {
                    let msg1019 = eph.to_rtcm_gps_msg1019(key.epoch, key.sv)?;
                    return Some(Message::Msg1019(msg1019));
                },
                Constellation::QZSS => {
                    let msg1044 = eph.to_rtcm_qzss_msg1044(key.epoch, key.sv)?;
                    return Some(Message::Msg1044(msg1044));
                },
                Constellation::Galileo => {
                    // TODO may have 2 forms
                    let msg1045 = eph.to_rtcm_gal_msg1045(key.epoch, key.sv)?;
                    return Some(Message::Msg1045(msg1045));
                },
                Constellation::Glonass => {
                    let msg1020 = eph.to_rtcm_glo_msg1020(key.epoch, key.sv)?;
                    return Some(Message::Msg1020(msg1020));
                },
                Constellation::BeiDou => {
                    let msg1042 = eph.to_rtcm_bds_msg1042(key.epoch, key.sv)?;
                    return Some(Message::Msg1042(msg1042));
                },
                _ => {
                    // Not supported yet
                },
            }
        }
    }
}
