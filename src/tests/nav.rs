use crate::{
    navigation::{NavFrameType, NavMessageType},
    prelude::{Constellation, Epoch, Rinex, TimeScale, SV},
    tests::toolkit::{generic_navigation_test, TimeFrame},
};

use hifitime::Unit;

use std::{path::PathBuf, str::FromStr};

#[test]
fn v2_amel0010_21g() {
    let test_resource = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/NAV/V2/amel0010.21g";

    let dut = Rinex::from_file(&test_resource).unwrap();

    generic_navigation_test(
        &dut,
        "2.11",
        "Glonass",
        Some(TimeFrame::from_erratic_csv(
            "2020-12-31T23:45:00 UTC,
            2021-01-01T11:15:00 UTC,
            2021-01-01T11:45:00 UTC,
            2021-01-01T16:15:00 UTC,
            2021-01-01T16:15:00 UTC,
            2021-01-01T16:15:00 UTC",
        )),
        "R01, R02, R03, R04, R05, R07",
        6,
    );

    let mut num_tests = 0;

    for (k, eph) in dut.nav_ephemeris_frames_iter() {
        assert_eq!(k.frmtype, NavFrameType::Ephemeris);
        assert_eq!(k.msgtype, NavMessageType::LNAV);
        assert_eq!(k.sv.constellation, Constellation::Glonass);

        match k.sv.prn {
            1 => {
                assert_eq!(eph.sv_clock(), (7.282570004460E-5, 0.0, 7.380000000000E+04));
                //TODO
                //assert_eq!(v.sv_position((-1.488799804690E+03, 1.292880712890E+04, 2.193169775390E+04)))

                assert!(eph.get_orbit_f64("ageOp").is_none());
                assert_eq!(eph.glonass_freq_channel(), Some(1));

                num_tests += 1;
            },
            2 => {
                assert_eq!(eph.clock_bias, 4.610531032090E-04);
                assert_eq!(eph.clock_drift, 1.818989403550E-12);
                assert_eq!(eph.clock_drift_rate, 4.245000000000E+04);
                // assert_eq!(eph.get_orbit_f64("channel"), Some(-4.0));
                assert!(eph.get_orbit_f64("ageOp").is_none());

                num_tests += 1;
                //TODO
                //assert_eq!(eph.sv_position((
                //                 assert_eq!(posx.as_f64(), Some(-8.955041992190E+03));
                //                 assert_eq!(posy.as_f64(), Some(-1.834875292970E+04));
                //                 assert_eq!(posz.as_f64(), Some(1.536620703130E+04));
            },
            3 => {
                assert_eq!(eph.clock_bias, 2.838205546140E-05);
                assert_eq!(eph.clock_drift, 0.0);
                assert_eq!(eph.clock_drift_rate, 4.680000000000E+04);
                //assert_eq!(eph.get_orbit_f64("health"), Some(0.0));
                //assert_eq!(eph.get_orbit_f64("channel"), Some(5.0));
                //assert_eq!(eph.get_orbit_f64("ageOp"), Some(0.0));
                //                 assert_eq!(posx.as_f64(), Some(1.502522949220E+04));
                //                 assert_eq!(posy.as_f64(), Some(-1.458877050780E+04));
                //                 assert_eq!(posz.as_f64(), Some(1.455863281250E+04));
                num_tests += 1;
            },
            4 => {
                assert_eq!(eph.clock_bias, 6.817653775220E-05);
                assert_eq!(eph.clock_drift, 1.818989403550E-12);
                assert_eq!(eph.clock_drift_rate, 4.680000000000E+04);
                //assert_eq!(eph.get_orbit_f64("ageOp"), Some(0.0));
                // assert_eq!(eph.get_orbit_f64("channel"), Some(6.0));
                //assert_eq!(eph.get_orbit_f64("health"), Some(0.0));
                //                 assert_eq!(posx.as_f64(), Some(-1.688173828130E+03));
                //                 assert_eq!(posy.as_f64(), Some(-1.107156738280E+04));
                //                 assert_eq!(posz.as_f64(), Some(2.293745361330E+04));
                num_tests += 1;
            },
            5 => {
                assert_eq!(eph.clock_bias, 6.396882236000E-05);
                assert_eq!(eph.clock_drift, 9.094947017730E-13);
                assert_eq!(eph.clock_drift_rate, 8.007000000000E+04);
                //assert_eq!(eph.get_orbit_f64("ageOp"), Some(0.0));
                //assert_eq!(eph.get_orbit_f64("channel"), Some(1.0));
                //assert_eq!(eph.get_orbit_f64("health"), Some(0.0));
                //                 assert_eq!(posx.as_f64(), Some(-1.754308935550E+04));
                //                 assert_eq!(posy.as_f64(), Some(-1.481773437500E+03));
                //                 assert_eq!(posz.as_f64(), Some(1.847386083980E+04));
                num_tests += 1;
            },
            7 => {
                assert_eq!(eph.clock_bias, -4.201009869580E-05);
                assert_eq!(eph.clock_drift, 0.0);
                assert_eq!(eph.clock_drift_rate, 2.88E4);
                assert!(eph.get_orbit_f64("ageOp").is_none());
                //assert_eq!(eph.get_orbit_f64("channel"), Some(5.0));
                //assert_eq!(eph.get_orbit_f64("health"), Some(0.0));
                //                 assert_eq!(posx.as_f64(), Some(1.817068505860E+04));
                //                 assert_eq!(posy.as_f64(), Some(1.594814404300E+04));
                //                 assert_eq!(posz.as_f64(), Some(8.090271484380E+03));
                num_tests += 1;
            },
            prn => panic!("invalid SV: R{}", prn),
        }
    }

    assert_eq!(num_tests, 6);
}

