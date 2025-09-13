//! RINEX to BINEX serialization
use crate::prelude::{Epoch, Header, Rinex};
use binex::prelude::{Message, Meta, MonumentGeoMetadata, MonumentGeoRecord};

mod nav;
use nav::Streamer as NavStreamer;

/// RINEX Type dependant record streamer
enum TypeDependentStreamer<'a> {
    /// NAV Record streamer
    Nav(NavStreamer<'a>),
}

impl<'a> TypeDependentStreamer<'a> {
    pub fn new(meta: Meta, rinex: &'a Rinex) -> Self {
        Self::Nav(NavStreamer::new(meta, rinex))
    }
}

impl<'a> Iterator for TypeDependentStreamer<'a> {
    type Item = Message;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Nav(streamer) => streamer.next(),
        }
    }
}

/// RNX2BIN can serialize a [Rinex] into a stream of BINEX [Message]s
pub struct RNX2BIN<'a> {
    /// First [Epoch] or [Epoch] of publication
    t0: Epoch,

    /// BINEX [Message] encoding [Meta]
    meta: Meta,

    /// Header consumption State machine
    state: State,

    /// RINEX [Header] snapshot
    header: &'a Header,

    /// RINEX [TypeDependentStreamer]
    streamer: TypeDependentStreamer<'a>,

    /// Assert (before deployment) whether the Header should not be serialized (no default!)
    pub skip_header: bool,

    /// Define (before deployment) a custom message to be included in the announcement.
    pub custom_announce: Option<String>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum State {
    /// Describes the RINEX format, Constellation and revision
    #[default]
    HeaderPkgVersion,
    MonumentGeo,
    AnnounceHeaderComments,
    HeaderComments,
    AnnounceRecord,
    RecordStream,
}

impl<'a> Iterator for RNX2BIN<'a> {
    type Item = Message;
    fn next(&mut self) -> Option<Self::Item> {
        let content = match self.state {
            State::HeaderPkgVersion => {
                let mut geo = self.forge_monument_geo();
                if let Some(custom) = &self.custom_announce {
                    geo.comments.push(custom.clone());
                }
                // announce stream beginning
                geo.comments.push("Stream starting!".to_string());
                self.state = State::MonumentGeo;
                Some(geo)
            },
            State::MonumentGeo => {
                let mut geo = self.forge_monument_geo();
                if let Some(agency) = &self.header.agency {
                    geo = geo.with_agency(agency);
                }
                if let Some(observer) = &self.header.observer {
                    geo = geo.with_observer(observer);
                }
                if let Some(_marker) = &self.header.geodetic_marker {
                    //geo = geo.with_marker_name();
                    //geo = geo.with_marker_number();
                }
                if let Some(rx) = &self.header.rcvr {
                    geo = geo.with_receiver_model(&rx.model);
                    geo = geo.with_receiver_serial_number(&rx.sn);
                    geo = geo.with_receiver_firmware_version(&rx.firmware);
                }
                if let Some(_cospar) = &self.header.cospar {
                    //geo = geo.with_reference_number(cospar);
                }
                if let Some(_position) = &self.header.rx_position {
                    //geo = geo.with_site_location();
                }
                if !self.header.comments.is_empty() && !self.skip_header {
                    self.state = State::AnnounceHeaderComments;
                } else {
                    self.state = State::AnnounceRecord;
                }
                Some(geo)
            },
            State::AnnounceHeaderComments => {
                let mut geo = self.forge_monument_geo();
                geo = geo.with_comment("RINEX Header comments following!");
                self.state = State::HeaderComments;
                Some(geo)
            },
            State::HeaderComments => {
                let mut geo = self.forge_monument_geo();
                for comment in self.header.comments.iter() {
                    geo = geo.with_comment(comment);
                }
                self.state = State::AnnounceRecord;
                Some(geo)
            },
            State::AnnounceRecord => {
                let mut geo = self.forge_monument_geo();
                geo = geo.with_comment("RINEX Record starting!");
                self.state = State::RecordStream;
                Some(geo)
            },
            State::RecordStream => {
                let msg = self.streamer.next()?;
                return Some(msg);
            },
        };

        if let Some(content) = content {
            // forge new message
            Some(Message {
                meta: self.meta,
                record: content.into(),
            })
        } else {
            None
        }
    }
}

impl<'a> RNX2BIN<'a> {
    fn forge_monument_geo(&self) -> MonumentGeoRecord {
        let mut geo = MonumentGeoRecord::default();
        geo.epoch = self.t0;
        geo.meta = MonumentGeoMetadata::RNX2BIN;
        geo = geo.with_software_name(&format!(
            "nav-solutions/rinex v{}",
            env!("CARGO_PKG_VERSION")
        ));
        geo
    }
}

impl Rinex {
    /// Create a [RNX2BIN] streamer to convert this [Rinex]
    /// into a stream of BINEX [Message]s. You can then use the Iterator implementation
    /// to forge the stream.
    /// The stream will be made of
    /// - One geo monument message describing this software package
    /// - One geo monument message announcing the Header fields
    /// - One geo monument message describing all [Header] fields
    /// - One geo monument message wrapping all comments contained in [Header]
    /// - One geo monument message announcing the start of Record stream
    /// - One RINEX format depending by record entry. For example,
    /// one Ephemeris frame per decoded Navigation message.
    ///
    /// ## Inputs:
    /// - meta: BINEX encoding [Meta]
    /// ## Output
    /// - [RNX2BIN]: a BINEX [Message] Iterator
    ///
    /// This is work in progress. Currently, we support
    /// the streaming of Navigation Ephemeris.
    pub fn rnx2bin<'a>(&'a self, meta: Meta) -> Option<RNX2BIN<'a>> {
        let t0 = self.first_epoch()?;
        Some(RNX2BIN {
            t0,
            meta,
            header: &self.header,
            state: State::default(),
            skip_header: false,
            custom_announce: Default::default(),
            streamer: TypeDependentStreamer::new(meta, self),
        })
    }
}
