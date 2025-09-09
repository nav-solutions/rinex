use crate::prelude::Rinex;

mod nav;
use nav::Streamer as NavStreamer;

use ublox::PacketRef;

#[cfg(doc)]
use ublox::Parser;

/// RINEX Type dependant record streamer
enum TypeDependentStreamer<'a> {
    /// NAV frames streamer
    NAV(NavStreamer<'a>),
}

impl<'a> TypeDependentStreamer<'a> {
    pub fn new(rinex: &'a Rinex) -> Self {
        // Only one format supported currently
        Self::NAV(NavStreamer::new(rinex))
    }
}

impl Rinex {
    /// Obtain a [RNX2UBX] streamer to serialize this [Rinex] into a stream of U-Blox [PacketRef]s.
    /// You can then use the Iterator implementation to iterate each messages.
    /// The stream is RINEX format dependent, and we currently only truly support NAV RINEX.
    pub fn rnx2ubx<'a>(&'a self) -> Option<RNX2UBX<'a>> {
        Some(RNX2UBX {
            streamer: TypeDependentStreamer::new(self),
        })
    }
}

/// [RNX2UBX] can serialize a [Rinex] structure as a stream of UBX frames.
/// It implements [Read] which lets you stream data bytes into your own buffer.
pub struct RNX2UBX<'a> {
    /// [TypeDependentStreamer]
    streamer: TypeDependentStreamer<'a>,
}

impl<'a> std::io::Read for RNX2UBX<'a> {
    /// Fills proposed mutable buffer with as many complete UBX frames as possible.
    ///
    /// ## Inputs
    /// - buffer: mutable user buffer to which we write the UBX bytes.
    /// You can then transmit them or decode them using the UBX [Parser].
    /// We will not encode partial frame, but will postpone the pending frame
    /// that will not fit into a successive read that will need to be invoked later on.
    /// As per stardards, we return Ok(0) once the [Rinex] file has been fully consumed.
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.streamer {
            TypeDependentStreamer::NAV(ref mut streamer) => streamer.read(buffer),
        }
    }
}
