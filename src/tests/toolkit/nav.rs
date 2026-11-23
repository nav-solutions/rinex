use crate::{
    navigation::{Ephemeris, HeaderFields, NavMessageType},
    prelude::{Constellation, Epoch, Rinex, RinexType, SV},
    tests::toolkit::{generic_rinex_test, sv_csv as sv_csv_parser, TimeFrame},
};

use std::f64::consts::PI;

use gnss_protos::gps::lnav::GpsQzssLnavSubframe1;

/// Panics on strict inequality between two [Ephemeris] objects
pub fn panic_eq_ephemeris(dut: &Ephemeris, model: &Ephemeris) {
    assert_eq!(dut.clock_bias, model.clock_bias, "clock bias mismatch");
    assert_eq!(dut.clock_drift, model.clock_drift, "clock drift mismatch");
    assert_eq!(
        dut.clock_drift_rate, model.clock_drift_rate,
        "clock drift rate mismatch"
    );

    for (k, v) in dut.orbits.iter() {
        let value = model.orbits.get(k).unwrap_or_else(|| {
            panic!("{} missing item", k);
        });
        assert_eq!(v, value, "{} invalid item value", k);
    }

    for (k, v) in model.orbits.iter() {
        if dut.orbits.get(k).is_none() {
            panic!("found unexpected item '{}'", k);
        }
    }
}

/// Smart and compelling comparison between two [Ephemeris] objects.
/// Verifies that the decoded [Ephemeris] is the original one,
/// within the resolution of the binary message.
pub fn assert_faithful_ephemeris(sv: SV, toc: Epoch, original: &Ephemeris, decoded: &Ephemeris) {
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
                    GpsQzssLnavSubframe1::default()
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

// use crate::navigation::KbModel;
// use crate::navigation::NgModel;
// use crate::prelude::{Constellation, Rinex};

// pub fn check_klobuchar_models(rinex: &Rinex, tuple: &[(Constellation, KbModel)]) {
//     let header = &rinex.header;
//     for (constell, model) in tuple {
//         let parsed = header.ionod_corrections.get(&constell);
//         assert!(parsed.is_some(), "missing KB Model for {}", constell);
//         let parsed = parsed.unwrap();
//         let parsed = parsed.as_klobuchar().unwrap();
//         assert_eq!(parsed, model);
//     }
// }

// pub fn check_nequick_g_models(rinex: &Rinex, tuple: &[(Constellation, NgModel)]) {
//     let header = &rinex.header;
//     for (constell, model) in tuple {
//         let parsed = header.ionod_corrections.get(&constell);
//         assert!(parsed.is_some(), "missing NG Model for {}", constell);
//         let parsed = parsed.unwrap();
//         let parsed = parsed.as_nequick_g().unwrap();
//         assert_eq!(parsed, model);
//     }
// }

pub fn generic_test(
    dut: &Rinex,
    version: &str,
    header_constellation: &str,
    time_frame: Option<TimeFrame>,
    sv_csv: &str,
    nb_ephemeris: usize,
) {
    assert!(dut.is_navigation_rinex());

    let _ = dut.header.nav.as_ref().expect("missing header definition!");
    let _ = dut.record.as_nav().unwrap();

    let mut sv = sv_csv_parser(sv_csv);
    sv.sort();

    generic_rinex_test(
        dut,
        version,
        Some(header_constellation),
        RinexType::NavigationData,
        time_frame,
    );

    assert_eq!(dut.sv_iter().collect::<Vec<_>>(), sv);
    assert_eq!(dut.nav_ephemeris_frames_iter().count(), nb_ephemeris);

    // EPH frames logic
    for (k, _) in dut.nav_ephemeris_frames_iter() {
        if k.sv.constellation.is_sbas() {
            let expected = [NavMessageType::LNAV, NavMessageType::SBAS];
            assert!(
                expected.contains(&k.msgtype),
                "parsed invalid SBAS message: \"{}\"",
                k.msgtype
            );
        }
        match k.sv.constellation {
            Constellation::GPS | Constellation::QZSS => {
                let expected = [
                    NavMessageType::LNAV,
                    NavMessageType::CNAV,
                    NavMessageType::CNV2,
                ];
                assert!(
                    expected.contains(&k.msgtype),
                    "parsed invalid GPS/QZSS message: \"{}\"",
                    k.msgtype,
                );
            },
            Constellation::Galileo => {
                let expected = [
                    NavMessageType::LNAV,
                    NavMessageType::FNAV,
                    NavMessageType::INAV,
                ];
                assert!(
                    expected.contains(&k.msgtype),
                    "parsed invalid Galileo message: \"{}\"",
                    k.msgtype,
                );
            },
            Constellation::BeiDou => {
                let expected = [
                    NavMessageType::D1,
                    NavMessageType::D2,
                    NavMessageType::CNV1,
                    NavMessageType::CNV2,
                    NavMessageType::CNV3,
                    NavMessageType::LNAV,
                ];
                assert!(
                    expected.contains(&k.msgtype),
                    "parsed invalid BeiDou message: \"{}\"",
                    k.msgtype,
                );
            },
            Constellation::Glonass => {
                let expected = [NavMessageType::LNAV, NavMessageType::FDMA];
                assert!(
                    expected.contains(&k.msgtype),
                    "parsed invalid Glonass message: \"{}\"",
                    k.msgtype,
                );
            },
            _ => {},
        }
    }
}

fn generic_header_comparison(dut: &HeaderFields, model: &HeaderFields) {
    for t_offset in dut.time_offsets.iter() {
        let mut found = false;
        for rhs_t_offset in model.time_offsets.iter() {
            if rhs_t_offset.rhs == t_offset.rhs && rhs_t_offset.lhs == t_offset.lhs {
                if rhs_t_offset.t_ref == t_offset.t_ref && rhs_t_offset.utc == t_offset.utc {
                    if rhs_t_offset.polynomial == t_offset.polynomial {
                        found = true;
                    }
                }
            }
        }

        if !found {
            panic!("Found expected time offset definition: {:?}", t_offset);
        }
    }

    for t_offset in model.time_offsets.iter() {
        let mut found = false;
        for lhs_t_offset in dut.time_offsets.iter() {
            if lhs_t_offset.rhs == t_offset.rhs && lhs_t_offset.lhs == t_offset.lhs {
                if lhs_t_offset.t_ref == t_offset.t_ref && lhs_t_offset.utc == t_offset.utc {
                    if lhs_t_offset.polynomial == t_offset.polynomial {
                        found = true;
                    }
                }
            }
        }

        if !found {
            panic!("Missing time offset definition: {:?}", t_offset);
        }
    }
}

pub fn generic_comparison(dut: &Rinex, model: &Rinex) {
    let dut_header = dut.header.nav.as_ref().expect("missing dut header!");
    let model_header = model.header.nav.as_ref().expect("missing model header!");
    generic_header_comparison(&dut_header, &model_header);

    let dut = dut.record.as_nav().unwrap();
    let model = model.record.as_nav().unwrap();

    for (k, v) in model.iter() {
        if let Some(dut_v) = dut.get(&k) {
            assert_eq!(v, dut_v, "frame differs at {:?}", k);
        } else {
            panic!("missing data at {:?}", k);
        }
    }

    for (k, _) in dut.iter() {
        if model.get(&k).is_none() {
            panic!("found invalid data at {:?}", k);
        }
    }
}
