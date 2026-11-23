use std::{fs::File, io::Write};

use rinex::{
    navigation::{GpsQzssLnavCompositionOpts, GpsQzssLnavProto},
    prelude::{Constellation, Rinex},
};

use env_logger::init;

use log::info;

use hifitime::prelude::{TimeScale, TimeSeries, Unit};

use gnss_protos::{
    gps::lnav::{GpsQzssLnavFrame, GpsQzssLnavSubframe, GpsQzssLnavTelemetry},
    Message,
};

struct FileBuffer<const N: usize> {
    fd: File,
    buffer: [u8; N],
}

impl<const N: usize> FileBuffer<N> {
    pub fn new(path: &str) -> Self {
        Self {
            buffer: [0u8; N],
            fd: File::create(path).unwrap_or_else(|e| {
                panic!("failed to create file {}: {}", path, e);
            }),
        }
    }

    pub fn flush(&mut self) {
        self.fd.flush().unwrap();
    }

    pub fn write_frame(&mut self, msg: GpsQzssLnavFrame) {
        self.write_subframe(*msg.eph_1());
        self.write_subframe(*msg.eph_2());
        self.write_subframe(*msg.eph_3());
        self.write_subframe(*msg.alm_4());
        self.write_subframe(*msg.alm_5());
    }

    pub fn write_subframe(&mut self, frame: GpsQzssLnavSubframe) {
        let size = frame.encode(&mut self.buffer).unwrap_or_else(|e| {
            panic!("failed to encode GPS frame: {}", e);
        });

        self.fd.write_all(&self.buffer[..size]).unwrap_or_else(|e| {
            panic!("failed to write GPS frame into file: {}", e);
        });
    }
}

