use std::fs::File;
use std::io::Write;

use rinex::{
    navigation::{GpsQzssEphemerisMessage, GpsQzssEphemerisTelemetries},
    prelude::{Constellation, Rinex, SV},
};

use gnss_protos::{GpsQzssFrame, GpsQzssTelemetry};

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

    /// Encode EPH-1,2,3 burst
    pub fn encode_frame(&mut self, frame: GpsQzssFrame) {}

    /// Dump buffer to static file
    pub fn save_file(&self) {
        let mut fd = File::open(self.path).unwrap_or_else(|e| {
            panic!("Failed to open (WR) file: {} - {}", self.path, e);
        });

        fd.write_all(&self.buffer).unwrap_or_else(|e| {
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
    let fullpath = "../data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz";

    // GPS
    let g15 = SV::new(Constellation::GPS, 15);
    let g24 = SV::new(Constellation::GPS, 24);
    let g31 = SV::new(Constellation::GPS, 31);

    // JPAN (seen from DNK :'))
    let j01 = SV::new(Constellation::QZSS, 01);
    let j02 = SV::new(Constellation::QZSS, 02);
    let j03 = SV::new(Constellation::QZSS, 03);

    // GAL
    let e07 = SV::new(Constellation::Galileo, 07);
    let e11 = SV::new(Constellation::Galileo, 11);
    let e15 = SV::new(Constellation::Galileo, 15);

    // Dumps
    let mut g15_buf = Buffer::<8192>::new("../data/GPS/esbcdnk_g15_eph123.bin");
    let mut g24_buf = Buffer::<8192>::new("../data/GPS/esbcdnk_g24_eph123.bin");
    let mut g31_buf = Buffer::<8192>::new("../data/GPS/esbcdnk_g31_eph123.bin");
    let mut j01_buf = Buffer::<8192>::new("../data/GPS/esbcdnk_j01_eph123.bin");
    let mut j02_buf = Buffer::<8192>::new("../data/GPS/esbcdnk_j02_eph123.bin");
    let mut j03_buf = Buffer::<8192>::new("../data/GPS/esbcdnk_j03_eph123.bin");
    let mut e07_buf = Buffer::<8192>::new("../data/GPS/esbcdnk_e07_eph123.bin");
    let mut e11_buf = Buffer::<8192>::new("../data/GPS/esbcdnk_e11_eph123.bin");
    let mut e15_buf = Buffer::<8192>::new("../data/GPS/esbcdnk_e15_eph123.bin");

    let rinex = Rinex::from_gzip_file(fullpath).unwrap();

    // limited to ephemeris frames
    for (key, eph) in rinex.nav_ephemeris_frames_iter() {
        let tlm = GpsQzssEphemerisTelemetries {
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

        match key.sv {
            g15 => {
                // extract
                let burst = eph
                    .to_gps_qzss_eph_message(key.sv, key.epoch, false, false, tlm)
                    .unwrap_or_else(|e| {
                        panic!(
                            "Failed to extract an Ephemeris burst for {} @ {}",
                            key.sv, key.epoch
                        );
                    });

                g15_buf.encode_frame(burst.eph_1); // EPH-1
                g15_buf.encode_frame(burst.eph_2); // EPH-2
                g15_buf.encode_frame(burst.eph_3); // EPH-3
            },
            g24 => {},
            g31 => {},
            j01 => {},
            j02 => {},
            j03 => {},
            e07 => {},
            e11 => {},
            e15 => {},
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
