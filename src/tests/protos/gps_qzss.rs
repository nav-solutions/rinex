use crate::{
    navigation::{Ephemeris, 
        GpsQzssEphemerisMessage, 
        GpsQzssEphCompositionOpts, 
        GpsQzssEphemerisItem,
        OrbitItem,
    },
    prelude::{Constellation, Epoch, Rinex, SV},
    tests::toolkit::nav::panic_eq_ephemeris,
};

use std::f64::consts::PI;

use gnss_protos::{GpsQzssFrame, GpsQzssTelemetry, Message, GPS_WORDS_PER_FRAME};

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

/// Verifies that the decoded [Ephemeris] is the original one,
/// within the resolution of the binary message.
fn assert_faithful(sv: SV, toc: Epoch, original: &Ephemeris, decoded: &Ephemeris) {
    // half LSB, in RINEX units (seconds, meters, radians)
    let half_lsb = |pow2: i32| 0.5 * 2.0_f64.powi(pow2);
    let half_lsb_sc = |pow2: i32| half_lsb(pow2) * PI;

    let close = |name: &str, model: f64, dut: f64, tolerance: f64| {
        assert!(
            (model - dut).abs() <= tolerance,
            "{} @ {}: {}: decoded {:e} expecting {:e} (tolerance {:e})",
            sv,
            toc,
            name,
            dut,
            model,
            tolerance
        );
    };

    close(
        "af0",
        original.clock_bias,
        decoded.clock_bias,
        half_lsb(-31),
    );
    close(
        "af1",
        original.clock_drift,
        decoded.clock_drift,
        half_lsb(-43),
    );
    close(
        "af2",
        original.clock_drift_rate,
        decoded.clock_drift_rate,
        half_lsb(-55),
    );

    for (key, model) in original.orbits.iter() {
        let dut = decoded
            .orbits
            .get(key)
            .unwrap_or_else(|| panic!("{} @ {}: missing {}", sv, toc, key));

        let (model_f64, dut_f64) = (model.as_f64(), dut.as_f64());

        match key.as_str() {
            "tgd" => close(key, model_f64, dut_f64, half_lsb(-31)),
            "crs" | "crc" => close(key, model_f64, dut_f64, half_lsb(-5)),
            "cuc" | "cus" | "cic" | "cis" => close(key, model_f64, dut_f64, half_lsb(-29)),
            "e" => close(key, model_f64, dut_f64, half_lsb(-33)),
            "sqrta" => close(key, model_f64, dut_f64, half_lsb(-19)),
            "m0" | "omega0" | "i0" | "omega" => close(key, model_f64, dut_f64, half_lsb_sc(-31)),
            "deltaN" | "omegaDot" | "idot" => close(key, model_f64, dut_f64, half_lsb_sc(-43)),
            "accuracy" => {
                // only the URA index is broadcast
                let index = |ura_m: f64| {
                    gnss_protos::GpsQzssFrame1::default()
                        .with_user_range_accuracy_m(ura_m)
                        .ura
                };
                assert_eq!(
                    index(model_f64),
                    index(dut_f64),
                    "{} @ {}: URA index",
                    sv,
                    toc
                );
            },
            "fitInt" => {
                // only a flag is broadcast
                let threshold = if sv.constellation == Constellation::QZSS {
                    2.0
                } else {
                    4.0
                };
                assert_eq!(
                    model_f64 > threshold,
                    dut_f64 > threshold,
                    "{} @ {}: fit interval flag",
                    sv,
                    toc
                );
            },
            "t_tm" => {
                // transmission time: same 30s frame (RINEX producers do not
                // agree on which subframe instant t_tm designates)
                let frame = |t_tm: f64| (t_tm / 30.0).floor();
                assert_eq!(
                    frame(model_f64),
                    frame(dut_f64),
                    "{} @ {}: t_tm (decoded {}, expecting {})",
                    sv,
                    toc,
                    dut_f64,
                    model_f64
                );
            },
            // integer fields, flags & identifiers: strict equality
            "toe" | "week" | "iode" | "iodc" | "health" | "l2Codes" | "l2p" => {
                assert_eq!(model, dut, "{} @ {}: {}", sv, toc, key);
            },
            _ => {
                // not part of the LNAV message: cannot be recovered
            },
        }
    }

    // only the fit interval may be introduced (not always described in RINEX V2)
    for key in decoded.orbits.keys() {
        if !original.orbits.contains_key(key) {
            assert_eq!(key, "fitInt", "{} @ {}: unexpected {}", sv, toc, key);
        }
    }
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

        assert_faithful(key.sv, key.epoch, eph, &decoded_eph);
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

    let message = eph
        .to_gps_qzss_eph_message(opts)
        .unwrap();

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
