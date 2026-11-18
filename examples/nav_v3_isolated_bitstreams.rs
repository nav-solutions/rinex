use std::fs::File;
use std::io::Write;

use rinex::{
    navigation::{GpsQzssEphCompositionOpts, GpsQzssEphemerisMessage},
    prelude::{Constellation, Rinex},
};

use gnss_protos::{GpsQzssFrame, GpsQzssTelemetry, Message};

struct Buffer<'a, const N: usize> {
    /// destination
    pub path: &'a str,

    /// internal buffer
    pub buffer: [u8; N],

    /// current pointer in buffer
    pub ptr: usize,
}

impl<'a, const N: usize> Buffer<'a, N> {
    /// Buffer allocation + init
    pub fn new(path: &'a str) -> Self {
        Self {
            path,
            ptr: 0,
            buffer: [0; N],
        }
    }

    /// Encodes EPH-1,2,3 burst
    pub fn encode_eph123_burst(&mut self, burst: GpsQzssEphemerisMessage) {
        self.encode_frame(burst.eph_1);
        self.encode_frame(burst.eph_2);
        self.encode_frame(burst.eph_3);
    }

    /// Encodes one GPS frame
    pub fn encode_frame(&mut self, frame: GpsQzssFrame) {
        let size = frame.encode(&mut self.buffer[self.ptr..]).unwrap();
        self.ptr += size;
    }

    /// Dump buffer to static file
    pub fn save_file(&self) {
        let mut fd = File::create(self.path).unwrap_or_else(|e| {
            panic!("Failed to open (WR) file: {} - {}", self.path, e);
        });

        fd.write_all(&self.buffer[..self.ptr]).unwrap_or_else(|e| {
            panic!("Failed to dump buffer to file: {} - {}", self.path, e);
        });
    }
}

impl<'a, const N: usize> Write for Buffer<'a, N> {
    fn write(&mut self, src: &[u8]) -> Result<usize, std::io::Error> {
        let size = src.len();
        self.buffer[self.ptr..].copy_from_slice(&src);
        self.ptr += size;
        Ok(size)
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        self.ptr = 0;
        Ok(())
    }
}