#[test]
#[cfg(feature = "flate2")]
fn v2_cbw10010_21n() {
    let data = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/NAV/V2/cbw10010.21n.gz";

    let dut = Rinex::from_gzip_file(&data).unwrap();

    generic_navigation_test(
        &dut,
        "2.11",
        "GPS",
        None,
        "G01, G07, G08, G04, G19, G10, G15, G20, G18, G31, G03, G06, G27, G09, G11, G13, G30, G12, G14, G17, G23, G24, G19, G21, G22, G28, G32, G25, G02, G03, G06, G12, G17, G22, G26, G05, G16, G29, G14",
        187,
    );

    let t0 = Epoch::from_str("2020-12-31T23:59:44 GPST").unwrap();
    let t1 = Epoch::from_str("2021-01-02T00:00:00 GPST").unwrap();

    let mut tests_passed = 0;

    for (k, eph) in dut.nav_ephemeris_frames_iter() {
        assert_eq!(k.msgtype, NavMessageType::LNAV, "Legacy NAV file");
        assert_eq!(k.sv.constellation, Constellation::GPS, "GPS NAV file");

        if k.epoch == t0 {
            if k.sv.prn == 7 {
                assert_eq!(eph.clock_bias, 4.204921424390E-6);
                assert_eq!(eph.clock_drift, 1.477928890380E-11);
                assert_eq!(eph.clock_drift_rate, 0.0);

                for (field, value) in [
                    ("crs", Some(-1.509375000000E1)),
                    ("deltaN", Some(5.043781392540E-9)),
                    ("m0", Some(-1.673144695710)),
                    ("cuc", Some(-8.475035429000E-7)),
                    ("e", Some(1.431132073050E-2)),
                    ("cus", Some(5.507841706280E-6)),
                    ("sqrta", Some(5.153606595990E3)),
                    ("toe", Some(4.319840000000E5)),
                    ("cic", Some(2.216547727580E-7)),
                    ("omega0", Some(2.333424778860)),
                    ("cis", Some(-8.009374141690E-8)),
                    ("i0", Some(9.519533967710E-1)),
                    ("crc", Some(2.626562500000E2)),
                    ("omega", Some(-2.356931900380)),
                    ("omegaDot", Some(-8.034263032640E-9)),
                    ("idot", Some(-1.592923432050E-10)),
                    ("l2Codes", Some(1.000000000000)),
                    ("tgd", Some(-1.117587089540E-8)),
                    ("t_tm", Some(4.283760000000E5)),
                ] {
                    let orbit_value = eph.get_orbit_f64(field);
                    assert_eq!(
                        orbit_value, value,
                        "parsed wrong \"{}\" value for G07 T0",
                        field
                    );
                }

                assert_eq!(eph.get_week(), Some(2138));
                assert!(
                    eph.get_orbit_f64("fitInt").is_none(),
                    "parsed fitInt unexpectedly"
                );
                tests_passed += 1;
            }
        } else if k.epoch == t1 {
            if k.sv.prn == 30 {
                assert_eq!(eph.clock_bias, -3.621461801230E-04);
                assert_eq!(eph.clock_drift, -6.139089236970E-12);
                assert_eq!(eph.clock_drift_rate, 0.000000000000);

                for (field, value) in vec![
                    ("iode", Some(8.500000000000E1)),
                    ("crs", Some(-7.500000000000)),
                    ("deltaN", Some(5.476656696160E-9)),
                    ("m0", Some(-1.649762378650)),
                    ("cuc", Some(-6.072223186490E-7)),
                    ("e", Some(4.747916595080E-3)),
                    ("cus", Some(5.392357707020E-6)),
                    ("sqrta", Some(5.153756387710E+3)),
                    ("toe", Some(5.184000000000E+5)),
                    ("cic", Some(7.636845111850E-8)),
                    ("omega0", Some(2.352085289360E+00)),
                    ("cis", Some(-2.421438694000E-8)),
                    ("i0", Some(9.371909002540E-1)),
                    ("crc", Some(2.614687500000E+2)),
                    ("omega", Some(-2.846234079630)),
                    ("omegaDot", Some(-8.435351366240E-9)),
                    ("idot", Some(-7.000291590240E-11)),
                    ("l2Codes", Some(1.000000000000)),
                    ("tgd", Some(3.725290298460E-9)),
                    ("iodc", Some(8.500000000000E1)),
                    ("t_tm", Some(5.146680000000E5)),
                ] {
                    let orbit = eph.get_orbit_f64(field);
                    assert_eq!(orbit, value, "parsed wrong \"{}\" value for G30 T1", field);
                }
                assert_eq!(eph.get_week(), Some(2138));
                assert!(
                    eph.get_orbit_f64("fitInt").is_none(),
                    "parsed fitInt unexpectedly"
                );
                tests_passed += 1;
            }
        }
    }

    assert_eq!(tests_passed, 2);
}

#[test]
fn v3_amel00nld_r_2021() {
    let test_resource =
        env!("CARGO_MANIFEST_DIR").to_owned() + "/data/NAV/V3/AMEL00NLD_R_20210010000_01D_MN.rnx";

    let rinex = Rinex::from_file(&test_resource).unwrap();

    assert!(rinex.is_navigation_rinex());
    assert!(rinex.header.obs.is_none());
    assert!(rinex.header.meteo.is_none());

    let c05 = SV::from_str("C05").unwrap();
    let c21 = SV::from_str("C21").unwrap();
    let e01 = SV::from_str("E01").unwrap();
    let r07 = SV::from_str("R07").unwrap();
    let r19 = SV::from_str("R19").unwrap();

    let mut num_tests = 0;

    for (k, eph) in rinex.nav_ephemeris_frames_iter() {
        assert_eq!(k.msgtype, NavMessageType::LNAV);

        if k.sv == c05 {
            assert_eq!(eph.clock_bias, -0.426337239332e-03);
            assert_eq!(eph.clock_drift, -0.752518047875e-10);
            assert_eq!(eph.clock_drift_rate, 0.0);

            assert_eq!(eph.get_orbit_f64("aode"), Some(0.100000000000e+01));
            assert_eq!(eph.get_orbit_f64("crs"), Some(0.118906250000e+02));

        //                         let m0 = data.get("m0").unwrap();
        //                         assert_eq!(m0.as_f64(), Some(-0.255139531119e+01));
        //                         let i0 = data.get("i0").unwrap();
        //                         assert_eq!(i0.as_f64(), Some(0.607169709798e-01));
        //                         let acc = data.get("svAccuracy").unwrap();
        //                         assert_eq!(acc.as_f64(), Some(0.200000000000e+01));
        //                         let sath1 = data.get("satH1").unwrap();
        //                         assert_eq!(sath1.as_f64(), Some(0.0));
        //                         let tgd1 = data.get("tgd1b1b3").unwrap();
        //                         assert_eq!(tgd1.as_f64(), Some(-0.599999994133e-09));
        } else if k.sv == c21 {
            assert_eq!(eph.clock_bias, -0.775156309828e-03);
            assert_eq!(eph.clock_drift, -0.144968481663e-10);
            assert_eq!(eph.clock_drift_rate, 0.000000000000e+0);
        //                         let aode = data.get("aode").unwrap();
        //                         assert_eq!(aode.as_f64(), Some(0.100000000000e+01));
        //                         let crs = data.get("crs").unwrap();
        //                         assert_eq!(crs.as_f64(), Some(-0.793437500000e+02));
        //                         let m0 = data.get("m0").unwrap();
        //                         assert_eq!(m0.as_f64(), Some(0.206213212749e+01));
        //                         let i0 = data.get("i0").unwrap();
        //                         assert_eq!(i0.as_f64(), Some(0.964491154768e+00));
        //                         let acc = data.get("svAccuracy").unwrap();
        //                         assert_eq!(acc.as_f64(), Some(0.200000000000e+01));
        //                         let sath1 = data.get("satH1").unwrap();
        //                         assert_eq!(sath1.as_f64(), Some(0.0));
        //                         let tgd1 = data.get("tgd1b1b3").unwrap();
        //                         assert_eq!(tgd1.as_f64(), Some(0.143000002950e-07));
        } else if k.sv == r19 {
            assert_eq!(eph.clock_bias, -0.126023776829e-03);
            assert_eq!(eph.clock_drift, -0.909494701773e-12);
            assert_eq!(eph.clock_drift_rate, 0.0);
        //                         let pos = data.get("satPosX").unwrap();
        //                         assert_eq!(pos.as_f64(), Some(0.783916601562e+04));
        //                         let pos = data.get("satPosY").unwrap();
        //                         assert_eq!(pos.as_f64(), Some(-0.216949155273e+05));
        //                         let pos = data.get("satPosZ").unwrap();
        //                         assert_eq!(pos.as_f64(), Some(0.109021518555e+05));
        } else if k.sv == r07 {
            assert_eq!(eph.clock_bias, -0.420100986958E-04);
            assert_eq!(eph.clock_drift, 0.0);
            assert_eq!(eph.clock_drift_rate, 0.342000000000e+05);
        //                         let pos = data.get("satPosX").unwrap();
        //                         assert_eq!(pos.as_f64(), Some(0.124900639648e+05));
        //                         let pos = data.get("satPosY").unwrap();
        //                         assert_eq!(pos.as_f64(), Some(0.595546582031e+04));
        //                         let pos = data.get("satPosZ").unwrap();
        //                         assert_eq!(pos.as_f64(), Some(0.214479208984e+05));
        } else if k.sv == e01 {
            assert_eq!(eph.clock_bias, -0.101553811692e-02);
            assert_eq!(eph.clock_drift, -0.804334376880e-11);
            assert_eq!(eph.clock_drift_rate, 0.0);
        //                         let iodnav = data.get("iodnav").unwrap();
        //                         assert_eq!(iodnav.as_f64(), Some(0.130000000000e+02));
        //                         let crs = data.get("crs").unwrap();
        //                         assert_eq!(crs.as_f64(), Some(0.435937500000e+02));
        //                         let cis = data.get("cis").unwrap();
        //                         assert_eq!(cis.as_f64(), Some(0.409781932831e-07));
        //                         let omega_dot = data.get("omegaDot").unwrap();
        //                         assert_eq!(omega_dot.as_f64(), Some(-0.518200156545e-08));
        //                         let idot = data.get("idot").unwrap();
        //                         assert_eq!(idot.as_f64(), Some(-0.595381942905e-09));
        //                         let sisa = data.get("sisa").unwrap();
        //                         assert_eq!(sisa.as_f64(), Some(0.312000000000e+01));
        //                         let bgd = data.get("bgdE5aE1").unwrap();
        //                         assert_eq!(bgd.as_f64(), Some(0.232830643654e-09));
        } else if k.sv == e01 {
            assert_eq!(eph.clock_bias, -0.382520200219e-03);
            assert_eq!(eph.clock_drift, -0.422062385041e-11);
            assert_eq!(eph.clock_drift_rate, 0.0);
            //                         let iodnav = data.get("iodnav").unwrap();
            //                         assert_eq!(iodnav.as_f64(), Some(0.460000000000e+02));
            //                         let crs = data.get("crs").unwrap();
            //                         assert_eq!(crs.as_f64(), Some(-0.103750000000e+02));
            //                         let cis = data.get("cis").unwrap();
            //                         assert_eq!(cis.as_f64(), Some(0.745058059692e-08));
            //                         let omega_dot = data.get("omegaDot").unwrap();
            //                         assert_eq!(omega_dot.as_f64(), Some(-0.539986778331e-08));
            //                         let idot = data.get("idot").unwrap();
            //                         assert_eq!(idot.as_f64(), Some(0.701814947695e-09));
            //                         let sisa = data.get("sisa").unwrap();
            //                         assert_eq!(sisa.as_f64(), Some(0.312000000000e+01));
            //                         let bgd = data.get("bgdE5aE1").unwrap();
            //                         assert_eq!(bgd.as_f64(), Some(0.302679836750e-08));
        }
        num_tests += 1;
    }
    assert_eq!(num_tests, 6);
}

