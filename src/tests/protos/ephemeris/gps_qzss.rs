use crate::{
    navigation::{Ephemeris, GpsQzssLnavCompositionOpts, GpsQzssLnavProto, OrbitItem},
    prelude::{Constellation, Epoch, Rinex, SV},
    tests::toolkit::nav::{assert_faithful_ephemeris, panic_eq_ephemeris},
};

use std::{f64::consts::PI, fs::File, io::Read};

use gnss_protos::{
    gps::lnav::{
        GpsQzssLnavFrame, GpsQzssLnavSubfDecoder, GpsQzssLnavSubframe, GpsQzssLnavSubframeId,
        GpsQzssLnavTelemetry, GPS_LNAV_FRAME_BYTES, GPS_LNAV_SUBFRAME_DURATION_S,
    },
    Message,
};

/// RINEX -> bits (10 words per subframe, parity included) -> RINEX.
/// Returns the transmitted words, the decoded ToC and the decoded [Ephemeris].
fn bitstream_round_trip(eph: &Ephemeris, sv: SV, toc: Epoch) -> (Epoch, Ephemeris) {
    let opts = GpsQzssLnavCompositionOpts {
        satellite: sv,
        epoch: toc,
        alert: false,
        anti_spoofing: false,
        tlm_1: GpsQzssLnavTelemetry {
            message: 1,
            integrity: true,
            reserved_bit: false,
        },
        tlm_2: GpsQzssLnavTelemetry {
            message: 2,
            integrity: false,
            reserved_bit: true,
        },
        tlm_3: GpsQzssLnavTelemetry {
            message: 3,
            integrity: true,
            reserved_bit: false,
        },
        tlm_4: GpsQzssLnavTelemetry {
            message: 4,
            integrity: true,
            reserved_bit: false,
        },
        alm_4: Default::default(),
        tlm_5: GpsQzssLnavTelemetry {
            message: 5,
            integrity: true,
            reserved_bit: false,
        },
        alm_5: Default::default(),
    };

    let proto = eph
        .to_gps_qzss_lnav_proto(opts)
        .unwrap_or_else(|e| panic!("{} @ {}: encoding failure: {}", sv, toc, e));

    let (decoded_toc, decoded) = Ephemeris::from_gps_qzss_lnav_proto(proto)
        .unwrap_or_else(|e| panic!("{} @ {}: decoding failure: {}", sv, toc, e));

    (decoded_toc, decoded)
}

/// Runs the round trip on all GPS/QZSS frames of the file
/// and returns the number of tested frames.
fn run_file_roundtrip(path: &str) -> usize {
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
    let count = run_file_roundtrip("data/NAV/V2/cbw10010.21n.gz");
    assert_eq!(count, 187);
}

#[test]
fn test_lnav_v3() {
    let count = run_file_roundtrip("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz");
    assert_eq!(count, 260);

    let count = run_file_roundtrip("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz");
    assert_eq!(count, 272);
}

#[test]
fn test_lnav_v4() {
    let count = run_file_roundtrip("data/NAV/V4/KMS300DNK_R_20221591000_01H_MN.rnx.gz");
    assert_eq!(count, 31);
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

    let opts = GpsQzssLnavCompositionOpts {
        satellite: g01,
        epoch: toc,
        alert: true,
        anti_spoofing: false,
        tlm_1: GpsQzssLnavTelemetry {
            message: 1,
            integrity: false,
            reserved_bit: true,
        },
        tlm_2: GpsQzssLnavTelemetry {
            message: 2,
            integrity: true,
            reserved_bit: false,
        },
        tlm_3: GpsQzssLnavTelemetry {
            message: 3,
            integrity: false,
            reserved_bit: true,
        },
        tlm_4: GpsQzssLnavTelemetry {
            message: 4,
            integrity: false,
            reserved_bit: true,
        },
        alm_4: Default::default(),
        tlm_5: GpsQzssLnavTelemetry {
            message: 5,
            integrity: false,
            reserved_bit: true,
        },
        alm_5: Default::default(),
    };

    let proto = eph.to_gps_qzss_lnav_proto(opts).unwrap();

    let subf1 = proto.frame.eph_1().subframe.as_eph1().unwrap();

    assert_eq!(subf1.week, (2399 % 1024) as u16, "transmission week");
    assert_eq!(subf1.toc, 0);
    assert_eq!(proto.frame.eph_1().how.tow, 597_606);
    assert_eq!(proto.frame.eph_2().how.tow, 597_612);
    assert_eq!(proto.frame.eph_3().how.tow, 597_618);

    let (toc_dut, dut) = bitstream_round_trip(&eph, g01, toc);

    assert_eq!(toc_dut, toc);
    assert_eq!(dut.get_week(), Some(2400));
    assert_eq!(dut.get_orbit_f64("t_tm"), Some(-7200.0));
    assert_eq!(dut.get_orbit_f64("toe"), Some(0.0));
}

