use crate::{
    navigation::{HeaderFields, NavMessageType},
    prelude::{Constellation, Rinex, RinexType},
    tests::toolkit::{generic_rinex_test, sv_csv as sv_csv_parser, TimeFrame},
};

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

    assert_eq!(dut.satellite_iter().collect::<Vec<_>>(), sv);
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
            // TODO: add f64_eq verifications
            // assert_eq!(v, dut_v);
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
