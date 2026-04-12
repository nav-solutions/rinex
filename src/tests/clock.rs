#[cfg(test)]
mod test {
    use crate::prelude::{clock::*, Constellation, DOMESTrackingPoint, Epoch, Rinex, DOMES, SV};
    use std::str::FromStr;

    #[test]
    fn clk_v2_cod20352() {
        let test_resource = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/CLK/V2/COD20352.CLK";
        let rinex = Rinex::from_file(&test_resource);
        let rinex = rinex.unwrap();
        assert_eq!(rinex.epoch_iter().count(), 10);

        for (epoch, content) in rinex.precise_clock() {
            let epoch_str = epoch.to_string();
            for (key, profile) in content {
                if let Some(sv) = key.clock_type.as_sv() {
                    match sv {
                        SV {
                            constellation: Constellation::Glonass,
                            prn: 10,
                        } => {
                            assert_eq!(key.profile_type, ClockProfileType::AS);
                            match epoch_str.as_str() {
                                "2019-01-08T00:01:30 GPST" => {
                                    assert_eq!(profile.bias, 0.391709678221E-04);
                                    assert!(profile.bias_dev.is_none());
                                    assert!(profile.drift.is_none());
                                    assert!(profile.drift_dev.is_none());
                                    assert!(profile.drift_change.is_none());
                                    assert!(profile.drift_change_dev.is_none());
                                },
                                "2019-01-08T00:02:00 GPST" => {
                                    assert_eq!(profile.bias, 0.391708653726E-04);
                                    assert!(profile.bias_dev.is_none());
                                    assert!(profile.drift.is_none());
                                    assert!(profile.drift_dev.is_none());
                                    assert!(profile.drift_change.is_none());
                                    assert!(profile.drift_change_dev.is_none());
                                },
                                _ => {},
                            }
                        },
                        SV {
                            constellation: Constellation::Glonass,
                            prn: 21,
                        } => {
                            assert_eq!(key.profile_type, ClockProfileType::AS);
                            match epoch_str.as_str() {
                                "2019-01-08T00:00:00 GPST" => {
                                    assert_eq!(profile.bias, -0.243172599885E-04);
                                    assert_eq!(profile.bias_dev, Some(0.850129218038E-11));
                                    assert!(profile.drift.is_none());
                                    assert!(profile.drift_dev.is_none());
                                    assert!(profile.drift_change.is_none());
                                    assert!(profile.drift_change_dev.is_none());
                                },
                                "2019-01-08T00:00:30 GPST" => {
                                    assert_eq!(profile.bias, -0.243173099640E-04);
                                    assert!(profile.bias_dev.is_none());
                                    assert!(profile.drift.is_none());
                                    assert!(profile.drift_dev.is_none());
                                    assert!(profile.drift_change.is_none());
                                    assert!(profile.drift_change_dev.is_none());
                                },
                                "2019-01-08T00:01:00 GPST" => {
                                    assert_eq!(profile.bias, -0.243174034292E-04);
                                    assert!(profile.bias_dev.is_none());
                                    assert!(profile.drift.is_none());
                                    assert!(profile.drift_dev.is_none());
                                    assert!(profile.drift_change.is_none());
                                    assert!(profile.drift_change_dev.is_none());
                                },
                                "2019-01-08T00:01:30 GPST" => {
                                    assert_eq!(profile.bias, -0.243174284491E-04);
                                    assert!(profile.bias_dev.is_none());
                                    assert!(profile.drift.is_none());
                                    assert!(profile.drift_dev.is_none());
                                    assert!(profile.drift_change.is_none());
                                    assert!(profile.drift_change_dev.is_none());
                                },
                                "2019-01-08T00:02:00 GPST" => {
                                    assert_eq!(profile.bias, -0.243175702770E-04);
                                    assert!(profile.bias_dev.is_none());
                                    assert!(profile.drift.is_none());
                                    assert!(profile.drift_dev.is_none());
                                    assert!(profile.drift_change.is_none());
                                    assert!(profile.drift_change_dev.is_none());
                                },
                                "2019-01-08T00:02:30 GPST" => {
                                    assert_eq!(profile.bias, -0.243176490245E-04);
                                    assert!(profile.bias_dev.is_none());
                                    assert!(profile.drift.is_none());
                                    assert!(profile.drift_dev.is_none());
                                    assert!(profile.drift_change.is_none());
                                    assert!(profile.drift_change_dev.is_none());
                                },
                                "2019-01-08T00:03:00 GPST" => {
                                    assert_eq!(profile.bias, -0.243176769102E-04);
                                    assert!(profile.bias_dev.is_none());
                                    assert!(profile.drift.is_none());
                                    assert!(profile.drift_dev.is_none());
                                    assert!(profile.drift_change.is_none());
                                    assert!(profile.drift_change_dev.is_none());
                                },
                                "2019-01-08T00:03:30 GPST" => {
                                    assert_eq!(profile.bias, -0.243177259494E-04);
                                    assert!(profile.bias_dev.is_none());
                                    assert!(profile.drift.is_none());
                                    assert!(profile.drift_dev.is_none());
                                    assert!(profile.drift_change.is_none());
                                    assert!(profile.drift_change_dev.is_none());
                                },
                                "2019-01-08T10:00:00 GPST" => {
                                    assert_eq!(profile.bias, -0.243934947986E-04);
                                    assert_eq!(profile.bias_dev, Some(0.846286338370E-11));
                                    assert!(profile.drift.is_none());
                                    assert!(profile.drift_dev.is_none());
                                    assert!(profile.drift_change.is_none());
                                    assert!(profile.drift_change_dev.is_none());
                                },
                                _ => {},
                            }
                        },
                        _ => {},
                    }
                }
            }
        }
    }
    #[test]
    fn clk_v3_usno() {
        let test_resource = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/CLK/V3/USNO1.txt";
        let rinex = Rinex::from_file(&test_resource);

        let rinex = rinex.unwrap();
        let clock_header = rinex.header.clock.as_ref().expect("badly formed clk rinex");

        assert_eq!(
            clock_header.codes,
            vec![
                ClockProfileType::AS,
                ClockProfileType::AR,
                ClockProfileType::CR,
                ClockProfileType::DR,
            ],
            "badly identified clock data"
        );

        assert_eq!(clock_header.igs, Some("USN".to_string()));
        assert_eq!(clock_header.site, Some("USNO".to_string()));
        assert_eq!(
            clock_header.domes,
            Some(DOMES {
                area: 404,
                site: 51,
                point: DOMESTrackingPoint::Instrument,
                sequential: 3,
            })
        );
        assert_eq!(
            clock_header.full_name,
            Some("USNO USING GIPSY/OASIS-II".to_string())
        );
        assert_eq!(
            clock_header.ref_clock,
            Some("UTC(USNO) MASTER CLOCK VIA CONTINUOUS CABLE MONITOR".to_string())
        );

        assert_eq!(
            clock_header.work_clock,
            vec![
                WorkClock {
                    name: "USNO".to_string(),
                    domes: Some(DOMES {
                        area: 404,
                        site: 51,
                        point: DOMESTrackingPoint::Instrument,
                        sequential: 3,
                    }),
                    constraint: Some(-0.123456789012),
                },
                WorkClock {
                    name: "TIBD".to_string(),
                    domes: Some(DOMES {
                        area: 501,
                        site: 3,
                        point: DOMESTrackingPoint::Monument,
                        sequential: 108,
                    }),
                    constraint: Some(-0.123456789012),
                },
            ]
        );

        assert_eq!(rinex.epoch_iter().count(), 1);

        for (epoch, content) in rinex.precise_clock() {
            assert_eq!(*epoch, Epoch::from_str("1994-07-14T20:59:00 GPST").unwrap());
            for (key, profile) in content {
                match key.profile_type {
                    ClockProfileType::AR => {
                        assert_eq!(key.clock_type, ClockType::Station("AREQ".to_string()));
                        assert_eq!(profile.bias, -0.123456789012E+00);
                        assert_eq!(profile.bias_dev, Some(-0.123456789012E+01));
                        assert_eq!(profile.drift, Some(-0.123456789012E+02));
                        assert_eq!(profile.drift_dev, Some(-0.123456789012E+03));
                        assert_eq!(profile.drift_change, Some(-0.123456789012E+04));
                        assert_eq!(profile.drift_change_dev, Some(-0.123456789012E+05));
                    },
                    ClockProfileType::AS => {
                        assert_eq!(key.clock_type, ClockType::SV(SV::from_str("G16").unwrap()));
                        assert_eq!(profile.bias, -0.123456789012E+00);
                        assert_eq!(profile.bias_dev, Some(-0.123456789012E+01));
                        assert!(profile.drift.is_none());
                        assert!(profile.drift_dev.is_none());
                        assert!(profile.drift_change.is_none());
                        assert!(profile.drift_change_dev.is_none());
                    },
                    ClockProfileType::CR => {
                        assert_eq!(key.clock_type, ClockType::Station("USNO".to_string()));
                        assert_eq!(profile.bias, -0.123456789012E+00);
                        assert_eq!(profile.bias_dev, Some(-0.123456789012E+01));
                        assert!(profile.drift.is_none());
                        assert!(profile.drift_dev.is_none());
                        assert!(profile.drift_change.is_none());
                        assert!(profile.drift_change_dev.is_none());
                    },
                    ClockProfileType::DR => {
                        assert_eq!(key.clock_type, ClockType::Station("USNO".to_string()));
                        assert_eq!(profile.bias, -0.123456789012E+00);
                        assert_eq!(profile.bias_dev, Some(-0.123456789012E+01));
                        assert_eq!(profile.drift, Some(-0.123456789012E-03));
                        assert_eq!(profile.drift_dev, Some(-0.123456789012E-04));
                        assert!(profile.drift_change.is_none());
                        assert!(profile.drift_change_dev.is_none());
                    },
                    _ => panic!("decoded unexpected content"),
                }
            }
        }
    }
    #[test]
    fn clk_v3_04_example1() {
        let test_resource = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/CLK/V3/example1.txt";
        let rinex = Rinex::from_file(&test_resource);
        assert!(rinex.is_ok());
        let rinex = rinex.unwrap();

        let clock_header = rinex.header.clock.as_ref().expect("badly formed clk rinex");

        assert_eq!(
            clock_header.codes,
            vec![ClockProfileType::AS, ClockProfileType::AR,],
            "badly identified clock data"
        );

        assert_eq!(
            clock_header.work_clock,
            vec![
                WorkClock {
                    name: "USNO".to_string(),
                    domes: Some(DOMES {
                        area: 404,
                        site: 51,
                        point: DOMESTrackingPoint::Instrument,
                        sequential: 3,
                    }),
                    constraint: Some(-0.123456789012),
                },
                WorkClock {
                    name: "TIDB".to_string(),
                    domes: Some(DOMES {
                        area: 501,
                        site: 3,
                        point: DOMESTrackingPoint::Monument,
                        sequential: 108,
                    }),
                    constraint: Some(-0.123456789012),
                },
            ]
        );

        assert_eq!(rinex.epoch_iter().count(), 1);
    }
    #[test]
    fn clk_v3_04_example2() {
        let test_resource = env!("CARGO_MANIFEST_DIR").to_owned() + "/data/CLK/V3/example2.txt";
        let rinex = Rinex::from_file(&test_resource);
        assert!(rinex.is_ok());
        let rinex = rinex.unwrap();

        assert_eq!(rinex.epoch_iter().count(), 1);
    }
}
