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
        None
    }
}
