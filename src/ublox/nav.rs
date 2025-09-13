use crate::{
    navigation::{Ephemeris, NavKey},
    prelude::{Constellation, Rinex},
};

use std::io::{Error, ErrorKind};

pub struct Streamer<'a> {
    /// Pending bytes
    pending_size: usize,

    /// Pending frame
    buffer: [u8; 1024],

    /// Iterator
    ephemeris_iter: Box<dyn Iterator<Item = (&'a NavKey, &'a Ephemeris)> + 'a>,
}

impl<'a> Streamer<'a> {
    pub fn new(rinex: &'a Rinex) -> Self {
        Self {
            pending_size: 0,
            buffer: [0; 1024],
            ephemeris_iter: rinex.nav_ephemeris_frames_iter(),
        }
    }
}

impl<'a> std::io::Read for Streamer<'a> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let mut size = 0;
        let mut size_avail = buffer.len();

        if self.pending_size > 0 {
            if size_avail < self.pending_size {
                return Err(Error::new(ErrorKind::StorageFull, "would not fit"));
            } else {
                size += self.pending_size;
                size_avail -= self.pending_size;
                buffer[..self.pending_size].copy_from_slice(&self.buffer[..self.pending_size]);
                self.pending_size = 0;
            }
        }

        loop {
            match self.ephemeris_iter.next() {
                Some((key, ephemeris)) => match key.sv.constellation {
                    Constellation::GPS => {
                        if let Some(bytes) = ephemeris.to_ubx_mga_gps_qzss(key.epoch, key.sv) {
                            let new_len = bytes.len();

                            if size_avail > new_len {
                                buffer[size..size + new_len].copy_from_slice(&bytes);
                                size += new_len;
                                size_avail -= new_len;
                            } else {
                                self.pending_size = new_len;
                                self.buffer[..new_len].copy_from_slice(&bytes);
                                return Ok(size);
                            }
                        }
                    },
                    Constellation::QZSS => {
                        if let Some(bytes) = ephemeris.to_ubx_mga_gps_qzss(key.epoch, key.sv) {
                            let new_len = bytes.len();

                            if size_avail > new_len {
                                buffer[size..size + new_len].copy_from_slice(&bytes);
                                size += new_len;
                                size_avail -= new_len;
                            } else {
                                self.pending_size = new_len;
                                self.buffer[..new_len].copy_from_slice(&bytes);
                                return Ok(size);
                            }
                        }
                    },
                    Constellation::BeiDou => {
                        if let Some(bytes) = ephemeris.to_ubx_mga_bds(key.epoch, key.sv) {
                            let new_len = bytes.len();

                            if size_avail > new_len {
                                buffer[size..size + new_len].copy_from_slice(&bytes);
                                size += new_len;
                                size_avail -= new_len;
                            } else {
                                self.pending_size = new_len;
                                self.buffer[..new_len].copy_from_slice(&bytes);
                                return Ok(size);
                            }
                        }
                    },
                    Constellation::Glonass => {
                        if let Some(bytes) = ephemeris.to_ubx_mga_glo(key.sv) {
                            let new_len = bytes.len();

                            if size_avail > new_len {
                                buffer[size..size + new_len].copy_from_slice(&bytes);
                                size += new_len;
                                size_avail -= new_len;
                            } else {
                                self.pending_size = new_len;
                                self.buffer[..new_len].copy_from_slice(&bytes);
                                return Ok(size);
                            }
                        }
                    },
                    _ => {
                        // frame not supported
                    },
                },
                None => {
                    return Ok(size);
                },
            }
        }
    }
}
