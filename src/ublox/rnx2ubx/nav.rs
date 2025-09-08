use crate::{
    navigation::{Ephemeris, NavKey},
    prelude::{Constellation, Rinex},
};

use ublox::PacketRef;

/// NAV Record Streamer
pub struct Streamer<'a> {
    ephemeris_iter: Box<dyn Iterator<Item = (&'a NavKey, &'a Ephemeris)> + 'a>,
}

impl<'a> Streamer<'a> {
    pub fn new(rinex: &'a Rinex) -> Self {
        Self {
            ephemeris_iter: rinex.nav_ephemeris_frames_iter(),
        }
    }
}

impl<'a> Iterator for Streamer<'a> {
    type Item = PacketRef<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let (key, ephemeris) = self.ephemeris_iter.next()?;

        match key.sv.constellation {
            Constellation::GPS => {
                let _ = ephemeris.to_ubx_mga_gps_qzss(key.epoch, key.sv)?;
                None // TODO: UBX encapsulation
            },
            Constellation::QZSS => {
                let _ = ephemeris.to_ubx_mga_gps_qzss(key.epoch, key.sv)?;
                None // TODO: UBX encapsulation
            },
            Constellation::BeiDou => {
                let _ = ephemeris.to_ubx_mga_bds(key.epoch, key.sv)?;
                None // TODO: UBX encapsulation
            },
            Constellation::Glonass => {
                let _ = ephemeris.to_ubx_mga_glo(key.sv)?;
                None // TODO: UBX encapsulation
            },
            _ => None,
        }
    }
}