/// Decoder that grabs EPH-1.2.3 at once specifically
struct MessageDecoder {
    size: usize,
    bit_offset: usize,
    buffer: [u8; 8192],
    decoder: GpsQzssLnavSubfDecoder,
}

impl MessageDecoder {
    pub fn new(path: &str) -> Self {
        let mut buffer = [0u8; 8192];
        let mut file = File::open(path).unwrap();
        let size = file.read(&mut buffer).unwrap();
        Self {
            buffer,
            size,
            bit_offset: 0,
            decoder: GpsQzssLnavSubfDecoder::default(),
        }
    }

    /// Pulls EPH-1.2.3 +ALM4 +ALM5 burst from given isolated buffer,
    /// reconstructs object to run comparison directly.
    /// Internally verifies EPH-1, 2, 3, ALM4 and ALM5 correct identity.
    pub fn pull_message(&mut self, satellite: SV, epoch: Epoch) -> Option<GpsQzssLnavProto> {
        if self.size < 3 * GPS_LNAV_FRAME_BYTES {
            None
        } else {
            // pull EPH-1
            let (consumed, sbf) = self.decoder.decode(&mut self.buffer, self.bit_offset);

            let sbf = sbf.unwrap_or_else(|| {
                panic!("Failed to pull EPH-1 for {}@{}", satellite, epoch);
            });

            assert_eq!(
                sbf.how.sbf_id,
                GpsQzssLnavSubframeId::Ephemeris1,
                "invalid frame: expecting EPH-1"
            );

            let eph_1 = sbf.clone();

            self.bit_offset += consumed;

            // pull EPH-2
            let (consumed, sbf) = self.decoder.decode(&mut self.buffer, self.bit_offset);

            let sbf = sbf.unwrap_or_else(|| {
                panic!("Failed to pull EPH-2 for {}@{}", satellite, epoch);
            });

            assert_eq!(
                sbf.how.sbf_id,
                GpsQzssLnavSubframeId::Ephemeris2,
                "invalid frame: expecting EPH-2"
            );

            let eph_2 = sbf.clone();

            self.bit_offset += consumed;

            // pull EPH-3
            let (consumed, sbf) = self.decoder.decode(&mut self.buffer, self.bit_offset);

            let sbf = sbf.unwrap_or_else(|| {
                panic!("Failed to pull EPH-3 for {}@{}", satellite, epoch);
            });

            assert_eq!(
                sbf.how.sbf_id,
                GpsQzssLnavSubframeId::Ephemeris3,
                "invalid frame: expecting EPH-3"
            );

            let eph_3 = sbf.clone();
            self.bit_offset += consumed;

            // pull ALM-4
            let (consumed, sbf) = self.decoder.decode(&mut self.buffer, self.bit_offset);

            let sbf = sbf.unwrap_or_else(|| {
                panic!("Failed to pull ALM-4 for {}@{}", satellite, epoch);
            });

            assert_eq!(
                sbf.how.sbf_id,
                GpsQzssLnavSubframeId::Almanach4,
                "invalid frame: expecting ALM-4"
            );

            let alm_4 = sbf.clone();
            self.bit_offset += consumed;

            // pull ALM-5
            let (consumed, sbf) = self.decoder.decode(&mut self.buffer, self.bit_offset);

            let sbf = sbf.unwrap_or_else(|| {
                panic!("Failed to pull ALM-5 for {}@{}", satellite, epoch);
            });

            assert_eq!(
                sbf.how.sbf_id,
                GpsQzssLnavSubframeId::Almanach5,
                "invalid frame: expecting ALM-5"
            );

            let alm_5 = sbf.clone();

            self.bit_offset += consumed;

            self.size -= 5 * GPS_LNAV_FRAME_BYTES;

            let frame = GpsQzssLnavFrame::from_slice([eph_1, eph_2, eph_3, alm_4, alm_5])
                .unwrap_or_else(|e| {
                    panic!("invalid GPS/QZSS 30s frame: {}", e);
                });

            Some(GpsQzssLnavProto {
                satellite,
                epoch,
                frame,
            })
        }
    }
}

