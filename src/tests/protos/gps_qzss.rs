use crate::{
    navigation::{
        Ephemeris, GpsQzssEphCompositionOpts, GpsQzssEphemerisItem, GpsQzssEphemerisMessage,
        OrbitItem,
    },
    prelude::{Constellation, Epoch, Rinex, SV},
    tests::toolkit::nav::{assert_faithful_ephemeris, panic_eq_ephemeris},
};

use std::{fs::File, io::Read};

use gnss_protos::{
    GpsQzssDecoder, GpsQzssFrame, GpsQzssFrameId, GpsQzssTelemetry, Message, GPS_FRAME_BYTES,
    GPS_WORDS_PER_FRAME,
};

/// RINEX -> bits (10 words per subframe, parity included) -> RINEX.
/// Returns the transmitted words, the decoded ToC and the decoded [Ephemeris].
fn bitstream_round_trip(eph: &Ephemeris, sv: SV, toc: Epoch) -> (Epoch, Ephemeris) {
    let opts = GpsQzssEphCompositionOpts {
        satellite: sv,
        epoch: toc,
        alert: false,
        anti_spoofing: false,
        tlm_1: GpsQzssTelemetry {
            message: 1,
            integrity: true,
            reserved_bit: false,
        },
        tlm_2: GpsQzssTelemetry {
            message: 2,
            integrity: false,
            reserved_bit: true,
        },
        tlm_3: GpsQzssTelemetry {
            message: 3,
            integrity: true,
            reserved_bit: false,
        },
    };

    let eph123 = eph
        .to_gps_qzss_eph_message(opts)
        .unwrap_or_else(|e| panic!("{} @ {}: encoding failure: {}", sv, toc, e));

    let item = GpsQzssEphemerisItem {
        satellite: sv,
        epoch: toc,
        eph123,
    };

    let (decoded_toc, decoded) = Ephemeris::from_gps_qzss_eph_message(item)
        .unwrap_or_else(|e| panic!("{} @ {}: decoding failure: {}", sv, toc, e));

    (decoded_toc, decoded)
}

/// Runs the round trip on all GPS/QZSS frames of the file
/// and returns the number of tested frames.
fn run_file(path: &str) -> usize {
    let mut count = 0;

    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/" + path;

    let rinex = if path.ends_with(".gz") {
        Rinex::from_gzip_file(&path)
    } else {
        Rinex::from_file(&path)
    }
    .unwrap();

    for (key, eph) in rinex.nav_ephemeris_frames_iter() {
        match key.sv.constellation {
            Constellation::GPS | Constellation::QZSS => {},
            _ => continue,
        }

        let (decoded_toc, decoded_eph) = bitstream_round_trip(eph, key.sv, key.epoch);

        assert_eq!(
            decoded_toc, key.epoch,
            "{} @ {}: ToC reciprocity",
            key.sv, key.epoch
        );

        assert_faithful_ephemeris(key.sv, key.epoch, eph, &decoded_eph);
        count += 1;
    }
    count
}

#[test]
fn test_lnav_v2() {
    let count = run_file("data/NAV/V2/cbw10010.21n.gz");
    assert_eq!(count, 187);
}

#[test]
fn test_lnav_v3() {
    let count = run_file("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz");
    assert_eq!(count, 260);

    let count = run_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz");
    assert_eq!(count, 272);
}

#[test]
fn test_lnav_v4() {
    let count = run_file("data/NAV/V4/KMS300DNK_R_20221591000_01H_MN.rnx.gz");
    assert_eq!(count, 31);
}

#[test]
#[ignore]
fn test_lnav_cnav_v4() {
    let count = run_file("data/NAV/V4/BRD400DLR_S_20241310000_01D_MN.rnx.gz");
    assert_eq!(count, 260);
}