/// Reconstructs 6 hours of
/// - GPS bursts for G15, G24 and G31
/// - isolates EPH-1, EPH-2, EPH-3, ALM-4 and ALM-5 as well
/// - QZSS bursts for J02 and J03
/// - isolates EPH-1, EPH-2, EPH-3, ALM-4 and ALM-5 as well
/// using ESBC00DNK_R_20201770000_01D_MN.rnx.gz (profesionnal file)
/// and dumps them to individual binary files.
pub fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    // full RINEX path
    let fullpath = "data/NAV/V4/BRD400DLR_S_20241310000_01D_MN.rnx.gz";

    // isolated buffers
    let mut g15_buf = FileBuffer::<1024>::new("data/GPS/brd400_g15.bin");
    let mut g15_eph1_buf = FileBuffer::<128>::new("data/GPS/brd400_g15_eph1.bin");
    let mut g15_eph2_buf = FileBuffer::<128>::new("data/GPS/brd400_g15_eph2.bin");
    let mut g15_eph3_buf = FileBuffer::<128>::new("data/GPS/brd400_g15_eph3.bin");
    let mut g15_alm4_buf = FileBuffer::<128>::new("data/GPS/brd400_g15_alm4.bin");
    let mut g15_alm5_buf = FileBuffer::<128>::new("data/GPS/brd400_g15_alm5.bin");

    let mut g24_buf = FileBuffer::<1024>::new("data/GPS/brd400_g24.bin");
    let mut g24_eph1_buf = FileBuffer::<128>::new("data/GPS/brd400_g24_eph1.bin");
    let mut g24_eph2_buf = FileBuffer::<128>::new("data/GPS/brd400_g24_eph2.bin");
    let mut g24_eph3_buf = FileBuffer::<128>::new("data/GPS/brd400_g24_eph3.bin");
    let mut g24_alm4_buf = FileBuffer::<128>::new("data/GPS/brd400_g24_alm4.bin");
    let mut g24_alm5_buf = FileBuffer::<128>::new("data/GPS/brd400_g24_alm5.bin");

    let mut g31_buf = FileBuffer::<1024>::new("data/GPS/brd400_g31.bin");
    let mut g31_eph1_buf = FileBuffer::<128>::new("data/GPS/brd400_g31_eph1.bin");
    let mut g31_eph2_buf = FileBuffer::<128>::new("data/GPS/brd400_g31_eph2.bin");
    let mut g31_eph3_buf = FileBuffer::<128>::new("data/GPS/brd400_g31_eph3.bin");
    let mut g31_alm4_buf = FileBuffer::<128>::new("data/GPS/brd400_g31_alm4.bin");
    let mut g31_alm5_buf = FileBuffer::<128>::new("data/GPS/brd400_g31_alm5.bin");

    let mut j02_buf = FileBuffer::<1024>::new("data/QZSS/brd400_j02.bin");
    let mut j02_eph1_buf = FileBuffer::<128>::new("data/QZSS/brd400_j02_eph1.bin");
    let mut j02_eph2_buf = FileBuffer::<128>::new("data/QZSS/brd400_j02_eph2.bin");
    let mut j02_eph3_buf = FileBuffer::<128>::new("data/QZSS/brd400_j02_eph3.bin");
    let mut j02_alm4_buf = FileBuffer::<128>::new("data/QZSS/brd400_j02_alm4.bin");
    let mut j02_alm5_buf = FileBuffer::<128>::new("data/QZSS/brd400_j02_alm5.bin");

    let mut j03_buf = FileBuffer::<1024>::new("data/QZSS/brd400_j03.bin");
    let mut j03_eph1_buf = FileBuffer::<128>::new("data/QZSS/brd400_j03_eph1.bin");
    let mut j03_eph2_buf = FileBuffer::<128>::new("data/QZSS/brd400_j03_eph2.bin");
    let mut j03_eph3_buf = FileBuffer::<128>::new("data/QZSS/brd400_j03_eph3.bin");
    let mut j03_alm4_buf = FileBuffer::<128>::new("data/QZSS/brd400_j03_alm4.bin");
    let mut j03_alm5_buf = FileBuffer::<128>::new("data/QZSS/brd400_j03_alm5.bin");

    let rinex = Rinex::from_gzip_file(fullpath).unwrap();

    // design temporal arc
    let t0 = rinex.first_epoch().unwrap().to_time_scale(TimeScale::GPST) + 1.0 * Unit::Hour; // random, not aligned to GPS

    let arc = TimeSeries::inclusive(t0, t0 + 6.0 * Unit::Hour, 6.0 * Unit::Second);

    for message in
        rinex.to_gps_qzss_lnav_proto(arc, |(satellite, t_tx)| GpsQzssLnavCompositionOpts {
            satellite,
            epoch: t_tx,
            alert: false,
            anti_spoofing: false,
            tlm_1: GpsQzssLnavTelemetry {
                message: 1,
                integrity: false,
                reserved_bit: false,
            },
            tlm_2: GpsQzssLnavTelemetry {
                message: 2,
                integrity: false,
                reserved_bit: false,
            },
            tlm_3: GpsQzssLnavTelemetry {
                message: 3,
                integrity: false,
                reserved_bit: false,
            },
            tlm_4: GpsQzssLnavTelemetry {
                message: 4,
                integrity: false,
                reserved_bit: false,
            },
            tlm_5: GpsQzssLnavTelemetry {
                message: 5,
                integrity: false,
                reserved_bit: false,
            },
            alm_4: None,
            alm_5: None,
        })
    {
        info!("encoded {:?}", message);

        match (message.satellite.constellation, message.satellite.prn) {
            (Constellation::GPS, 15) => {
                g15_buf.write_frame(message.frame);
                g15_eph1_buf.write_subframe(*message.frame.eph_1());
                g15_eph2_buf.write_subframe(*message.frame.eph_2());
                g15_eph3_buf.write_subframe(*message.frame.eph_3());
                g15_alm4_buf.write_subframe(*message.frame.alm_4());
                g15_alm5_buf.write_subframe(*message.frame.alm_5());
            },
            (Constellation::GPS, 24) => {
                g24_buf.write_frame(message.frame);
                g24_eph1_buf.write_subframe(*message.frame.eph_1());
                g24_eph2_buf.write_subframe(*message.frame.eph_2());
                g24_eph3_buf.write_subframe(*message.frame.eph_3());
                g24_alm4_buf.write_subframe(*message.frame.alm_4());
                g24_alm5_buf.write_subframe(*message.frame.alm_5());
            },
            (Constellation::GPS, 31) => {
                g31_buf.write_frame(message.frame);
                g31_eph1_buf.write_subframe(*message.frame.eph_1());
                g31_eph2_buf.write_subframe(*message.frame.eph_2());
                g31_eph3_buf.write_subframe(*message.frame.eph_3());
                g31_alm4_buf.write_subframe(*message.frame.alm_4());
                g31_alm5_buf.write_subframe(*message.frame.alm_5());
            },
            (Constellation::QZSS, 2) => {
                j02_buf.write_frame(message.frame);
                j02_eph1_buf.write_subframe(*message.frame.eph_1());
                j02_eph2_buf.write_subframe(*message.frame.eph_2());
                j02_eph3_buf.write_subframe(*message.frame.eph_3());
                j02_alm4_buf.write_subframe(*message.frame.alm_4());
                j02_alm5_buf.write_subframe(*message.frame.alm_5());
            },
            (Constellation::QZSS, 3) => {
                j03_buf.write_frame(message.frame);
                j03_eph1_buf.write_subframe(*message.frame.eph_1());
                j03_eph2_buf.write_subframe(*message.frame.eph_2());
                j03_eph3_buf.write_subframe(*message.frame.eph_3());
                j03_alm4_buf.write_subframe(*message.frame.alm_4());
                j03_alm5_buf.write_subframe(*message.frame.alm_5());
            },
            _ => {},
        }
    }
    g15_buf.flush();
    g24_buf.flush();
    g31_buf.flush();
    j02_buf.flush();
    j03_buf.flush();
}
