#[cfg(test)]
mod test {
    use crate::{
        prelude::{Rinex, qc::Merge},
        tests::toolkit::{TimeFrame, generic_observation_rinex_test},
    };
    use std::{
        //fs::remove_file as fs_remove_file,
        path::PathBuf,
    };

    #[test]
    fn fail_on_type_mismatch() {
        let data = PathBuf::new().join(env!("CARGO_MANIFEST_DIR")).join("data");

        let path1 = data
            .clone()
            .join("NAV")
            .join("V3")
            .join("AMEL00NLD_R_20210010000_01D_MN.rnx");

        let path2 = data.clone().join("OBS").join("V3").join("LARM0630.22O");

        let path = path1.to_string_lossy().to_string();
        let mut r1 = Rinex::from_file(&path).unwrap();

        let path = path2.to_string_lossy().to_string();
        let r2 = Rinex::from_file(&path).unwrap();

        assert!(r1.merge_mut(&r2).is_err())
    }

    #[test]
    fn merge_nav() {
        let data = PathBuf::new().join(env!("CARGO_MANIFEST_DIR")).join("data");

        let path = data
            .clone()
            .join("NAV")
            .join("V3")
            .join("AMEL00NLD_R_20210010000_01D_MN.rnx");

        let path = path.to_string_lossy().to_string();
        let rnx_a = Rinex::from_file(&path).unwrap();

        let path = data
            .clone()
            .join("NAV")
            .join("V3")
            .join("CBW100NLD_R_20210010000_01D_MN.rnx");

        let path = path.to_string_lossy().to_string();
        let rnx_b = Rinex::from_file(&path).unwrap();

        let merged = rnx_a.merge(&rnx_b);
        assert!(
            merged.is_ok(),
            "failed to merge NAV/V3/CBW100NLD_R_20210010000_01D_MN.rnx into NAV/V3/AMEL00NLD_R_20210010000_01D_MN.rnx"
        );

        let merged = merged.unwrap();

        merged
            .to_file("cbw-merged.txt")
            .expect("failed to generate file!");

        assert!(merged.is_merged());

        // parse back
        let rnx = Rinex::from_file("cbw-merged.txt").unwrap();

        assert!(rnx.is_merged());

        /*
         * Unlock reciprocity test in near future
         *  NAV file production does not work correctly at the moment,
         *  due to formatting issues
         */
        // assert_eq!(rnx, merged, "Merge::ops reciprocity");

        // remove file we just generated
        let _ = std::fs::remove_file("cbw-merged.txt");
    }