#[test]
fn test_lnav_week_rollover() {
    // Transmitted saturday 22:00 GPST (week 2399),
    // ToC = ToE = sunday 00:00 GPST (week 2400) => t_tm = -7200s
    let toc = Epoch::from_time_of_week(2400, 0, crate::prelude::TimeScale::GPST);

    let eph = Ephemeris::default()
        .with_week(2400)
        .with_orbit("toe", OrbitItem::F64(0.0))
        .with_orbit("t_tm", OrbitItem::F64(-7200.0))
        .with_orbit("iodc", OrbitItem::F64(0x12a as f64))
        .with_orbit("iode", OrbitItem::F64(0x2a as f64))
        .with_orbit("sqrta", OrbitItem::F64(5153.6));

    let g01 = SV::new(Constellation::GPS, 1);

    let opts = GpsQzssEphCompositionOpts {
        satellite: g01,
        epoch: toc,
        alert: true,
        anti_spoofing: false,
        tlm_1: GpsQzssTelemetry {
            message: 1,
            integrity: false,
            reserved_bit: true,
        },
        tlm_2: GpsQzssTelemetry {
            message: 2,
            integrity: true,
            reserved_bit: false,
        },
        tlm_3: GpsQzssTelemetry {
            message: 3,
            integrity: false,
            reserved_bit: true,
        },
    };

    let message = eph.to_gps_qzss_eph_message(opts).unwrap();

    let subf1 = message.eph_1.subframe.as_eph1().unwrap();

    assert_eq!(subf1.week, (2399 % 1024) as u16, "transmission week");
    assert_eq!(subf1.toc, 0);
    assert_eq!(message.eph_1.how.tow, 597_606);
    assert_eq!(message.eph_2.how.tow, 597_612);
    assert_eq!(message.eph_3.how.tow, 597_618);

    let (toc_dut, dut) = bitstream_round_trip(&eph, g01, toc);

    assert_eq!(toc_dut, toc);
    assert_eq!(dut.get_week(), Some(2400));
    assert_eq!(dut.get_orbit_f64("t_tm"), Some(-7200.0));
    assert_eq!(dut.get_orbit_f64("toe"), Some(0.0));
}

/// Decoder that grabs EPH-1.2.3 at once specifically
struct GpsEph123BurstDecoder {
    size: usize,
    bit_offset: usize,
    buffer: [u8; 8192],
    decoder: GpsQzssDecoder,
}

impl GpsEph123BurstDecoder {
    pub fn new(path: &str) -> Self {
        let mut buffer = [0u8; 8192];
        let mut file = File::open(path).unwrap();
        let size = file.read(&mut buffer).unwrap();
        Self {
            buffer,
            size,
            bit_offset: 0,
            decoder: GpsQzssDecoder::default(),
        }
    }

    /// Pulls 1 EPH-1.2.3 burst from given isolated buffer,
    /// reconstructs object to run comparison directly.
    /// Internally verifies EPH-1, 2, 3 correct identity.
    pub fn pull_burst(&mut self, satellite: SV, epoch: Epoch) -> Option<GpsQzssEphemerisItem> {
        if self.size < 3 * GPS_FRAME_BYTES {
            None
        } else {
            let mut eph123 = GpsQzssEphemerisMessage::default();

            // pull EPH-1
            let (consumed, frame) = self.decoder.decode(&mut self.buffer, self.bit_offset);

            let frame = frame.unwrap_or_else(|| {
                panic!("Failed to pull EPH-1 for {}@{}", satellite, epoch);
            });

            assert_eq!(
                frame.how.frame_id,
                GpsQzssFrameId::Ephemeris1,
                "invalid frame: expecting EPH-1"
            );

            let eph_1 = frame.clone();
            self.bit_offset += consumed;

            // pull EPH-2
            let (consumed, frame) = self.decoder.decode(&mut self.buffer, self.bit_offset);
            let frame = frame.unwrap_or_else(|| {
                panic!("Failed to pull EPH-2 for {}@{}", satellite, epoch);
            });

            assert_eq!(
                frame.how.frame_id,
                GpsQzssFrameId::Ephemeris2,
                "invalid frame: expecting EPH-2"
            );

            let eph_2 = frame.clone();
            self.bit_offset += consumed;

            // pull EPH-2
            let (consumed, frame) = self.decoder.decode(&mut self.buffer, self.bit_offset);
            let frame = frame.unwrap_or_else(|| {
                panic!("Failed to pull EPH-2 for {}@{}", satellite, epoch);
            });

            assert_eq!(
                frame.how.frame_id,
                GpsQzssFrameId::Ephemeris3,
                "invalid frame: expecting EPH-3"
            );

            let eph_3 = frame.clone();
            self.bit_offset += consumed;

            self.size -= 3 * GPS_FRAME_BYTES;

            Some(GpsQzssEphemerisItem {
                satellite,
                epoch,
                eph123: GpsQzssEphemerisMessage {
                    eph_1,
                    eph_2,
                    eph_3,
                },
            })
        }
    }
}