/// Extracts (approx 10 messages, slow RINEX rate)
/// from G15, G24, G31, J01, J02, J03, E07, E11 and E15
/// from ESBC00DNK_R_20201770000_01D_MN.rnx.gz (profesionnal file)
/// and dumps them to their respective bit stream protocol, into individual files
/// (isolated bit streams). Each resulting files serves as further testing purposes
/// and demos.
pub fn main() {
    // full RINEX path
    let fullpath = "data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz";

    // Dumps
    let mut g15_buf = Buffer::<8192>::new("data/GPS/esbcdnk_g15_eph123.bin");
    let mut g24_buf = Buffer::<8192>::new("data/GPS/esbcdnk_g24_eph123.bin");
    let mut g31_buf = Buffer::<8192>::new("data/GPS/esbcdnk_g31_eph123.bin");
    let mut j01_buf = Buffer::<8192>::new("data/GPS/esbcdnk_j01_eph123.bin");
    let mut j02_buf = Buffer::<8192>::new("data/GPS/esbcdnk_j02_eph123.bin");
    let mut j03_buf = Buffer::<8192>::new("data/GPS/esbcdnk_j03_eph123.bin");
    let mut e07_buf = Buffer::<8192>::new("data/GPS/esbcdnk_e07_eph123.bin");
    let mut e11_buf = Buffer::<8192>::new("data/GPS/esbcdnk_e11_eph123.bin");
    let mut e15_buf = Buffer::<8192>::new("data/GPS/esbcdnk_e15_eph123.bin");

    let rinex = Rinex::from_gzip_file(fullpath).unwrap();

    // limited to ephemeris frames
    for (key, eph) in rinex.nav_ephemeris_frames_iter() {
        let opts = GpsQzssEphCompositionOpts {
            satellite: key.sv,
            epoch: key.epoch,
            alert: false,
            anti_spoofing: false,
            tlm_1: GpsQzssTelemetry {
                message: 1,
                integrity: false,
                reserved_bit: false,
            },
            tlm_2: GpsQzssTelemetry {
                message: 2,
                integrity: false,
                reserved_bit: false,
            },
            tlm_3: GpsQzssTelemetry {
                message: 3,
                integrity: false,
                reserved_bit: false,
            },
        };

        match (key.sv.constellation, key.sv.prn) {
            (Constellation::GPS, 15) => {
                // extract
                let burst = eph.to_gps_qzss_eph_message(opts).unwrap_or_else(|e| {
                    panic!(
                        "Failed to extract an Ephemeris burst for {} @ {} - {}",
                        key.sv, key.epoch, e
                    );
                });
                g15_buf.encode_eph123_burst(burst);
            },
            (Constellation::GPS, 24) => {
                // extract
                let burst = eph.to_gps_qzss_eph_message(opts).unwrap_or_else(|e| {
                    panic!(
                        "Failed to extract an Ephemeris burst for {} @ {} - {}",
                        key.sv, key.epoch, e,
                    );
                });
                g24_buf.encode_eph123_burst(burst);
            },
            (Constellation::GPS, 31) => {
                // extract
                let burst = eph.to_gps_qzss_eph_message(opts).unwrap_or_else(|e| {
                    panic!(
                        "Failed to extract an Ephemeris burst for {} @ {} - {}",
                        key.sv, key.epoch, e
                    );
                });
                g31_buf.encode_eph123_burst(burst);
            },
            (Constellation::QZSS, 1) => {
                // extract
                let burst = eph.to_gps_qzss_eph_message(opts).unwrap_or_else(|e| {
                    panic!(
                        "Failed to extract an Ephemeris burst for {} @ {} - {}",
                        key.sv, key.epoch, e
                    );
                });
                j01_buf.encode_eph123_burst(burst);
            },
            (Constellation::QZSS, 2) => {
                // extract
                let burst = eph.to_gps_qzss_eph_message(opts).unwrap_or_else(|e| {
                    panic!(
                        "Failed to extract an Ephemeris burst for {} @ {} - {}",
                        key.sv, key.epoch, e
                    );
                });
                j02_buf.encode_eph123_burst(burst);
            },
            (Constellation::QZSS, 3) => {
                // extract
                let burst = eph.to_gps_qzss_eph_message(opts).unwrap_or_else(|e| {
                    panic!(
                        "Failed to extract an Ephemeris burst for {} @ {} - {}",
                        key.sv, key.epoch, e
                    );
                });
                j03_buf.encode_eph123_burst(burst);
            },
            // (Constellation::Galileo, 7) => {
            //     // extract
            //     let burst = eph
            //         .to_gps_qzss_eph_message(key.sv, key.epoch, false, false, tlm)
            //         .unwrap_or_else(|e| {
            //             panic!(
            //                 "Failed to extract an Ephemeris burst for {} @ {}",
            //                 key.sv, key.epoch
            //             );
            //         });
            //     e07_buf.encode_eph123_burst(burst);
            // },
            // (Constellation::Galileo, 11) => {
            //     // extract
            //     let burst = eph
            //         .to_gps_qzss_eph_message(key.sv, key.epoch, false, false, tlm)
            //         .unwrap_or_else(|e| {
            //             panic!(
            //                 "Failed to extract an Ephemeris burst for {} @ {}",
            //                 key.sv, key.epoch
            //             );
            //         });
            //     e11_buf.encode_eph123_burst(burst);
            // },
            // (Constellation::Galileo, 15) => {
            //     // extract
            //     let burst = eph
            //         .to_gps_qzss_eph_message(key.sv, key.epoch, false, false, tlm)
            //         .unwrap_or_else(|e| {
            //             panic!(
            //                 "Failed to extract an Ephemeris burst for {} @ {}",
            //                 key.sv, key.epoch
            //             );
            //         });
            //     e15_buf.encode_eph123_burst(burst);
            // },
            _ => {},
        }
    }

    // generate each files
    g15_buf.save_file();
    g24_buf.save_file();
    g31_buf.save_file();
    j01_buf.save_file();
    j02_buf.save_file();
    j03_buf.save_file();
    e07_buf.save_file();
    e11_buf.save_file();
    e15_buf.save_file();
}