// #[test]
// #[cfg(feature = "flate2")]
// fn v4_kms300dnk_r_202215910() {

//             } else if let Some(fr) = fr.as_ion() {
//                 ion_count += 1; // ION test
//                 let (_msg, _sv, model) = fr;
//                 if let Some(model) = model.as_klobuchar() {
//                     let e0 = Epoch::from_str("2022-06-08T09:59:48 GPST").unwrap();
//                     let e1 = Epoch::from_str("2022-06-08T09:59:50 BDT").unwrap();
//                     if *e == e0 {
//                         assert_eq!(
//                             model.alpha,
//                             (
//                                 1.024454832077E-08,
//                                 2.235174179077E-08,
//                                 -5.960464477539E-08,
//                                 -1.192092895508E-07
//                             )
//                         );
//                         assert_eq!(
//                             model.beta,
//                             (
//                                 9.625600000000E+04,
//                                 1.310720000000E+05,
//                                 -6.553600000000E+04,
//                                 -5.898240000000E+05
//                             )
//                         );
//                     } else if *e == e1 {
//                         assert_eq!(
//                             model.alpha,
//                             (
//                                 2.142041921616E-08,
//                                 1.192092895508E-07,
//                                 -1.013278961182E-06,
//                                 1.549720764160E-06
//                             )
//                         );
//                         assert_eq!(
//                             model.beta,
//                             (
//                                 1.208320000000E+05,
//                                 1.474560000000E+05,
//                                 -1.310720000000E+05,
//                                 -6.553600000000E+04
//                             )
//                         );
//                     } else {
//                         panic!("misplaced ION message {:?} @ {}", model, e)
//                     }
//                     assert_eq!(model.region, KbRegionCode::WideArea);
//                 } else if let Some(model) = model.as_nequick_g() {
//                     assert_eq!(*e, Epoch::from_str("2022-06-08T09:59:57 GST").unwrap());
//                     assert_eq!(model.region, NgRegionFlags::empty());
//                 }
//             }
//         }
//     }
// }

#[test]
#[cfg(feature = "flate2")]
fn v3_brdc00gop_r_2021_gz() {
    let test_resource = env!("CARGO_MANIFEST_DIR").to_owned()
        + "/data/NAV/V3/BRDC00GOP_R_20210010000_01D_MN.rnx.gz";

    let dut = Rinex::from_gzip_file(&test_resource).unwrap();

    generic_navigation_test(
        &dut,
        "3.04",
        "MIX",
        Some(TimeFrame::from_erratic_csv(
            "
            2021-01-01T00:00:00 BDT,
            2021-01-01T01:28:00 GPST,
            2021-01-01T07:15:00 UTC,
            2021-01-01T08:20:00 GST",
        )),
        "C01, E03, R10, S36",
        4,
    );
}

#[test]
#[cfg(feature = "flate2")]
fn v3_esbc00dnk_r2020() {
    let test_resource = env!("CARGO_MANIFEST_DIR").to_owned()
        + "/data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz";

    let dut = Rinex::from_gzip_file(&test_resource).unwrap();

    generic_navigation_test(
        &dut,
        "3.05",
        "MIX",
        None,
        "
        C05, C06, C07, C08, C09, C10, C11, C12, C13, C14,
        C16, C19, C20, C21, C22, C23, C24, C25, C26, C27,
        C28, C29, C30, C32, C33, C34, C35, C36, C37,
        E01, E02, E03, E04, E05, E07, E08, E09, E11, E12,
        E13, E14, E15, E18, E19, E21, E24, E25, E26, E27,
        E30, E31, E33, E36,
        G01, G02, G03, G04, G05, G06, G07, G08, G09, G10, G11,
        G12, G13, G14, G15, G16, G17, G18, G19, G20, G21, G22,
        G24, G25, G26, G27, G28, G29, G30, G31, G32,
        J01, J02, J03,
        R01, R02, R03, R04, R05, R06, R07, R08, R09, R10,
        R11, R12, R13, R14, R15, R16, R17, R18, R19, R20, R21, R23, R24,
        S23, S25, S26, S36, S44",
        4092,
    );

    let mut tests_passed = 0;

    let t0 = Epoch::from_str("2020-06-24T22:00:00 GPST").unwrap();

    let g30 = SV::from_str("G30").unwrap();

    for (k, eph) in dut.nav_ephemeris_frames_iter() {
        if k.epoch == t0 && k.sv == g30 {
            assert_eq!(
                eph.sv_clock(),
                (-2.486067824066e-04, -7.844391802792e-12, 0.000000000000e+00)
            );
            tests_passed += 1;
        }
    }

    assert_eq!(tests_passed, 1);
}