    #[test]
    fn merge_obs_v2() {
        let data = PathBuf::new()
            .join(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("OBS")
            .join("V2");

        let path = data.clone().join("AJAC3550.21O");

        let path = path.to_string_lossy().to_string();
        let rnx_a = Rinex::from_file(&path).unwrap();

        let path = data.clone().join("npaz3550.21o");

        let path = path.to_string_lossy().to_string();
        let rnx_b = Rinex::from_file(&path).unwrap();

        let merged = rnx_a.merge(&rnx_b).unwrap();

        assert!(merged.is_merged());

        generic_observation_rinex_test(
            &merged,
            "2.11",
            Some("MIXED"),
            false,
            "G01, G07, G08, G10, G15, G16, G18, G21, G23, G26, G32, R04, R05, R06, R07, R10, R12, R19, R20, R21, R22, E04, E11, E12, E19, E24, E25, E31, E33, S23, S36",
            "GPS, GLO, GAL, EGNOS",
            &[
                (
                    "GPS",
                    "C1, C2, C5, C7, C8, L1, L2, L5, L7, L8, P1, P2, S1, S2, S5, S7, S8, D1, D2, D5, D7, D8",
                ),
                (
                    "GLO",
                    "L1, L2, L5, L7, L8, D1, D2, D5, D7, D8, S1, S2, S5, S7, S8, C1, C2, C5, C7, C8, P1, P2",
                ),
                (
                    "GAL",
                    "L1, L2, L5, L7, L8, D1, D2, D5, D7, D8, S1, S2, S5, S7, S8, C1, C2, C5, C7, C8, P1, P2",
                ),
            ],
            Some("2021-12-21T00:00:00 GPST"),
            Some("2021-12-21T23:59:30 GPST"),
            None,
            None,
            None,
            TimeFrame::from_inclusive_csv(
                "2021-12-21T00:00:00 GPST, 2021-12-21T01:04:00 GPST, 30 s",
            ),
            vec![],
            vec![],
        );

        // TODO
        // merged.to_file("ajac-merge.txt").unwrap();

        // // parse back
        // let rnx = Rinex::from_file("ajac-merge.txt").unwrap();

        // assert!(rnx.is_merged(), "merged file not declared as such!");

        // remove file we just generated
        // let _ = fs_remove_file("ajac-merged.txt");
    }

    #[test]
    fn merge_obs_v3() {
        let data = PathBuf::new()
            .join(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("OBS")
            .join("V3");

        let path = data.clone().join("ACOR00ESP_R_20213550000_01D_30S_MO.rnx");

        let path = path.to_string_lossy().to_string();
        let rnx_a = Rinex::from_file(&path).unwrap();

        let path = data.clone().join("ALAC00ESP_R_20220090000_01D_30S_MO.rnx");

        let path = path.to_string_lossy().to_string();
        let rnx_b = Rinex::from_file(&path).unwrap();

        let merged = rnx_a.merge(&rnx_b).unwrap();

        assert!(merged.is_merged());

        generic_observation_rinex_test(
            &merged,
            "3.04",
            Some("MIXED"),
            false,
            "G01, G03, G07, G08, G10, G14, G16, G18, G21, G23, G26, G27, G30, G32, 
             R01, R02, R04, R05, R08, R10, R12, R14, R15, R17, R20, R21, R23, R24,
            E02, E03, E05, E09, E11, E12, E15, E24, E25, E27, E30, E31, E33, E36, 
            C05, C06, C09, C11, C14, C16, C21, C22, C23, C25, C27, C28, C30, C32, C33, C34, C36, C37, C39, C41, C42, C43, C44, C46, C58",
            "GPS, GLO, GAL, BDS",
            &[
                ("GPS", "C1C, L1C, S1C, C2S, L2S, S2S, C2W, L2W, S2W, C5Q, L5Q, S5Q"),
                ("GLO", "C1C, L1C, S1C, C2P, L2P, S2P, C2C, L2C, S2C, C3Q, L3Q, S3Q"),
                ("GAL", "C1C, L1C, S1C, C5Q, L5Q, S5Q, C6C, L6C, S6C, C7Q, L7Q, S7Q, C8Q, L8Q, S8Q"),
                ("BDS", "C2I, L2I, S2I, C6I, L6I, S6I, C7I, L7I, S7I"),
            ],
            Some("2021-12-21T00:00:00 GPST"),
            Some("2022-01-09T23:59:30 GPST"),
            None,
            None,
            None,
            TimeFrame::from_erratic_csv(
                "2021-12-21T00:00:00 GPST,
                2021-12-21T00:00:30 GPST,
                2021-12-21T00:01:00 GPST,
                2021-12-21T00:01:30 GPST,
                2021-12-21T00:02:00 GPST,
                2021-12-21T00:02:30 GPST,
                2021-12-21T00:03:00 GPST,
                2021-12-21T00:03:30 GPST,
                2021-12-21T00:04:00 GPST,
                2021-12-21T00:04:30 GPST,
                2021-12-21T00:05:00 GPST,
                2021-12-21T00:05:30 GPST,
                2021-12-21T00:06:00 GPST,
                2021-12-21T00:06:30 GPST,
                2021-12-21T00:07:00 GPST,
                2021-12-21T00:07:30 GPST,
                2021-12-21T00:08:00 GPST,
                2021-12-21T00:08:30 GPST,
                2021-12-21T00:09:00 GPST,
                2021-12-21T00:09:30 GPST,
                2021-12-21T00:10:00 GPST,
                2021-12-21T00:10:30 GPST,
                2021-12-21T00:11:00 GPST,
                2021-12-21T00:11:30 GPST,
                2021-12-21T00:12:00 GPST,
                2022-01-09T00:00:00 GPST,
                2022-01-09T00:00:30 GPST,
                2022-01-09T00:13:30 GPST",
            ),
            vec![],
            vec![],
        );

        merged.to_file("alac-acor-merge.txt").unwrap();

        // parse back
        let rnx = Rinex::from_file("alac-acor-merge.txt").unwrap();

        assert!(rnx.is_merged(), "merged file not declared as such!");

        // remove file we just generated
        // let _ = fs_remove_file("ajac-merged.txt");
    }

    // #[cfg(feature = "antex")]
    // use crate::antex::antenna::AntennaMatcher;
    // #[cfg(feature = "antex")]
    // use crate::Carrier;

    // TODO
    // #[test]
    // #[cfg(feature = "flate2")]
    // #[cfg(feature = "antex")]
    // fn merge_atx() {
    //     let fp = env!("CARGO_MANIFEST_DIR").to_owned()
    //         + "/data/ATX/V1/TROSAR25.R4__LEIT_2020_09_23.atx";
    //     let rinex_a = Rinex::from_file(&fp);
    //     let rinex_a = rinex_a.unwrap();

    //     let fp =
    //         env!("CARGO_MANIFEST_DIR").to_owned() + "/data/ATX/V1/igs14_small.atx.gz";
    //     let rinex_b = Rinex::from_file(&fp);
    //     let rinex_b = rinex_b.unwrap();

    //     let merged = rinex_a.merge(&rinex_b);
    //     assert!(merged.is_ok(), "merged atx(a,b) failed");
    //     let merged = merged.unwrap();

    //     let antennas: Vec<_> = merged.antennas().collect();
    //     assert_eq!(antennas.len(), 7, "bad number of antennas");

    //     for (name, expected_apc) in [
    //         ("JPSLEGANT_E", (1.36, -0.43, 35.44)),
    //         ("JPSODYSSEY_I", (1.06, -2.43, 70.34)),
    //         ("TROSAR25.R4", (-0.22, -0.01, 154.88)),
    //     ] {
    //         let fakenow = Epoch::from_gregorian_utc_at_midnight(2023, 01, 01);
    //         let apc = merged.rx_antenna_apc_offset(
    //             fakenow,
    //             AntennaMatcher::IGSCode(name.to_string()),
    //             Carrier::L1,
    //         );
    //         assert!(apc.is_some(), "APC should still be contained after merge()");
    //         assert_eq!(apc.unwrap(), expected_apc);
    //     }
    // }
}
