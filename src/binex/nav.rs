use crate::{
    navigation::{Ephemeris, NavKey},
    prelude::Rinex,
};

use binex::prelude::{Message, Meta, Record};

/// NAV Record Streamer
pub struct Streamer<'a> {
    meta: Meta,
    ephemeris_iter: Box<dyn Iterator<Item = (&'a NavKey, &'a Ephemeris)> + 'a>,
}

impl<'a> Streamer<'a> {
    pub fn new(meta: Meta, rinex: &'a Rinex) -> Self {
        Self {
            meta: meta,
            ephemeris_iter: rinex.nav_ephemeris_frames_iter(),
        }
    }
}

impl<'a> Iterator for Streamer<'a> {
    type Item = Message;
    fn next(&mut self) -> Option<Self::Item> {
        let (key, eph) = self.ephemeris_iter.next()?;
        let frame = eph.to_binex(key.epoch, key.sv)?;

        Some(Message {
            meta: self.meta,
            record: Record::new_ephemeris_frame(frame),
        })
    }
}
