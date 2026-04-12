#[cfg(test)]
#[cfg(feature = "antex")]
mod test {
    use crate::antex::pcv::Pcv;
    use crate::antex::AntennaMatcher;
    use crate::antex::CalibrationMethod;
    use crate::carrier::Carrier;
    use crate::linspace::Linspace;
    use crate::prelude::*;
    use std::str::FromStr;
    #[test]
    fn v1_trosar_25r4_leit_2020_09_23() {
        let test_resource =
            env!("CARGO_MANIFEST_DIR").to_owned() + "/data/ATX/V1/TROSAR25.R4__LEIT_2020_09_23.atx";
        let rinex = Rinex::from_file(&test_resource);
        assert!(rinex.is_ok());
        let rinex = rinex.unwrap();
        assert!(rinex.is_antex());

        let header = &rinex.header;
        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 4);
        assert!(header.antex.is_some());

        let atx_header = header.antex.as_ref().unwrap();
        assert_eq!(atx_header.pcv_type, Pcv::Absolute);

        /*
         * record test
         */
        let record = rinex.record.as_antex();
        assert!(record.is_some());
        let record = record.unwrap();

        assert_eq!(record.len(), 1);
        let (antenna, freq_data) = record.first().unwrap();

        assert_eq!(antenna.calibration.method, CalibrationMethod::Chamber);
        assert_eq!(antenna.calibration.agency, "IGG, Univ. Bonn");
        assert_eq!(antenna.calibration.number, 1);

        assert_eq!(
            antenna.calibration.date,
            Epoch::from_str("2023-09-20T00:00:00 UTC").unwrap()
        );

        assert_eq!(
            antenna.zenith_grid,
            Linspace {
                start: 0.0,
                end: 90.0,
                spacing: 5.0,
            }
        );

        // specs for 3 freqz
        assert_eq!(freq_data.len(), 3);

        // L1 frequency
        assert!(
            freq_data.get(&Carrier::L1).is_some(),
            "missing specs for L1 frequency"
        );
        let l1_specs = freq_data.get(&Carrier::L1).unwrap();
        assert_eq!(
            l1_specs.apc_eccentricity,
            (-0.22, -0.01, 154.88),
            "bad APC for L1 frequency"
        );

        // L5 frequency
        assert!(
            freq_data.get(&Carrier::L5).is_some(),
            "missing specs for L5 frequency"
        );
        let l5_specs = freq_data.get(&Carrier::L5).unwrap();
        assert_eq!(
            l5_specs.apc_eccentricity,
            (0.34, -0.62, 164.34),
            "bad APC for L5 frequency"
        );

        // B2B frequency
        assert!(
            freq_data.get(&Carrier::B2b).is_some(),
            "missing specs for B2B frequency"
        );

        let b2b_specs = freq_data.get(&Carrier::B2b).unwrap();
        assert_eq!(
            b2b_specs.apc_eccentricity,
            (0.32, -0.63, 160.39),
            "bad APC for B2B frequency"
        );

        /*
         * crate feature: RX antenna location
         */
        let fake_now = Epoch::from_gregorian_utc_at_midnight(2023, 01, 01);
        let apc = rinex.rx_antenna_apc_offset(
            fake_now,
            AntennaMatcher::IGSCode("trosar25.r4".to_string()),
            Carrier::L1,
        );
        assert!(
            apc.is_some(),
            "failed to locate APC for TROSAR25.R4 antenna"
        );
        assert_eq!(apc.unwrap(), (-0.22, -0.01, 154.88));
    }
    #[cfg(feature = "antex")]
    #[cfg(feature = "flate2")]
    #[test]
    fn v1_4_igs_atx() {
        let test_resource =
            env!("CARGO_MANIFEST_DIR").to_owned() + "/data/ATX/V1/igs14_small.atx.gz";

        let rinex = Rinex::from_gzip_file(&test_resource).unwrap();

        let fake_now = Epoch::from_gregorian_utc_at_midnight(2023, 01, 01);

        for (antenna, expected) in [
            ("JPSLEGANT_E", (1.36, -0.43, 35.44)),
            ("JPSODYSSEY_I", (1.06, -2.43, 70.34)),
        ] {
            let apc = rinex.rx_antenna_apc_offset(
                fake_now,
                AntennaMatcher::IGSCode(antenna.to_string()),
                Carrier::L1,
            );
            assert!(apc.is_some(), "failed to locate APC {} antenna", antenna,);
            assert_eq!(apc.unwrap(), expected);
        }
    }
}