//     check_nequick_g_models(
//         &rinex,
//         &[(
//             Constellation::Galileo,
//             NgModel {
//                 a: (6.6250e+01, -1.6406e-01, -2.4719e-03),
//                 region: NgRegionFlags::empty(),
//             },
//         )],
//     );

//     check_klobuchar_models(
//         &rinex,
//         &[
//             (
//                 Constellation::GPS,
//                 KbModel {
//                     alpha: (7.4506e-09, -1.4901e-08, -5.9605e-08, 1.1921e-07),
//                     beta: (9.0112e04, -6.5536e04, -1.3107e05, 4.5875e05),
//                     region: KbRegionCode::WideArea,
//                 },
//             ),
//             (
//                 Constellation::QZSS,
//                 KbModel {
//                     alpha: (8.3819e-09, -2.9802e-08, -2.3842e-07, -1.1921e-07),
//                     beta: (6.9632e+04, -1.6384e+05, 5.8982e+05, 4.1288e+06),
//                     region: KbRegionCode::JapanArea,
//                 },
//             ),
//             (
//                 Constellation::BeiDou,
//                 KbModel {
//                     alpha: (1.1180e-08, 2.9800e-08, -4.1720e-07, 6.5570e-07),
//                     beta: (1.4130e+05, -5.2430e+05, 1.6380e+06, -4.5880e+05),
//                     region: KbRegionCode::WideArea,
//                 },
//             ),
//             (
//                 Constellation::IRNSS,
//                 KbModel {
//                     alpha: (2.7940e-08, 3.4273e-07, -7.5102e-06, 7.5102e-06),
//                     beta: (1.2698e+05, 7.7005e+05, -8.3231e+06, 8.3231e+06),
//                     region: KbRegionCode::WideArea,
//                 },
//             ),
//         ],
//     );

//     let record = record.unwrap();
//     let mut epochs: Vec<Epoch> = vec![
//         Epoch::from_str("2021-01-01T00:00:00 BDT").unwrap(),
//         Epoch::from_str("2021-01-01T07:15:00 UTC").unwrap(),
//         Epoch::from_str("2021-01-01T01:28:00 GPST").unwrap(),
//         Epoch::from_str("2021-01-01T08:20:00 GST").unwrap(),
//     ];
//     epochs.sort(); // for comparison purposes

//     assert!(
//         rinex.epoch_iter().sorted().eq(epochs.clone()),
//         "parsed wrong epoch content.\nExpecting {:?}\nGot {:?}",
//         epochs.clone(),
//         rinex.epoch_iter().collect::<Vec<Epoch>>(),
//     );

//     let mut vehicles: Vec<SV> = vec![
//         SV::from_str("E03").unwrap(),
//         SV::from_str("C01").unwrap(),
//         SV::from_str("R10").unwrap(),
//         SV::from_str("S36").unwrap(),
//     ];
//     vehicles.sort(); // for comparison purposes
//     assert!(
//         rinex.sv_iter().sorted().eq(vehicles),
//         "parsed wrong sv content"
//     );

//     for (_, frames) in record {
//         for fr in frames {
//             let fr = fr.as_eph();
//             assert!(fr.is_some(), "only ephemeris frames expected here");
//             let (msg, _sv, _data) = fr.unwrap();
//             assert!(msg == NavMessageType::LNAV, "only lnav frame expected here");
//         }
//     }
// }

#[test]
#[cfg(feature = "flate2")]
fn nav_v4_kms300dnk_r2022() {
    let test_resource = env!("CARGO_MANIFEST_DIR").to_owned()
        + "/data/NAV/V4/KMS300DNK_R_20221591000_01H_MN.rnx.gz";

    let dut = Rinex::from_gzip_file(&test_resource).unwrap();

    generic_navigation_test(
        &dut,
        "4.00",
        "MIX",
        None,
        "G02, G04, G05, G07, G08, G09, 
        G10, G11, G12, G13,
        G15, G16, G18, G20, G22, G23, 
        G25, G26, G27, G29, G31,
        E01, E03, E05, E07, E08, E09, E10,
        E11, E12, E13, E14, E15, E21, E24,
        E25, E26, E31, E33,
        R03, R04, R05, R10, R11, R12, R13,
        R20, R21, R23, 
        C05, C08, C10, C13, C14, C20, C21, C24, C26, C27,
        C28, C29, C30, C32, C33, C35, C36, C38, C41, C42, C45, C46, C60,
        J04,
        S48, S36, S26, S44, S23, S25, S27, S26, S28",
        357,
    );

    let t0 = Epoch::from_str("2022-06-10T19:56:48 GPST").unwrap();
    let t1 = Epoch::from_str("2022-06-08T00:00:00 GST").unwrap();
    let t2 = Epoch::from_str("2022-06-08T09:50:00 GST").unwrap();
    let t_11_00_00_gpst = Epoch::from_str("2022-06-08T11:00:00 GPST").unwrap();
    let t_last = Epoch::from_str("2022-06-10T19:56:48 GPST").unwrap();

    let g26 = SV::from_str("G26").unwrap();
    let e01 = SV::from_str("E01").unwrap();
    let e14 = SV::from_str("E14").unwrap();
    let j04 = SV::from_str("J04").unwrap();

    // test EPH frames
    let mut tests_passed = 0;

    for (k, v) in dut.nav_ephemeris_frames_iter() {
        assert_eq!(k.frmtype, NavFrameType::Ephemeris);

        // test first epoch
        if k.epoch == t0 {
        } else if k.epoch == t_11_00_00_gpst {
            assert_eq!(k.sv, j04);
            assert_eq!(k.msgtype, NavMessageType::LNAV);
            assert_eq!(v.clock_bias, 1.080981455743E-04);
            assert_eq!(v.clock_drift, 3.751665644813E-12);
            assert_eq!(v.clock_drift_rate, 0.0);
            tests_passed += 1;
        } else if k.epoch == t2 {
            if k.sv == e14 {
                if k.msgtype == NavMessageType::INAV {
                    assert_eq!(v.clock_bias, -1.813994604163E-03);
                    assert_eq!(v.clock_drift, 1.104183411371E-11);
                    assert_eq!(v.clock_drift_rate, 0.000000000000E+00);
                    tests_passed += 1;
                } else if k.msgtype == NavMessageType::FNAV {
                    assert_eq!(v.clock_bias, -1.813993556425E-03);
                    assert_eq!(v.clock_drift, 1.104183411371E-11);
                    assert_eq!(v.clock_drift_rate, 0.000000000000E+00);
                    tests_passed += 1;
                }
            }
        } else if k.epoch == t_last {
        }
    }

    assert_eq!(tests_passed, 3);

    // TODO test ION frames

    // test STO frames
    let mut tests_passed = 0;

    for (k, v) in dut.nav_system_time_frames_iter() {
        if k.epoch == t0 {
            if k.sv == g26 {
                assert_eq!(k.msgtype, NavMessageType::LNAV);
                assert_eq!(k.frmtype, NavFrameType::SystemTimeOffset);

                assert_eq!(v.lhs, TimeScale::GPST);
                assert_eq!(v.rhs, TimeScale::UTC);
                // TODO assert_eq!(v.utc, "UTC(USNO)");

                let (seconds, drift) = (v.polynomial.0, v.polynomial.1);

                assert_eq!(v.polynomial.2, 0.0);
                tests_passed += 1;
            }
        } else if k.epoch == t1 {
            if k.sv == e01 {
                assert_eq!(k.msgtype, NavMessageType::IFNV);
                assert_eq!(k.frmtype, NavFrameType::SystemTimeOffset);

                assert_eq!(v.lhs, TimeScale::GST);
                assert_eq!(v.rhs, TimeScale::GPST);
                // TODO assert_eq!(v.utc, "");
                // TODO assert_eq!(v.t_tm, 0);

                let (seconds, drift) = (v.polynomial.0, v.polynomial.1);

                assert!((seconds - 3.201421350241E-09).abs() < 1E-12);
                assert!((drift - -4.440892098501E-15).abs() < 1E-12);

                assert_eq!(v.polynomial.2, 0.0);
                tests_passed += 1;
            }
        }
    }

    assert_eq!(tests_passed, 2);

    // NO EOP frames
    let mut tests = 0;

    for (_, _) in dut.nav_earth_orientation_frames_iter() {
        tests += 1;
    }

    assert_eq!(tests, 0);
}

