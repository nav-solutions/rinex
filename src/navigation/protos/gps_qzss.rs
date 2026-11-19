//! GPS/QZSS bitstream to RINEX serdes operations

use crate::{
    navigation::{Ephemeris, Record as NavRecord},
    prelude::{Header, Record, Rinex, Version, SV},
    production::ProductionAttributes,
};

use gnss_protos::gps::GpsQzssFrame;

#[derive(Debug, Copy, Clone, Default)]
pub struct GpsQzssFrameItem {
    /// Satellite signal source from which the demodulator works on.
    pub satellite: SV,
}

struct GpsQzssRotationStorage {}

impl Rinex {
    /// Collects a NAV [Rinex] file from multiple continuous [GpsQzssFrameItem] streamers.
    /// This allows you to collect a NAV RINEX from several [GpsQzssFrame] decoders, working in
    /// parallel, one per satellite and bitstream being received.
    ///
    /// The main difference with [Rinex::from_gps_qzss_eph_messages] is that this one processes
    /// the entire continuous [GpsQzssFrame] stream, as naturally streamed over the air by
    /// any sallite, while the latter was focused on the EPH-1,2,3 part of the stream for each
    /// satellite. Using this method, the analysis of the entire message cycle takes ALM4 and ALM5
    /// into account, concluding the entire 30s message cycle. By grabing specific pages of the
    /// ALM4 and ALM5 rotations, we can deduce more information and create a more precise RINEX
    /// file.
    ///
    /// In particular:
    ///
    /// - description of the timescale against UTC timescales. When working with RINEX V4
    /// this is regularly updated during the course of the day. When working with older revisions,
    /// it is only described in the [Header]. In this case, we will use the first description
    /// encountered in the binary stream, this information will be described in the [Header]
    /// and is supposed to apply for 24 hours. This creates biases when post-processing that file.
    ///
    /// - when working with NAV RINEX V4 revision, we can dump STO frames for each ALM4-5 burst,
    /// which allows to describe the evolution of the timescale with other timescales more
    /// precisely.
    ///
    /// - The satellite Almanach is not saved in RINEX, which remains an incomplete file format to
    /// describe Almanachs. Therefore, this information (all those pages) are dropped and
    /// disregarded by this method. One reason being, real-time navigation solely relies on
    /// satellite Ephemeris, and they are also more accurate than satellite Almanach.
    ///
    /// ## Input
    ///
    /// - mutable [Header], which allows you to compose an accurate RINEX, for example
    /// describing the name of the operator and hardware being used for that recording.
    /// Simple use [Header::default] when unsure to create a basic file. The most important
    /// field being the [Version] number, which will define the following behavior and resulting
    /// file quality.
    ///
    /// - possible [ProductionAttributes] which allow you to name your text file
    /// and customise your text file more accurately, which is particularly useful to professional
    /// agencies. Pass [None] here, and we will use our smart guesser, which will provide most of
    /// the information based on actual data content.
    ///
    /// - iterator: sequence of [GpsQzssFrameItem], where you must describe the decoding instant
    /// of the attached frame, and the [SV] satellite signal source on which the correlator
    /// is working on. That means, at some point in your workflow, you have to merge each
    /// individual demodulator into a single stream provided to this method. Which seems doable
    /// and acceptable. This method does not care about the rotation of the message.
    /// It is possible to stream physically incorect data streams.
    /// All this method cares about, is gathering and dump the correct information
    /// everytime it encounters a given part of the radio stream.
    ///
    /// ## Output
    ///
    /// - This method is infaillible. It may create an empty file if the input stream is empty (no
    /// data source) and if you fail to provide at least one EPH-1,2,3 ALM-4,5 rotation.
    /// The Almanach temporal state will be missing if your streamer does not provide enough data.
    /// It takes 12'30s of recording to record the entire constellation Almanach.
    pub fn from_gps_qzss_messages(
        &self,
        mut header: Header,
        attributes: Option<ProductionAttributes>,
        iterator: impl Iterator<Item = GpsQzssFrameItem>,
    ) -> Self {
        let major = header.version.major;

        let mut rinex = match major {
            4 => Self::from_gps_qzss_v4_messages(header, iterator),
            3 => Self::from_gps_qzss_v3_messages(header, iterator),
            _ => Self::from_gps_qzss_v2_messages(header, iterator),
        };

        // final attributes composition conveniently applies to all revisions similarly
        match attributes {
            Some(attributes) => {
                rinex.production = attributes;
            },
            None => {
                rinex.production = rinex.guess_production_attributes();
            },
        }

        rinex
    }

    /// [Self::from_gps_qzss_messages] specific to V4 revision.
    /// Main difference with others, is that we have the V3 accuracy in the header, and we have STO
    /// frames in the record.
    fn from_gps_qzss_v4_messages(
        mut header: Header,
        iterator: impl Iterator<Item = GpsQzssFrameItem>,
    ) -> Self {
        let mut record = NavRecord::default();
        Self {
            header,
            comments: Default::default(),
            production: Default::default(),
            record: Record::NavRecord(record),
        }
    }

    /// [Self::from_gps_qzss_messages] specific to V3 revision.
    /// Main difference with others, is that we have the V3 accuracy in the header, and we don't
    /// have STO frames in the record.
    fn from_gps_qzss_v3_messages(
        mut header: Header,
        iterator: impl Iterator<Item = GpsQzssFrameItem>,
    ) -> Self {
        let mut record = NavRecord::default();
        Self {
            header,
            comments: Default::default(),
            production: Default::default(),
            record: Record::NavRecord(record),
        }
    }

    /// [Self::from_gps_qzss_messages] specific to V2 revision.
    /// Main difference with others, is that we don't have the accuracy of V3 and V4 revisions in
    /// the header, and the STO frames are also impossible to encode.
    fn from_gps_qzss_v2_messages(
        mut header: Header,
        iterator: impl Iterator<Item = GpsQzssFrameItem>,
    ) -> Self {
        let mut record = NavRecord::default();
        let mut record = NavRecord::default();
        Self {
            header,
            comments: Default::default(),
            production: Default::default(),
            record: Record::NavRecord(record),
        }
    }
}