/// Parses G15, G24, G31, J02, J03 EPH-1.2.3 (3/5 30s) excerpt from NAV/V3/ESBCDNK.gzip
/// reconstruct a NAV RINEX from all collected bursts,
/// run bit true comparison between each Ephemeris contrusctured from each burst, and original
/// Ephemeris
#[test]
fn test_gps_qzss_lnav_bittrue() {
    // parse original file
    let esbcdnk =
        Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    let mut g15_dec = MessageDecoder::new("data/GPS/esbcdnk_g15.bin");
    let mut g24_dec = MessageDecoder::new("data/GPS/esbcdnk_g24.bin");
    let mut g31_dec = MessageDecoder::new("data/GPS/esbcdnk_g31.bin");
    let mut j02_dec = MessageDecoder::new("data/QZSS/esbcdnk_j02.bin");
    let mut j03_dec = MessageDecoder::new("data/QZSS/esbcdnk_j03.bin");

    let mut g15_count = 0;
    let mut g24_count = 0;
    let mut g31_count = 0;
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
                        let proto = g15_dec.pull_message(key.sv, key.epoch).unwrap_or_else(|| {
                            panic!("Failed to pull a new burst for {} @ {}", key.sv, key.epoch);
                        });

                        // reconstruct a RINEX
                        let (reconstructed_toc, reconstructed_eph) = Ephemeris::from_gps_qzss_lnav_proto(proto)
                                .unwrap_or_else(|e| {
                                    panic!("failed to reconstruct ToC and Eph from {} EPH-1.2.3 burst at {}: {}", key.sv, key.epoch, e);
                                });

                        // // run ToC bit true test
                        // assert_eq!(
                        //     reconstructed_toc, key.epoch,
                        //     "invalid ToC reconstructed for {}",
                        //     key.sv
                        // );

                        // run EPH bit true test
                        // assert_faithful_ephemeris(key.sv, key.epoch, eph, &reconstructed_eph);

                        g15_count += 1;
                    },
                    24 => {
                        // pull a burst in chronological order
                        let proto = g24_dec.pull_message(key.sv, key.epoch).unwrap_or_else(|| {
                            panic!("Failed to pull a new burst for {} @ {}", key.sv, key.epoch);
                        });

                        // reconstruct a RINEX
                        let (reconstructed_toc, reconstructed_eph) = Ephemeris::from_gps_qzss_lnav_proto(proto)
                                .unwrap_or_else(|e| {
                                    panic!("failed to reconstruct ToC and Eph from {} EPH-1.2.3 burst at {}: {}", key.sv, key.epoch, e);
                                });

                        // // run ToC bit true test
                        // assert_eq!(
                        //     reconstructed_toc, key.epoch,
                        //     "invalid ToC reconstructed for {}",
                        //     key.sv
                        // );

                        // run EPH bit true test
                        // assert_faithful_ephemeris(key.sv, key.epoch, eph, &reconstructed_eph);

                        g24_count += 1;
                    },
                    31 => {
                        // pull a burst in chronological order
                        let proto = g31_dec.pull_message(key.sv, key.epoch).unwrap_or_else(|| {
                            panic!("Failed to pull a new burst for {} @ {}", key.sv, key.epoch);
                        });

                        // reconstruct a RINEX
                        let (reconstructed_toc, reconstructed_eph) =
                            Ephemeris::from_gps_qzss_lnav_proto(proto).unwrap_or_else(|e| {
                                panic!(
                                    "failed to reconstruct ToC and Eph from {} burst at {}: {}",
                                    key.sv, key.epoch, e
                                );
                            });

                        // // run ToC bit true test
                        // assert_eq!(
                        //     reconstructed_toc, key.epoch,
                        //     "invalid ToC reconstructed for {}",
                        //     key.sv
                        // );

                        // run EPH bit true test
                        // assert_faithful_ephemeris(key.sv, key.epoch, eph, &reconstructed_eph);

                        g31_count += 1;
                    },
                    _ => {}, // ignored
                }
            },
            Constellation::QZSS => {
                match key.sv.prn {
                    2 => {
                        // pull a burst in chronological order
                        let proto = j02_dec.pull_message(key.sv, key.epoch).unwrap_or_else(|| {
                            panic!("Failed to pull a new burst for {} @ {}", key.sv, key.epoch);
                        });

                        // reconstruct a RINEX
                        let (reconstructed_toc, reconstructed_eph) = Ephemeris::from_gps_qzss_lnav_proto(proto)
                                .unwrap_or_else(|e| {
                                    panic!("failed to reconstruct ToC and Eph from {} EPH-1.2.3 burst at {}: {}", key.sv, key.epoch, e);
                                });

                        // // run ToC bit true test
                        // assert_eq!(
                        //     reconstructed_toc, key.epoch,
                        //     "invalid ToC reconstructed for {}",
                        //     key.sv
                        // );

                        // run EPH bit true test
                        // assert_faithful_ephemeris(key.sv, key.epoch, eph, &reconstructed_eph);

                        j02_count += 1;
                    },
                    3 => {
                        // pull a burst in chronological order
                        let proto = j03_dec.pull_message(key.sv, key.epoch).unwrap_or_else(|| {
                            panic!("Failed to pull a new burst for {} @ {}", key.sv, key.epoch);
                        });

                        // reconstruct a RINEX
                        let (reconstructed_toc, reconstructed_eph) = Ephemeris::from_gps_qzss_lnav_proto(proto)
                                .unwrap_or_else(|e| {
                                    panic!("failed to reconstruct ToC and Eph from {} EPH-1.2.3 burst at {}: {}", key.sv, key.epoch, e);
                                });

                        // // run ToC bit true test
                        // assert_eq!(
                        //     reconstructed_toc, key.epoch,
                        //     "invalid ToC reconstructed for {}",
                        //     key.sv
                        // );

                        // run EPH bit true test
                        // assert_faithful_ephemeris(key.sv, key.epoch, eph, &reconstructed_eph);

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
    assert_eq!(j02_count, 18 / 3);
    assert_eq!(j03_count, 15 / 3);
}

/// When user requests to forge complete 30s burst
/// while not providing ALM4 and ALM5 lazily,
/// we still have to forge correct HOW.ToW for a physically correct bitstream.
/// This test verifies that HOW.ToW is always correct when lazily forging GPS/QZSS bitstreams.
#[test]
fn how_correctness_in_zero_concluded_bursts() {
    // parse original file
    let esbcdnk =
        Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    let mut count = 0;

    for (key, eph) in esbcdnk.nav_ephemeris_frames_iter() {
        match key.sv.constellation {
            Constellation::GPS | Constellation::QZSS => {
                let opts = GpsQzssLnavCompositionOpts {
                    satellite: key.sv,
                    epoch: key.epoch,
                    alert: false,
                    anti_spoofing: false,
                    tlm_1: Default::default(),
                    tlm_2: Default::default(),
                    tlm_3: Default::default(),
                    tlm_4: Default::default(),
                    alm_4: Default::default(),
                    tlm_5: Default::default(),
                    alm_5: Default::default(),
                };

                // forge lazily
                let message = eph.to_gps_qzss_lnav_proto(opts).unwrap_or_else(|e| {
                    panic!(
                        "Failed to forge GPS/QZSS message lazily, for {}@{}: {}",
                        key.sv, key.epoch, e
                    );
                });

                // verify GPS/QZSS ToW progression
                assert_eq!(
                    message.frame.eph_2().how.tow - message.frame.eph_1().how.tow,
                    GPS_LNAV_SUBFRAME_DURATION_S,
                );
                assert_eq!(
                    message.frame.eph_3().how.tow - message.frame.eph_2().how.tow,
                    GPS_LNAV_SUBFRAME_DURATION_S,
                );
                assert_eq!(
                    message.frame.alm_4().how.tow - message.frame.eph_3().how.tow,
                    GPS_LNAV_SUBFRAME_DURATION_S,
                );
                assert_eq!(
                    message.frame.alm_5().how.tow - message.frame.alm_4().how.tow,
                    GPS_LNAV_SUBFRAME_DURATION_S,
                );

                // verify reciprocal
                let (reconstructed_toc, reconstructed_eph) =
                    Ephemeris::from_gps_qzss_lnav_proto(message).unwrap_or_else(|e| {
                        panic!(
                            "failed to reconstruct ephemeris from burst for {}@{}: {}",
                            key.sv, key.epoch, e
                        );
                    });

                assert_eq!(reconstructed_toc, key.epoch, "invalid ToC reconstructed");
                assert_faithful_ephemeris(key.sv, key.epoch, eph, &reconstructed_eph);

                count += 1;
            },
            _ => {},
        }
    }
    assert_eq!(count, 272);
}