#[test]
#[cfg(feature = "nav")]
#[cfg(feature = "flate2")]
fn v4_brd400dlr_s2023() {
    let path = PathBuf::new()
        .join(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("NAV")
        .join("V4")
        .join("BRD400DLR_S_20230710000_01D_MN.rnx.gz");

    let path = path.to_string_lossy().to_string();
    let rinex = Rinex::from_gzip_file(&path).unwrap();

    let t_03_12_00_00_00_gpst = Epoch::from_str("2023-03-12T00:00:00 GPST").unwrap();
    let t_03_12_00_00_00_bdt = Epoch::from_str("2023-03-12T00:00:00 BDT").unwrap();

    let t_03_12_01_30_00_gpst = Epoch::from_str("2023-03-12T01:30:00 GPST").unwrap();

    let t_03_12_02_00_00_gpst = Epoch::from_str("2023-03-12T02:00:00 GPST").unwrap();

    let g01 = SV::from_str("G01").unwrap();
    let c19 = SV::from_str("C19").unwrap();
    let j04 = SV::from_str("J04").unwrap();
    // TODO (IRNSS NAV) let i09 = SV::from_str("I09").unwrap();

    let mut tests_passed = 0;

    for (k, eph) in rinex.nav_ephemeris_frames_iter() {
        if k.sv == g01 {
            assert!(
                (k.msgtype == NavMessageType::LNAV) || (k.msgtype == NavMessageType::CNAV),
                "bad ephemeris message {} for G01 {}",
                k.msgtype,
                k.epoch
            );

            if k.epoch == t_03_12_00_00_00_gpst {
                assert_eq!(k.msgtype, NavMessageType::LNAV);
                assert_eq!(eph.clock_bias, 2.037500962615e-04);
                assert_eq!(eph.clock_drift, -3.865352482535e-12);
                assert_eq!(eph.clock_drift_rate, 0.000000000000e+00);
                tests_passed += 1;
            } else if k.epoch == t_03_12_02_00_00_gpst {
                assert_eq!(k.msgtype, NavMessageType::LNAV);
                assert_eq!(eph.clock_bias, 2.037221565843e-04);
                assert_eq!(eph.clock_drift, -3.865352482535e-12);
                assert_eq!(eph.clock_drift_rate, 0.000000000000e+00);
                tests_passed += 1;
            } else if k.epoch == t_03_12_01_30_00_gpst {
                assert_eq!(k.msgtype, NavMessageType::CNAV);
                assert_eq!(eph.clock_bias, 2.037292579189e-04);
                assert_eq!(eph.clock_drift, -3.829825345747e-12);
                assert_eq!(eph.clock_drift_rate, 0.000000000000e+00);
                tests_passed += 1;
            }
        } else if k.sv == c19 {
            assert!(
                (k.msgtype == NavMessageType::D1)
                    || (k.msgtype == NavMessageType::CNV1)
                    || (k.msgtype == NavMessageType::CNV2),
                "bad ephemeris message {} for C19 {}",
                k.msgtype,
                k.epoch
            );

            if k.epoch == t_03_12_00_00_00_bdt {
                if k.msgtype == NavMessageType::D1 {
                    assert_eq!(eph.clock_bias, -8.956108940765e-04);
                    assert_eq!(eph.clock_drift, -9.041656312547e-13);
                    assert_eq!(eph.clock_drift_rate, 0.000000000000e+00);
                    tests_passed += 1;
                } else if k.msgtype == NavMessageType::CNV2 {
                    assert_eq!(eph.clock_bias, -8.956108940765e-04);
                    assert_eq!(eph.clock_drift, -9.041656312547e-13);
                    assert_eq!(eph.clock_drift_rate, 0.000000000000e+00);
                    tests_passed += 1;
                }
            }
        } else if k.sv == j04 {
            assert!(
                (k.msgtype == NavMessageType::LNAV)
                    || (k.msgtype == NavMessageType::CNAV)
                    || (k.msgtype == NavMessageType::CNV2),
                "bad ephemeris message {} for J04 {}",
                k.msgtype,
                k.epoch
            );

            if k.epoch == t_03_12_00_00_00_gpst {
                if k.msgtype == NavMessageType::LNAV {
                    assert_eq!(eph.clock_bias, 9.417533874512e-05);
                    assert_eq!(eph.clock_drift, 0.000000000000e+00);
                    assert_eq!(eph.clock_drift_rate, 0.000000000000e+00);
                    tests_passed += 1;
                } else if k.msgtype == NavMessageType::CNAV {
                    assert_eq!(eph.clock_bias, 9.417530964129e-05);
                    assert_eq!(eph.clock_drift, -4.263256414561e-14);
                    assert_eq!(eph.clock_drift_rate, 0.000000000000e+00);
                    tests_passed += 1;
                } else if k.msgtype == NavMessageType::CNV2 {
                    assert_eq!(eph.clock_bias, 9.417530964129e-05);
                    assert_eq!(eph.clock_drift, -4.263256414561e-14);
                    assert_eq!(eph.clock_drift_rate, 0.000000000000e+00);
                    tests_passed += 1;
                }
            }

            // TODO missing data base for IRNSS/LNAV
            // } else if k.sv == i09 {
            //     assert_eq!(
            //         k.msgtype,
            //         NavMessageType::LNAV,
            //         "bad ephemeris message for I09",
            //     );

            //     if k.epoch == t_03_12_00_00_00_gpst {
            //         assert_eq!(eph.clock_bias, 7.243417203426e-04);
            //         assert_eq!(eph.clock_drift, 1.728039933369e-11);
            //         assert_eq!(eph.clock_drift_rate, 0.000000000000e+00);
            //         tests_passed += 1;
            //     }
        }
    }

    assert_eq!(tests_passed, 8);
}

//     for (epoch, (msg, sv, data)) in rinex.ephemeris() {
//         } else if sv == sv!("I09") {
//             if *epoch == Epoch::from_str("2023-03-12T20:05:36 UTC").unwrap() {
//                 assert_eq!(
//                     data.sv_clock(),
//                     (7.255990058184e-04, 1.716671249596e-11, 0.000000000000e+00)
//                 );
//             }
//         } else if sv == sv!("R10") {
//             assert!(
//                 msg == NavMessageType::FDMA,
//                 "parsed bad ephemeris message {} for I09 {}",
//                 msg,
//                 epoch
//             );
//             if *epoch == Epoch::from_str("2023-03-12T01:45:00 UTC").unwrap() {
//                 assert_eq!(
//                     data.sv_clock(),
//                     (-9.130407124758e-05, 0.000000000000e+00, 5.430000000000e+03)
//                 );
//             }
//         }
//     }
//
//     for (epoch, (msg, sv, iondata)) in rinex.ionod_correction_models() {
//         if sv == sv!("G21") {
//             assert_eq!(msg, NavMessageType::LNAV);
//             if epoch == Epoch::from_str("2023-03-12T00:08:54 UTC").unwrap() {
//                 let kb = iondata.as_klobuchar();
//                 assert!(kb.is_some());
//                 let kb = kb.unwrap();
//                 assert_eq!(
//                     kb.alpha,
//                     (
//                         2.887099981308e-08,
//                         7.450580596924e-09,
//                         -1.192092895508e-07,
//                         0.000000000000e+00
//                     )
//                 );
//                 assert_eq!(
//                     kb.beta,
//                     (
//                         1.331200000000e+05,
//                         0.000000000000e+00,
//                         -2.621440000000e+05,
//                         1.310720000000e+05
//                     )
//                 );
//                 assert_eq!(kb.region, KbRegionCode::WideArea);
//             } else if epoch == Epoch::from_str("2023-03-12T23:41:24 UTC").unwrap() {
//                 let kb = iondata.as_klobuchar();
//                 assert!(kb.is_some());
//                 let kb = kb.unwrap();
//                 assert_eq!(
//                     kb.alpha,
//                     (
//                         2.887099981308e-08,
//                         7.450580596924e-09,
//                         -1.192092895508e-07,
//                         0.000000000000e+00
//                     )
//                 );
//                 assert_eq!(
//                     kb.beta,
//                     (
//                         1.331200000000e+05,
//                         0.000000000000e+00,
//                         -2.621440000000e+05,
//                         1.310720000000e+05
//                     )
//                 );
//                 assert_eq!(kb.region, KbRegionCode::WideArea);
//             }
//         } else if sv == sv!("G21") {
//             assert_eq!(msg, NavMessageType::CNVX);
//         } else if sv == sv!("J04")
//             && epoch == Epoch::from_str("2023-03-12T02:01:54 UTC").unwrap()
//         {
//             let kb = iondata.as_klobuchar();
//             assert!(kb.is_some());
//             let kb = kb.unwrap();
//             assert_eq!(
//                 kb.alpha,
//                 (
//                     3.259629011154e-08,
//                     -1.490116119385e-08,
//                     -4.172325134277e-07,
//                     -1.788139343262e-07
//                 )
//             );
//             assert_eq!(
//                 kb.beta,
//                 (
//                     1.269760000000e+05,
//                     -1.474560000000e+05,
//                     1.310720000000e+05,
//                     2.490368000000e+06
//                 )
//             );
//             assert_eq!(kb.region, KbRegionCode::WideArea);
//         }
//     }
//
//     for (epoch, (msg, sv, eop)) in rinex.earth_orientation() {
//         if sv == sv!("J04") {
//             assert_eq!(msg, NavMessageType::CNVX);
//             if *epoch == Epoch::from_str("2023-03-12T06:00:00 UTC").unwrap() {
//                 assert_eq!(
//                     eop.x,
//                     (-4.072475433350e-02, 2.493858337402e-04, 0.000000000000e+00)
//                 );
//                 assert_eq!(
//                     eop.y,
//                     (3.506240844727e-01, 3.324031829834e-03, 0.000000000000e+00)
//                 );
//                 assert_eq!(eop.t_tm, 18186);
//                 assert_eq!(
//                     eop.delta_ut1,
//                     (-1.924991607666e-02, -7.354915142059e-04, 0.000000000000e+00)
//                 );
//             }
//         } else if sv == sv!("C30") {
//             assert_eq!(msg, NavMessageType::CNVX);
//             if *epoch == Epoch::from_str("2023-03-12T11:00:00 UTC").unwrap() {
//                 assert_eq!(
//                     eop.x,
//                     (-4.079341888428e-02, 6.389617919922e-04, 0.000000000000e+00)
//                 );
//                 assert_eq!(
//                     eop.y,
//                     (3.462553024292e-01, 2.998828887939e-03, 0.000000000000e+00)
//                 );
//                 assert_eq!(eop.t_tm, 60483);
//                 assert_eq!(
//                     eop.delta_ut1,
//                     (-1.820898056030e-02, -5.761086940765e-04, 0.000000000000e+00)
//                 );
//             }
//         }
//     }
// }

// #[test]
// #[cfg(feature = "nav")]
// fn toe_glo() {
//     let path = Path::new(env!("CARGO_MANIFEST_DIR"))
//         .join("data")
//         .join("NAV")
//         .join("V2")
//         .join("dlf10010.21g");
//     let rinex = Rinex::from_file(path.to_string_lossy().as_ref());
//     assert!(rinex.is_ok());
//     let rinex = rinex.unwrap();
//     for (_toc, (_, sv, _ephemeris)) in rinex.ephemeris() {
//         match sv.prn {
//             3 => {},
//             17 => {},
//             1 => {},
//             18 => {},
//             19 => {},
//             8 => {},
//             16 => {},
//             _ => panic!("found unexpected SV"),
//         }
//     }
// }

// Computes TOE in said timescale
fn toe_helper(week: f64, week_s: f64, ts: TimeScale) -> Epoch {
    if ts == TimeScale::GST {
        Epoch::from_duration((week - 1024.0) * Unit::Week + week_s * Unit::Second, ts)
    } else {
        Epoch::from_duration(week * Unit::Week + week_s * Unit::Second, ts)
    }
}

#[test]
fn nav_toe_gal_bds() {
    let mut tests_passed = 0;

    let path = format!(
        "{}/data/NAV/V3/AMEL00NLD_R_20210010000_01D_MN.rnx",
        env!("CARGO_MANIFEST_DIR")
    );

    let dut = Rinex::from_file(path).unwrap();

    let t0 = Epoch::from_str("2021-01-01T00:00:00 BDT").unwrap();
    let t1 = Epoch::from_str("2021-01-01T05:00:00 BDT").unwrap();
    let t2 = Epoch::from_str("2021-01-01T10:10:00 GST").unwrap();
    let t3 = Epoch::from_str("2021-01-01T15:40:00 GST").unwrap();

    for (k, eph) in dut.nav_ephemeris_frames_iter() {
        let ts = k.sv.timescale().expect("only known timescales here!");

        if let Some(toe) = eph.toe(k.sv) {
            if k.epoch == t0 {
                assert_eq!(toe, toe_helper(0.782E3, 0.432E6, TimeScale::BDT));
                tests_passed += 1;
            } else if k.epoch == t1 {
                assert_eq!(toe, toe_helper(0.782E3, 0.450E6, TimeScale::BDT));
                tests_passed += 1;
            } else if k.epoch == t2 {
                assert_eq!(toe, toe_helper(0.2138E4, 0.4686E6, TimeScale::GST));
                tests_passed += 1;
            } else if k.epoch == t3 {
                assert_eq!(toe, toe_helper(0.2138E4, 0.4884E6, TimeScale::GST));
                tests_passed += 1;
            }
        }
    }
    assert_eq!(tests_passed, 4);
}

#[test]
#[cfg(feature = "nav")]
fn nav_v3_ionospheric_corr() {
    let path = PathBuf::new()
        .join(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("NAV")
        .join("V3")
        .join("CBW100NLD_R_20210010000_01D_MN.rnx");

    let path = path.to_string_lossy().to_string();

    let _rinex = Rinex::from_file(&path).unwrap();

    // for (t0, should_work) in [
    //     // VALID : publication datetime
    //     (Epoch::from_str("2021-01-01T00:00:00 UTC").unwrap(), true),
    //     // VALID day course : random into that dat
    //     (Epoch::from_str("2021-01-01T05:33:24 UTC").unwrap(), true),
    //     // VALID day course : 30 sec prior next day
    //     (Epoch::from_str("2021-01-01T23:59:30 UTC").unwrap(), true),
    //     // VALID day course : 1 sec prior next publication
    //     (Epoch::from_str("2021-01-01T23:59:59 UTC").unwrap(), true),
    //     // TOO LATE : MIDNIGHT DAY +1
    //     (Epoch::from_str("2021-01-02T00:00:00 UTC").unwrap(), false),
    //     // TOO LATE : MIDNIGHT DAY +1
    //     (Epoch::from_gregorian_utc_at_midnight(2021, 02, 01), false),
    //     // TOO EARLY
    //     (Epoch::from_gregorian_utc_at_midnight(2020, 12, 31), false),
    // ] {
    //     // TODO
    //     // let ionod_corr = rinex.ionod_correction(
    //     //     t0,
    //     //     30.0,               // fake elev: DONT CARE
    //     //     30.0,               // fake azim: DONT CARE
    //     //     10.0,               // fake latitude: DONT CARE
    //     //     20.0,               // fake longitude: DONT CARE
    //     //     Carrier::default(), // fake signal: DONT CARE
    //     // );
    //     // if should_work {
    //     //     assert!(
    //     //         ionod_corr.is_some(),
    //     //         "v3 ionod corr: should have returned a correction model for datetime {:?}",
    //     //         t0
    //     //     );
    //     // } else {
    //     //     assert!(
    //     //         ionod_corr.is_none(),
    //     //         "v3 ionod corr: should not have returned a correction model for datetime {:?}",
    //     //         t0
    //     //     );
    //     // }
    // }
}

#[test]
#[cfg(feature = "flate2")]
fn nav_v4_messages() {
    for fp in [
        "KMS300DNK_R_20221591000_01H_MN.rnx.gz",
        "BRD400DLR_S_20230710000_01D_MN.rnx.gz",
    ] {
        let fullpath = format!("{}/data/NAV/V4/{}", env!("CARGO_MANIFEST_DIR"), fp);

        let rinex = Rinex::from_gzip_file(&fullpath).unwrap();

        // ION(V4) logical correctness
        for (k, model) in rinex.nav_ionosphere_models_iter() {
            match k.sv.constellation {
                Constellation::GPS => {
                    assert!(
                        model.as_klobuchar().is_some(),
                        "GPS vehicles only publish Kb model",
                    );
                },
                Constellation::QZSS => {
                    assert!(
                        model.as_klobuchar().is_some(),
                        "QZSS vehicles only publish Kb model",
                    );
                },
                Constellation::BeiDou => match k.msgtype {
                    NavMessageType::D1D2 => {
                        assert!(
                            model.as_klobuchar().is_some(),
                            "BeiDou (D1D2) should be be a Kb model",
                        );
                    },
                    NavMessageType::CNVX => {
                        assert!(
                            model.as_bdgim().is_some(),
                            "BeiDou (CNVX) should be a Bd model"
                        );
                    },
                    _ => {
                        panic!(
                            "invalid message type \"{}\" for BeiDou ION frame",
                            k.msgtype
                        );
                    },
                },
                Constellation::IRNSS => {
                    assert!(
                        model.as_klobuchar().is_some(),
                        "NavIC/IRNSS vehicles only publish Kb model",
                    );
                },
                Constellation::Galileo => {
                    assert!(
                        model.as_nequick_g().is_some(),
                        "GAL vehicles only publish Ng model",
                    );
                },
                _ => {
                    panic!("finvalid constellation: {}", k.sv.constellation,);
                },
            }
        }

        // EOP(V4) logical correctness
        for (k, _) in rinex.nav_earth_orientation_frames_iter() {
            match k.sv.constellation {
                Constellation::GPS
                | Constellation::QZSS
                | Constellation::IRNSS
                | Constellation::BeiDou => {},
                _ => panic!("found invalid constellation: {}", k.sv.constellation),
            }
            match k.msgtype {
                NavMessageType::CNVX | NavMessageType::LNAV => {},
                _ => panic!("bad msg identified for GPS vehicle: {}", k.msgtype),
            }
        }

        // STO(V4) logical correctness
        for (k, _) in rinex.nav_system_time_frames_iter() {
            match k.msgtype {
                NavMessageType::LNAV
                | NavMessageType::FDMA
                | NavMessageType::IFNV
                | NavMessageType::D1D2
                | NavMessageType::SBAS
                | NavMessageType::CNVX => {},
                _ => panic!("bad \"{}\" message for STO frame", k.msgtype),
            }
        }
    }
}

#[test]
#[cfg(feature = "flate2")]
fn nav_v2_iono_alphabeta_and_toe() {
    let path = PathBuf::new()
        .join(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("NAV")
        .join("V2")
        .join("cbw10010.21n.gz");

    let path = path.to_string_lossy().to_string();
    let rinex = Rinex::from_gzip_file(&path).unwrap();

    let mut num_tests = 0;

    let t_01_01_000000 = Epoch::from_str("2021-01-01T00:00:00 GPST").unwrap();
    let t_01_01_015944 = Epoch::from_str("2021-01-01T01:59:44 GPST").unwrap();
    let t_01_01_020000 = Epoch::from_str("2021-01-01T02:00:00 GPST").unwrap();
    let t_01_01_080000 = Epoch::from_str("2021-01-01T08:00:00 GPST").unwrap();
    let t_01_02_000000 = Epoch::from_str("2021-01-02T00:00:00 GPST").unwrap();

    for (k, eph) in rinex.nav_ephemeris_frames_iter() {
        let sv_ts = k.sv.timescale().expect("unknown timescale");

        assert_eq!(sv_ts, TimeScale::GPST, "GPS NAV");

        let toe = eph
            .toe(k.sv)
            .expect(&format!("toe() failed for {} ({})", k.epoch, k.sv));

        assert_eq!(
            toe.time_scale,
            TimeScale::GPST,
            "TOE returned wrong timescale for GPS NAV"
        );

        let prn = k.sv.prn;

        if k.epoch == t_01_01_000000 {
            assert_eq!(prn, 8, "invalid SV");
            let expected = toe_helper(2.138E3, 4.32E5, TimeScale::GPST);
            assert_eq!(toe, expected);
            num_tests += 1;
        } else if k.epoch == t_01_01_020000 {
            assert_eq!(prn, 1, "invalid SV");
            let expected = toe_helper(2.138E3, 4.392E5, TimeScale::GPST);

            assert_eq!(toe, expected);
            num_tests += 1;
        } else if k.epoch == t_01_01_080000 {
            if prn == 1 {
                num_tests += 1;
            } else if prn == 2 {
                num_tests += 1;
            } else if prn == 3 {
                num_tests += 1;
            } else if prn == 4 {
                num_tests += 1;
            } else if prn == 5 {
                num_tests += 1;
            } else if prn == 6 {
                num_tests += 1;
            } else if prn == 7 {
                let expected = toe_helper(2.138E3, 4.608E5, TimeScale::GPST);
                assert_eq!(toe, expected);
                num_tests += 1;
            } else if prn == 9 {
                num_tests += 1;
            } else if prn == 12 {
                num_tests += 1;
            } else if prn == 17 {
                num_tests += 1;
            } else if prn == 19 {
                num_tests += 1;
            } else if prn == 21 {
                num_tests += 1;
            } else if prn == 22 {
                num_tests += 1;
            } else if prn == 25 {
                num_tests += 1;
            } else if prn == 26 {
                num_tests += 1;
            } else if k.sv.prn == 29 {
                num_tests += 1;
            } else if k.sv.prn == 30 {
                num_tests += 1;
            } else if k.sv.prn == 31 {
                num_tests += 1;
            } else {
                panic!("invalid SV=G{:02} @ {}", prn, k.epoch);
            }
        } else if k.epoch == t_01_02_000000 {
            if prn == 5 {
                let expected = toe_helper(2.138E3, 5.184E5, TimeScale::GPST);
                assert_eq!(expected, toe);
                num_tests += 1;
            } else if prn == 7 {
                num_tests += 1;
            } else if prn == 8 {
                let expected = toe_helper(2.138E3, 5.184E5, TimeScale::GPST);
                assert_eq!(toe, expected);
                num_tests += 1;
            } else if k.sv.prn == 10 {
                num_tests += 1;
            } else if k.sv.prn == 11 {
                num_tests += 1;
            } else if k.sv.prn == 13 {
                num_tests += 1;
            } else if k.sv.prn == 15 {
                num_tests += 1;
            } else if k.sv.prn == 16 {
                num_tests += 1;
            } else if k.sv.prn == 18 {
                num_tests += 1;
            } else if k.sv.prn == 20 {
                num_tests += 1;
            } else if k.sv.prn == 21 {
                num_tests += 1;
            } else if k.sv.prn == 23 {
                num_tests += 1;
            } else if k.sv.prn == 26 {
                num_tests += 1;
            } else if k.sv.prn == 27 {
                num_tests += 1;
            } else if k.sv.prn == 29 {
                num_tests += 1;
            } else if k.sv.prn == 30 {
                let expected = toe_helper(2.138000000000E3, 5.184000000000E5, TimeScale::GPST);
                assert_eq!(expected, toe);
                num_tests += 1;
            } else {
                panic!("invalid SV=G{:02} @ {}", prn, k.epoch);
            }
        } else if k.epoch == t_01_01_015944 {
            if prn == 7 {
                let expected = toe_helper(2.138000000000E3, 4.391840000000E5, TimeScale::GPST);

                assert_eq!(toe, expected);
                num_tests += 1;
            } else if prn == 8 {
                let expected = toe_helper(2.138000000000E3, 4.391840000000E5, TimeScale::GPST);

                assert_eq!(toe, expected);
                num_tests += 1;
            } else {
                panic!("invalid SV=G{:02} @ {}", prn, k.epoch);
            }
        }
    }

    assert_eq!(num_tests, 38);

    // for (t0, should_work) in [
    //     // MIDNIGHT T0 exact match
    //     (Epoch::from_gregorian_utc(2021, 1, 1, 00, 00, 00, 0), true),
    //     // VALID day course : 1sec into that day
    //     (Epoch::from_gregorian_utc(2021, 1, 1, 00, 00, 01, 0), true),
    //     // VALID day course : random into that day
    //     (Epoch::from_gregorian_utc(2021, 1, 1, 05, 33, 24, 0), true),
    //     // VALID day course : 1 sec prior next day
    //     (Epoch::from_str("2021-01-01T23:59:59 UTC").unwrap(), true),
    //     // TOO LATE : MIDNIGHT DAY +1
    //     (Epoch::from_str("2021-01-02T00:00:00 UTC").unwrap(), false),
    //     // TOO LATE : MIDNIGHT DAY +1
    //     (Epoch::from_gregorian_utc_at_midnight(2021, 01, 02), false),
    //     // TOO EARLY
    //     (Epoch::from_gregorian_utc_at_midnight(2020, 12, 31), false),
    // ] {
    //     // TODO
    //     // let ionod_corr = rinex.ionod_correction(
    //     //     t0,
    //     //     30.0,               // fake elev: DONT CARE
    //     //     30.0,               // fake azim: DONT CARE
    //     //     10.0,               // fake latitude: DONT CARE
    //     //     20.0,               // fake longitude: DONT CARE
    //     //     Carrier::default(), // fake signal: DONT CARE
    //     // );
    //     // if should_work {
    //     //     assert!(
    //     //         ionod_corr.is_some(),
    //     //         "v2 ionod corr: should have returned a correction model @{}",
    //     //         t0
    //     //     );
    //     // } else {
    //     //     assert!(
    //     //         ionod_corr.is_none(),
    //     //         "v2 ionod corr: should not have returned a correction model @{}",
    //     //         t0
    //     //     );
    //     // }
    // }
}

#[test]
fn gh_395_kepler_nav3() {
    let rinex = Rinex::from_gzip_file("data/NAV/V3/BRDM00DLR_S_20241310000_01D_MN.rnx.gz")
        .unwrap();

    // C20@ 2024-05-10T17:00:00BDT is said to work
    // C21@ 2024-05-10T17:00:00BDT is said corrupt
    let t0 = Epoch::from_str("2024-05-10T17:00:00 BDT").unwrap();

    let (c20, mut c20_passed) = (SV::from_str("C20").unwrap(), false);
    let (c21, mut c21_passed) = (SV::from_str("C21").unwrap(), false);

    for (key, ephemeris) in rinex.nav_ephemeris_frames_iter() {
        if key.epoch == t0 {
            if key.sv == c20 {
                // check all fields
                assert!(ephemeris.get_orbit_f64("toe").is_some());
                assert!(ephemeris.get_orbit_f64("omega").is_some());
                assert!(ephemeris.get_orbit_f64("sqrta").is_some());
                assert!(ephemeris.get_orbit_f64("cis").is_some());
                assert!(ephemeris.get_orbit_f64("cic").is_some());
                
                // demonstrate we may proceed
                assert!(ephemeris.kepler().is_some());
                assert!(ephemeris.perturbations().is_some());

                // calculate kepler equations
                let position = ephemeris.kepler2position(c20, t0).unwrap();
                println!("C20: {position}");

                // test passed
                c20_passed = true;

            } else if key.sv == c21 {
                // check all fields
                assert!(ephemeris.get_orbit_f64("toe").is_some());
                assert!(ephemeris.get_orbit_f64("omega").is_some());
                assert!(ephemeris.get_orbit_f64("sqrta").is_some());
                assert!(ephemeris.get_orbit_f64("cis").is_some());
                assert!(ephemeris.get_orbit_f64("cic").is_some());
                
                // demonstrate we may proceed
                assert!(ephemeris.kepler().is_some());
                assert!(ephemeris.perturbations().is_some());
                
                // calculate kepler equations
                let position = ephemeris.kepler2position(c21, t0).unwrap();
                println!("C21: {position}");
                
                // test passed
                c21_passed = true;
            }
        }
    }

    assert!(c20_passed);
    assert!(c21_passed);
}
