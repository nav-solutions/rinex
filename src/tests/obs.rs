use crate::{
    observation::{EpochFlag, LliFlags, ObsKey, SignalObservation, SNR},
    prelude::{Epoch, GeodeticMarker, Observable, Rinex, SV},
    tests::toolkit::{
        generic_null_rinex_test, generic_observation_rinex_test, generic_rinex_comparison,
        SignalDataPoint, TimeFrame,
    },
};

use std::{fs::remove_file, path::Path, str::FromStr};

#[test]
fn v2_aopr0010_17o() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("OBS")
        .join("V2")
        .join("aopr0010.17o");

    let fullpath = path.to_string_lossy();
    let dut = Rinex::from_file(fullpath.as_ref()).unwrap();

    generic_observation_rinex_test(
            &dut,
            "2.10",
            Some("GPS"),
            false,
            "G31, G27, G03, G32, G16, G14, G08, G23, G22, G26, G30, G27, G11, G16, G08, G07, G23, G09, G01, G06, G17, G28, G19",
            "GPS",
            &[("GPS", "L1, L2, C1, P1, P2")],
            Some("2017-01-01T00:00:00 GPST"),
            None,
            Some((2390232.6900, -5564587.6100, 1995022.1400)),
            None,
            None,
            TimeFrame::from_erratic_csv(
                "2017-01-01T00:00:00 GPST,    
                2017-01-01T03:33:40 GPST,
                2017-01-01T06:09:10 GPST",
            ),
            vec![],
            vec![],
        );

    dut.to_file("v2_aopr0010_17o.txt").unwrap();

    let parsed = Rinex::from_file("v2_aopr0010_17o.txt").unwrap();
    generic_rinex_comparison(&parsed, &dut);
    let _ = remove_file("v2_aopr0010_17o.txt");
}

#[test]
fn v2_npaz3550_21o() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("OBS")
        .join("V2")
        .join("npaz3550.21o");

    let fullpath = path.to_string_lossy();
    let dut = Rinex::from_file(fullpath.as_ref()).unwrap();

    generic_observation_rinex_test(
        &dut,
        "2.11",
        Some("MIXED"),
        false,
        "G01, G08, G10, G15, G16, G18, G21, G23, G26, G32, 
            R04, R05, R06, R07, R10, R12, R19, R20, R21, R22",
        "GPS, GLO",
        &[
            ("GPS", "C1, L1, L2, P2, S1, S2"),
            ("GLO", "C1, L1, L2, P2, S1, S2"),
        ],
        Some("2021-12-21T00:00:00 GPST"),
        Some("2021-12-21T23:59:30 GPST"),
        None,
        None,
        None,
        TimeFrame::from_inclusive_csv("2021-12-21T00:00:00 GPST, 2021-12-21T01:04:00 GPST, 30 s"),
        vec![],
        vec![],
    );

    dut.to_file("v2_npaz3550_21o.txt").unwrap();

    let parsed = Rinex::from_file("v2_npaz3550_21o.txt").unwrap();
    generic_rinex_comparison(&parsed, &dut);

    let _ = remove_file("v2_npaz3550_21o.txt");
}

#[test]
fn v2_rovn0010_21o() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("OBS")
        .join("V2")
        .join("rovn0010.21o");

    let fullpath = path.to_string_lossy();

    let dut = Rinex::from_file(fullpath.as_ref()).unwrap();

    generic_observation_rinex_test(
            &dut,
            "2.11",
            Some("MIXED"), 
            false,
            "G01, G03, G07, G08, G10, G13, G14, G15, G16, G18, G20, G21, G22, G23, G24, G26, G27, G28, G30, G32, 
            R01, R02, R03, R04, R08, R09, R10, R15, R16, R17, R18, R19, R20, R24", 
            "GPS, GLO",
            &[
                ("GPS", "C1, C2, C5, L1, L2, L5, P1, P2, S1, S2, S5"),
                ("GLO", "C1, C2, C5, L1, L2, L5, P1, P2, S1, S2, S5"),
            ],
            Some("2021-01-01T00:00:00 GPST"), 
            Some("2021-01-01T23:59:30 GPST"),  
            Some((3859571.8076,  413007.6749, 5044091.5729)),
            Some("Hans van der Marel"),
            Some(GeodeticMarker::default().with_name("ROVN").with_number("13544M001")),

            TimeFrame::from_erratic_csv("
                2021-01-01T00:00:00 GPST,
                2021-01-01T00:00:30 GPST,
                2021-01-01T01:10:00 GPST,
                2021-01-01T02:25:00 GPST,
                2021-01-01T02:25:30 GPST,
                2021-01-01T02:26:00 GPST
            ",
            ),
            vec![],
            vec![],
        );

    let agency = dut.header.agency.as_ref().unwrap();
    assert_eq!(agency, "TU Delft for Deltares");

    dut.to_file("v2_rovn0010_21o.txt").unwrap();

    let parsed = Rinex::from_file("v2_rovn0010_21o.txt").unwrap();
    generic_rinex_comparison(&parsed, &dut);

    let _ = remove_file("v2_rovn0010_21o.txt");
}

