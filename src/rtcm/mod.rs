use crate::prelude::{Rinex, RinexType};

mod nav;
use nav::Streamer as NavStreamer;

use rtcm_rs::msg::message::Message;

/// RINEX type dependent record streamer
enum TypeDependentStreamer<'a> {
    /// NAV frames streamer
    NAV(NavStreamer<'a>),
}

/// [RNX2UBX] can serialize a [Rinex] structure as a stream of UBX frames.
/// It implements [Read] which lets you stream data bytes into your own buffer.
pub struct RNX2RTCM<'a> {
    type_dependent: TypeDependentStreamer<'a>,
}

impl<'a> Iterator for RNX2RTCM<'a> {
    type Item = Message;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.type_dependent {
            TypeDependentStreamer::NAV(streamer) => streamer.next(),
        }
    }
}

impl Rinex {
    /// Obtain a [RNX2RTCM] streamer to serialize this [Rinex] structure into a stream of RTCM [Message]s.
    /// You can then use the Iterator implementation to iterate each messages.
    ///
    /// RINEX NAV (V3) example:
    /// ```
    /// use std::io::Read;
    /// use rinex::prelude::Rinex;
    ///
    /// // NAV(V3) files will generate
    /// let rinex = Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    ///     .unwrap();
    ///
    /// // deploy
    /// let mut streamer = rinex.rnx2rtcm()
    ///     .unwrap(); // supported for this type
    ///
    /// // consume entirely
    /// loop {
    ///     match streamer.next() {
    ///         Some(message) => {
    ///             // TODO
    ///         },
    ///         None => {
    ///             // end of stream
    ///             // RINEX file has been consumed entirely
    ///             break;
    ///         },
    ///     }
    /// }
    pub fn rnx2rtcm<'a>(rinex: &'a Rinex) -> Option<RNX2RTCM<'a>> {
        let type_dependent = match rinex.header.rinex_type {
            RinexType::NavigationData => TypeDependentStreamer::NAV(NavStreamer::new(rinex)),
            _ => {
                return None;
            },
        };

        Some(RNX2RTCM { type_dependent })
    }
}
