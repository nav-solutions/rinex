use std::{
    fs::File,
    io::Write,
};

use rinex::{
    navigation::{GpsQzss30sMessage, GpsQzssCompositionOpts},
    prelude::{Constellation, Rinex},
};

use env_logger::init;

use log::info;

use hifitime::prelude::{Unit, TimeSeries, TimeScale};

use gnss_protos::{GpsQzssFrame, GpsQzssTelemetry, Message};

struct FileBuffer {
    // File pointer
    fd: File,

    // Enough for 30s burst to fit in
    buffer: [u8; 1024],
}

impl FileBuffer {
    pub fn new(path: &str) -> Self {
        Self {
            buffer: [0u8; 1024],
            fd: File::create(path)
                .unwrap_or_else(|e| {
                    panic!("failed to create file {}", path);
                }),
        }
    }

    pub fn flush(&mut self) {
        self.fd.flush().unwrap();
    }

    /// Writes [GpsQzss30sMessage] to file
    pub fn write_message(&mut self, msg: GpsQzss30sMessage) {
        self.write_frame(msg.eph_1());
        self.write_frame(msg.eph_2());
        self.write_frame(msg.eph_3());
        self.write_frame(msg.alm_4());
        self.write_frame(msg.alm_5());
    }

    /// Writes [GpsQzssFrame] to file
    pub fn write_frame(&mut self, frame: GpsQzssFrame) {
        let size = frame.encode(&mut self.buffer)
            .unwrap_or_else(|e| {
                panic!("failed to encode GPS frame: {}", e);
            });

        self.fd.write_all(&self.buffer[..size])
            .unwrap_or_else(|e| {
                panic!("failed to write GPS frame into file: {}", e);
            });
    }
}

/// Reconstructs 120s of 
/// - GPS 30" burst for G15, G24 and G31
/// - QZSS 30" burst for J01, J02 and J03 
/// using ESBC00DNK_R_20201770000_01D_MN.rnx.gz (profesionnal file)
/// and dumps them to individual binary files.
pub fn main() {
    env_logger::Builder::from_env(
       env_logger::Env::default().default_filter_or("debug") 
    ).init();


    // full RINEX path
    let fullpath = "data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz";

    // isolated buffers
    let mut g15_buf = FileBuffer::new("data/GPS/esbcdnk_g15.bin");
    let mut g24_buf = FileBuffer::new("data/GPS/esbcdnk_g24.bin");
    let mut g31_buf = FileBuffer::new("data/GPS/esbcdnk_g31.bin");
    let mut j01_buf = FileBuffer::new("data/GPS/esbcdnk_j01.bin");
    let mut j02_buf = FileBuffer::new("data/GPS/esbcdnk_j02.bin");
    let mut j03_buf = FileBuffer::new("data/GPS/esbcdnk_j03.bin");

    // let mut e07_buf = FileBuffer::new("data/GAL/esbcdnk_e07.bin");
    // let mut e11_buf = FileBuffer::new("data/GAL/esbcdnk_e11.bin");
    // let mut e15_buf = FileBuffer::new("data/GAL/esbcdnk_e15.bin");

    let rinex = Rinex::from_gzip_file(fullpath).unwrap();

    // design temporal arc
    let t0 = rinex.first_epoch()
        .unwrap()
        .to_time_scale(TimeScale::GPST)
        + 3600.0 * Unit::Second; // random, not aligned to GPS
    
    let arc = TimeSeries::inclusive(
        t0,
        t0 + 120.0 * Unit::Second,
        6.0 * Unit::Second,
    );

    for message in rinex.to_gps_qzss_messages(
        arc,
        |(satellite, t_tx)| {
            GpsQzssCompositionOpts {
                satellite,
                epoch: t_tx,
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
                tlm_4: GpsQzssTelemetry {
                    message: 4,
                    integrity: false,
                    reserved_bit: false,
                },
                tlm_5: GpsQzssTelemetry {
                    message: 5,
                    integrity: false,
                    reserved_bit: false,
                },
                alm_4: None,
                alm_5: None,
            }
        }
    ) {
        match message {
            Some(message) => {
                info!("encoded {:?}", message);

                match (message.satellite.constellation, message.satellite.prn) {
                    (Constellation::GPS, 15) => g15_buf.write_message(message),
                    (Constellation::GPS, 24) => g24_buf.write_message(message),
                    (Constellation::GPS, 31) => g24_buf.write_message(message),
                    (Constellation::QZSS, 1) => j01_buf.write_message(message),
                    (Constellation::QZSS, 2) => j02_buf.write_message(message),
                    (Constellation::QZSS, 3) => j03_buf.write_message(message),
                    _ => {},
                }
            },
            None => {
                // method will fail when it is impossible to select
                // a valid ephemeris frame (from that file) at specific point
                // in temporal arc.
                // You get a debug trace with "log" feature.
            },
        }
    }
    g15_buf.flush();
    g24_buf.flush();
    g31_buf.flush();
    j01_buf.flush();
    j02_buf.flush();
    j03_buf.flush();
}