#[test]
fn v3_duth0630() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("OBS")
        .join("V3")
        .join("DUTH0630.22O");

    let fullpath = path.to_string_lossy();
    let dut = Rinex::from_file(fullpath.as_ref()).unwrap();

    generic_observation_rinex_test(
            &dut,
            "3.02",
            Some("MIXED"),
            false,
            "G03, G01, G04, G06, G09, G17, G19, G21, G22, G26, G31, G32, R01, R02, R08, R09, R10, R17, R23, R24",
            "GPS, GLO",
            &[
                ("GPS", "C1C, L1C, D1C, S1C, C2W, L2W, D2W, S2W"),
                ("GLO", "C1C, L1C, D1C, S1C, C2P, L2P, D2P, S2P"),
            ],
            Some("2022-03-04T00:00:00 GPST"),
            Some("2022-03-04T23:59:30 GPST"),
            None,
            None,
            None,
            TimeFrame::from_erratic_csv("2022-03-04T00:00:00 GPST, 2022-03-04T00:28:30 GPST, 2022-03-04T00:57:00 GPST"),
            vec![
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2022-03-04T00:00:00 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: {
                        SignalObservation {
                            sv: SV::from_str("G01").unwrap(),
                            observable: Observable::from_str("C1C").unwrap(),
                            value: 20243517.560,
                            lli: None,
                            snr: None,
                        }
                    }
                },
            ],
            vec![],
        );

    dut.to_file("v3_duth0630.txt").unwrap();

    let parsed = Rinex::from_file("v3_duth0630.txt").unwrap();
    generic_rinex_comparison(&parsed, &dut);

    let _ = remove_file("v3_duth0630.txt");
}

#[test]
fn v4_kms300dnk_r_2022_v3crx() {
    let test_resource = env!("CARGO_MANIFEST_DIR").to_owned()
        + "/data/CRNX/V3/KMS300DNK_R_20221591000_01H_30S_MO.crx";

    let dut = Rinex::from_file(&test_resource).unwrap();

    generic_observation_rinex_test(
            &dut,
            "4.00",
            Some("MIXED"),
            false,
            "G05, G09, G16, G18, G20, G23, G26, G27, G29, G31, E01, E03, E07, E08, E24, E25, E26, E31, E33,
            R03, R04, R05, R10, R11, R12, R13, R20, R21,
            J04, 
            C05, C08, C13, C20, C24, C26, C29, C30, C32, C35, C36, C38, C41, C45, C60,
            S23, S25, S26, S27, S36, S44, S48",
            "GPS, GAL, GLO, QZSS, BDS, EGNOS, SDCM, GAGAN, BDSBAS, ASAL",
            &[
                ("GPS", "C1C, C1L, C1W, C2L, C2W, C5Q, L1C, L1L, L2L, L2W, L5Q"),
                ("GAL", "C1C, C5Q, C6C, C7Q, C8Q, L1C, L5Q, L6C, L7Q, L8Q"),
                ("GLO", "C1C, C1P, C2C, C2P, C3Q, L1C, L1P, L2C, L2P, L3Q"),
                ("QZSS", "C1C, C1L, C2L, C5Q, L1C, L1L, L2L, L5Q"),
                ("SBAS", "C1C, C5I, L1C, L5I"),
                ("BDS", "C1P, C2I, C5P, C6I, C7D, C7I, L1P, L2I, L5P, L6I, L7D, L7I"),
            ],
            Some("2022-06-08T10:00:00 GPST"),
            Some("2022-06-08T10:59:30 GPST"),
            Some((3516213.4380, 781859.8595, 5246037.9660)),
            Some("Unknown"),
            Some(GeodeticMarker::default().with_name("KMS3").with_number("Unknown")),
            TimeFrame::from_inclusive_csv(
                "2022-06-08T10:00:00 GPST, 2022-06-08T10:09:00 GPST, 30 s",
            ),
            vec![],
            vec![],
        );
}