/// Parses G15, G24, G31, J01, J02, J03 EPH-1.2.3 (3/5 30s) excerpt from NAV/V3/ESBCDNK.gzip
/// reconstruct a NAV RINEX from all collected bursts,
/// run bit true comparison between each Ephemeris contrusctured from each burst, and original
/// Ephemeris
#[test]
fn test_gps_qzss_lnav_eph123_bittrue() {
    // parse original file
    let esbcdnk =
        Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    let mut g15_dec = GpsEph123BurstDecoder::new("data/GPS/esbcdnk_g15_eph123.bin");
    let mut g24_dec = GpsEph123BurstDecoder::new("data/GPS/esbcdnk_g24_eph123.bin");
    let mut g31_dec = GpsEph123BurstDecoder::new("data/GPS/esbcdnk_g31_eph123.bin");
    let mut j01_dec = GpsEph123BurstDecoder::new("data/GPS/esbcdnk_j01_eph123.bin");
    let mut j02_dec = GpsEph123BurstDecoder::new("data/GPS/esbcdnk_j02_eph123.bin");
    let mut j03_dec = GpsEph123BurstDecoder::new("data/GPS/esbcdnk_j03_eph123.bin");

    let mut g15_count = 0;
    let mut g24_count = 0;
    let mut g31_count = 0;
    let mut j01_count = 0;
    let mut j02_count = 0;
    let mut j03_count = 0;

    // run bit true comparison for each burst, that _must_ exist
    // we take advantage for natural timeline sorting to easily grab each burst one after the other
    // when needed, to run the comparison
    for (key, eph) in esbcdnk.nav_ephemeris_frames_iter() {
        match key.sv.constellation {
            Constellation::GPS => {
                match key.sv.prn {
                    15 => {
                        // pull a burst in chronological order
                        let eph123 = g15_dec.pull_burst(key.sv, key.epoch).unwrap_or_else(|| {
                            panic!(
                                "Failed to pull a new EPH-123 burst for {} @ {}",
                                key.sv, key.epoch
                            );
                        });

                        // reconstruct a RINEX
                        let (reconstructed_toc, reconstructed_eph) = Ephemeris::from_gps_qzss_eph_message(eph123)
                                .unwrap_or_else(|e| {
                                    panic!("failed to reconstruct ToC and Eph from {} EPH-1.2.3 burst at {}: {}", key.sv, key.epoch, e);
                                });

                        // run ToC bit true test
                        assert_eq!(
                            reconstructed_toc, key.epoch,
                            "invalid ToC reconstructed for {}",
                            key.sv
                        );

                        // run EPH bit true test
                        assert_faithful_ephemeris(key.sv, key.epoch, eph, &reconstructed_eph);

                        g15_count += 1;
                    },
                    24 => {
                        // pull a burst in chronological order
                        let eph123 = g24_dec.pull_burst(key.sv, key.epoch).unwrap_or_else(|| {
                            panic!(
                                "Failed to pull a new EPH-123 burst for {} @ {}",
                                key.sv, key.epoch
                            );
                        });

                        // reconstruct a RINEX
                        let (reconstructed_toc, reconstructed_eph) = Ephemeris::from_gps_qzss_eph_message(eph123)
                                .unwrap_or_else(|e| {
                                    panic!("failed to reconstruct ToC and Eph from {} EPH-1.2.3 burst at {}: {}", key.sv, key.epoch, e);
                                });

                        // run ToC bit true test
                        assert_eq!(
                            reconstructed_toc, key.epoch,
                            "invalid ToC reconstructed for {}",
                            key.sv
                        );

                        // run EPH bit true test
                        assert_faithful_ephemeris(key.sv, key.epoch, eph, &reconstructed_eph);

                        g24_count += 1;
                    },
                    31 => {
                        // pull a burst in chronological order
                        let eph123 = g31_dec.pull_burst(key.sv, key.epoch).unwrap_or_else(|| {
                            panic!(
                                "Failed to pull a new EPH-123 burst for {} @ {}",
                                key.sv, key.epoch
                            );
                        });

                        // reconstruct a RINEX
                        let (reconstructed_toc, reconstructed_eph) = Ephemeris::from_gps_qzss_eph_message(eph123)
                                .unwrap_or_else(|e| {
                                    panic!("failed to reconstruct ToC and Eph from {} EPH-1.2.3 burst at {}: {}", key.sv, key.epoch, e);
                                });

                        // run ToC bit true test
                        assert_eq!(
                            reconstructed_toc, key.epoch,
                            "invalid ToC reconstructed for {}",
                            key.sv
                        );

                        // run EPH bit true test
                        assert_faithful_ephemeris(key.sv, key.epoch, eph, &reconstructed_eph);

                        g31_count += 1;
                    },
                    _ => {}, // ignored
                }
            },
            Constellation::QZSS => {
                match key.sv.prn {
                    1 => {
                        // pull a burst in chronological order
                        let eph123 = j01_dec.pull_burst(key.sv, key.epoch).unwrap_or_else(|| {
                            panic!(
                                "Failed to pull a new EPH-123 burst for {} @ {}",
                                key.sv, key.epoch
                            );
                        });

                        // reconstruct a RINEX
                        let (reconstructed_toc, reconstructed_eph) = Ephemeris::from_gps_qzss_eph_message(eph123)
                                .unwrap_or_else(|e| {
                                    panic!("failed to reconstruct ToC and Eph from {} EPH-1.2.3 burst at {}: {}", key.sv, key.epoch, e);
                                });

                        // run ToC bit true test
                        assert_eq!(
                            reconstructed_toc, key.epoch,
                            "invalid ToC reconstructed for {}",
                            key.sv
                        );

                        // run EPH bit true test
                        assert_faithful_ephemeris(key.sv, key.epoch, eph, &reconstructed_eph);

                        j01_count += 1;
                    },
                    2 => {
                        // pull a burst in chronological order
                        let eph123 = j02_dec.pull_burst(key.sv, key.epoch).unwrap_or_else(|| {
                            panic!(
                                "Failed to pull a new EPH-123 burst for {} @ {}",
                                key.sv, key.epoch
                            );
                        });

                        // reconstruct a RINEX
                        let (reconstructed_toc, reconstructed_eph) = Ephemeris::from_gps_qzss_eph_message(eph123)
                                .unwrap_or_else(|e| {
                                    panic!("failed to reconstruct ToC and Eph from {} EPH-1.2.3 burst at {}: {}", key.sv, key.epoch, e);
                                });

                        // run ToC bit true test
                        assert_eq!(
                            reconstructed_toc, key.epoch,
                            "invalid ToC reconstructed for {}",
                            key.sv
                        );

                        // run EPH bit true test
                        assert_faithful_ephemeris(key.sv, key.epoch, eph, &reconstructed_eph);

                        j02_count += 1;
                    },
                    3 => {
                        // pull a burst in chronological order
                        let eph123 = j03_dec.pull_burst(key.sv, key.epoch).unwrap_or_else(|| {
                            panic!(
                                "Failed to pull a new EPH-123 burst for {} @ {}",
                                key.sv, key.epoch
                            );
                        });

                        // reconstruct a RINEX
                        let (reconstructed_toc, reconstructed_eph) = Ephemeris::from_gps_qzss_eph_message(eph123)
                                .unwrap_or_else(|e| {
                                    panic!("failed to reconstruct ToC and Eph from {} EPH-1.2.3 burst at {}: {}", key.sv, key.epoch, e);
                                });

                        // run ToC bit true test
                        assert_eq!(
                            reconstructed_toc, key.epoch,
                            "invalid ToC reconstructed for {}",
                            key.sv
                        );

                        // run EPH bit true test
                        assert_faithful_ephemeris(key.sv, key.epoch, eph, &reconstructed_eph);

                        j03_count += 1;
                    },
                    _ => {}, // ignored
                }
            },
            _ => {},
        }
    }
    assert_eq!(g15_count, 24 / 3);
    assert_eq!(g24_count, 21 / 3);
    assert_eq!(g31_count, 27 / 3);
    assert_eq!(j01_count, 12 / 3);
    assert_eq!(j02_count, 18 / 3);
    assert_eq!(j03_count, 15 / 3);
}
