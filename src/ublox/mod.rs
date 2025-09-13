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
    /// Unlike other streamers (RTCM, BINEX..), the UBX streamer can only operate on a buffer.
    /// Conveniently, [RNX2UBX] implements [Read] and [BufRead] to let you stream all the supported messages
    /// into your own buffer.
    ///
    /// The stream content is RINEX dependent, and we currently only truly support NAV RINEX.
    ///
    /// RINEX NAV (V3) example:
    /// ```
    /// use std::io::Read;
    /// use rinex::prelude::Rinex;
    ///
    /// // allocate
    /// let mut buffer = [0; 1024];
    ///
    /// // NAV(V3) files will generate MGA-EPH frames
    /// // and potentally one MGA-IONO per model.
    /// let rinex = Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    ///     .unwrap();
    ///
    /// // deploy
    /// let mut streamer = rinex.rnx2ubx();
    ///
    /// // consume entirely
    /// loop {
    ///     match streamer.read(&mut buffer) {
    ///         Ok(0) => {
    ///             // end of stream
    ///             break;
    ///         },
    ///         Ok(size) => {
    ///             // example: decode all forwarded packets
    ///             // "size" is the total number of bytes, not the number of UBX frames
    ///             // note that we only encode complete frames, not partial frames.
    ///         },
    ///         Err(e) => {
    ///             // we wind up here on buffering or system errors.
    ///             // For example, the complete frame would not fit in the buffer.
    ///             break;
    ///         },
    ///     }
    /// }
    /// ```
    pub fn rnx2ubx<'a>(&'a self) -> RNX2UBX<'a> {
        RNX2UBX {
            streamer: TypeDependentStreamer::new(self),
        }
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