#[test]
#[ignore] // TODO
fn v2_kosg0010_95o() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("OBS")
        .join("V2")
        .join("KOSG0010.95O");

    let fullpath = path.to_string_lossy();

    let dut = Rinex::from_file(fullpath.as_ref()).unwrap();

    generic_observation_rinex_test(
        &dut,
        "2.0",
        Some("GPS"),
        false,
        "G07, G17, G21, G22, G23, G28, G31,
        G04, G16, G18, G19, G22, G24, G27, G29,
        G01, G05, G06, G17, G20, G22, G24, G25",
        "GPS",
        &[("GPS", "C1, L1, L2, P1, P2")],
        None, //TODO : problem here, parsing very old files
        // Some("1995-01-01T00:00:00 GPST"),
        None, //TODO: problem here, parsing very old files
        //Some("1995-01-01T23:59:30 GPST"),
        None,
        None,
        None,
        TimeFrame::from_erratic_csv(
            "
            1995-01-01T00:00:00 GPST,
            1995-01-01T11:00:00 GPST,
            1995-01-01T20:44:30 GPST
        ",
        ),
        vec![],
        vec![],
    );

    dut.to_file("v2_kosg0010_95o.txt").unwrap();

    // TODO
    let _parsed = Rinex::from_file("v2_kosg0010_95o.txt").unwrap();

    //generic_rinex_comparison(&parsed, &dut);

    let _ = remove_file("v2_kosg0010_95o.txt");
}

#[test]
fn v2_ajac3550() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("OBS")
        .join("V2")
        .join("AJAC3550.21O");

    let fullpath = path.to_string_lossy();
    let dut = Rinex::from_file(fullpath.as_ref()).unwrap();

    generic_observation_rinex_test(
            &dut,
            "2.11",
            Some("MIXED"),
            false,
            "G07, G08, G10, G16, G18, G21, G23, G26, G32, R04, R05, R10, R12, R19, R20, R21, E04, E11, E12, E19, E24, E25, E31, E33, S23, S36",
            "GPS, GAL, GLO, EGNOS",
            &[
                ("GPS", "L1, L2, C1, C2, P1, P2, D1, D2, S1, S2, L5, C5, D5, S5, L7, C7, D7, S7, L8, C8, D8, S8"),
                ("GLO", "L1, L2, C1, C2, P1, P2, D1, D2, S1, S2, L5, C5, D5, S5, L7, C7, D7, S7, L8, C8, D8, S8"),
                ("SBAS", "L1, L2, C1, C2, P1, P2, D1, D2, S1, S2, L5, C5, D5, S5, L7, C7, D7, S7, L8, C8, D8, S8"),
                ("Gal", "L1, L2, C1, C2, P1, P2, D1, D2, S1, S2, L5, C5, D5, S5, L7, C7, D7, S7, L8, C8, D8, S8"),
            ],
            Some("2021-12-21T00:00:00 GPST"),
            None,
            Some((4696989.6880, 723994.1970 ,4239678.3040)),
            None,
            None,
            TimeFrame::from_erratic_csv("2021-12-21T00:00:00 GPST, 2021-12-21T00:00:30 GPST"),
            vec![
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:00 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("G07").unwrap(),
                        value: 131857102.133,
                        observable: Observable::from_str("L1").unwrap(),
                        lli: None,
                        snr: Some(SNR::from(6)),
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:00 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("G07").unwrap(),
                        value: 102745756.542,
                        observable: Observable::from_str("L2").unwrap(),
                        lli: LliFlags::from_bits(4),
                        snr: Some(SNR::from(5)),
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:00 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("G07").unwrap(),
                        value: 25091572.300,
                        observable: Observable::from_str("C1").unwrap(),
                        lli: None,
                        snr: None,
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:00 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("G07").unwrap(),
                        value: 25091565.600,
                        observable: Observable::from_str("P2").unwrap(),
                        lli: None,
                        snr: None,
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:00 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("G07").unwrap(),
                        value: -411.138,
                        observable: Observable::from_str("D1").unwrap(),
                        lli: None,
                        snr: None,
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:00 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("G07").unwrap(),
                        value: -320.373,
                        observable: Observable::from_str("D2").unwrap(),
                        lli: None,
                        snr: None,
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:00 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("G07").unwrap(),
                        value: 35.300,
                        observable: Observable::from_str("S2").unwrap(),
                        lli: None,
                        snr: None,
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:00 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("G08").unwrap(),
                        value: 114374313.914,
                        observable: Observable::from_str("L1").unwrap(),
                        lli: None,
                        snr: Some(SNR::from(8)),
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:00 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("G08").unwrap(),
                        value: 89122819.839,
                        observable: Observable::from_str("L2").unwrap(),
                        lli: LliFlags::from_bits(4),
                        snr: Some(SNR::from(7)),
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:00 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("G08").unwrap(),
                        value: 21764705.880,
                        observable: Observable::from_str("C1").unwrap(),
                        lli: None,
                        snr: None,
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:00 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("S36").unwrap(),
                        value: 197948874.430,
                        observable: Observable::from_str("L1").unwrap(),
                        lli: None,
                        snr: Some(SNR::from(8)),
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:00 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("S36").unwrap(),
                        value: 37668418.660,
                        observable: Observable::from_str("C1").unwrap(),
                        lli: None,
                        snr: None,
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:30 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("G07").unwrap(),
                        value: 131869667.223,
                        observable: Observable::from_str("L1").unwrap(),
                        lli: None,
                        snr: Some(SNR::from(5)),
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:30 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("G07").unwrap(),
                        value: 25093963.200,
                        observable: Observable::from_str("C1").unwrap(),
                        lli: None,
                        snr: None,
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:30 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("G07").unwrap(),
                        value: -426.868,
                        observable: Observable::from_str("D1").unwrap(),
                        lli: None,
                        snr: None,
                    },
                },
                SignalDataPoint {
                    key: ObsKey {
                        epoch: Epoch::from_str("2021-12-21T00:00:30 GPST").unwrap(),
                        flag: EpochFlag::Ok,
                    },
                    signal: SignalObservation {
                        sv: SV::from_str("G08").unwrap(),
                        value: 114305043.723,
                        observable: Observable::from_str("L1").unwrap(),
                        lli: None,
                        snr: Some(SNR::DbHz48_53),
                    },
                },
            ],
            vec![],
        );

    dut.to_file("v2_ajac3550.txt").unwrap();

    let parsed = Rinex::from_file("v2_ajac3550.txt").unwrap();
    generic_rinex_comparison(&parsed, &dut);

    let _ = remove_file("v2_ajac3550.txt");
}

#[test]
fn v3_noa10630() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("OBS")
        .join("V3")
        .join("NOA10630.22O");
    let fullpath = path.to_string_lossy();
    let dut = Rinex::from_file(fullpath.as_ref()).unwrap();

    generic_observation_rinex_test(
        &dut,
        "3.02",
        Some("GPS"),
        false,
        "G01, G03, G04, G06, G09, G17, G19, G21, G22, G31",
        "GPS",
        &[("GPS", "C1C, L1C, D1C, S1C, C2W, L2W, D2W, S2W")],
        Some("2022-03-04T00:00:00 GPST"),
        Some("2022-03-04T23:59:30 GPST"),
        None,
        None,
        None,
        TimeFrame::from_erratic_csv(
            "2022-03-04T00:00:00 GPST,
                2022-03-04T00:00:30 GPST,
                2022-03-04T00:01:00 GPST,
                2022-03-04T00:52:30 GPST",
        ),
        vec![],
        vec![],
    );

    dut.to_file("v3_noa10630.txt").unwrap();

    let parsed = Rinex::from_file("v3_noa10630.txt").unwrap();
    generic_rinex_comparison(&parsed, &dut);

    let _ = remove_file("v3_noa10630.txt");
}

#[test]
fn obs_null_substract() {
    // Self - Self is always a null RINEX

    for file in ["aopr0010.17o", "npaz3550.21o", "rovn0010.21o"] {
        let fullpath = format!("{}/data/OBS/V2/{}", env!("CARGO_MANIFEST_DIR"), file);
        let dut = Rinex::from_file(fullpath).unwrap();

        let diffed = dut.observations_substract(&dut).unwrap_or_else(|e| {
            panic!("v2/{} diff failed with: {:?}", file, e);
        });

        generic_null_rinex_test(&diffed);
    }

    for file in ["DUTH0630.22O", "NOA10630.22O"] {
        let fullpath = format!("{}/data/OBS/V3/{}", env!("CARGO_MANIFEST_DIR"), file);
        let dut = Rinex::from_file(fullpath).unwrap();

        let diffed = dut.observations_substract(&dut).unwrap_or_else(|e| {
            panic!("v3/{} diff failed with: {:?}", file, e);
        });

        generic_null_rinex_test(&diffed);
    }

    for file in [
        "DUTH0630.22D",
        "VLNS0010.22D",
        "VLNS0630.22D",
        "pdel0010.21d",
    ] {
        let fullpath = format!("{}/data/CRNX/V3/{}", env!("CARGO_MANIFEST_DIR"), file);
        let dut = Rinex::from_file(fullpath).unwrap();

        let diffed = dut.observations_substract(&dut).unwrap_or_else(|e| {
            panic!("v3/{} diff failed with: {:?}", file, e);
        });

        generic_null_rinex_test(&diffed);
    }
}
