#[cfg(test)]
mod test {
    use crate::prelude::*;
    use itertools::Itertools;
    use qc_traits::{Filter, FilterItem, MaskOperand, Preprocessing};
    use std::str::FromStr;

    #[test]
    #[cfg(feature = "flate2")]
    fn obs_gnss_v3_esbcdnk() {
        let rnx = Rinex::from_gzip_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
            .unwrap();

        let mask = Filter::mask(
            MaskOperand::Equals,
            FilterItem::ConstellationItem(vec![Constellation::GPS]),
        );

        let dut = rnx.filter(&mask);
        assert_eq!(dut.constellations_iter().count(), 1, "mask:constel failed");
        assert_eq!(
            dut.satellite_iter().count(),
            31,
            "mask:constel - wrong # of SV"
        );

        let mask = Filter::mask(
            MaskOperand::Equals,
            FilterItem::ConstellationItem(vec![Constellation::SBAS]),
        );

        let dut = rnx.filter(&mask);
        assert_eq!(
            dut.constellations_iter().count(),
            3,
            "mask:constell(SBAS) failed"
        );
        assert_eq!(
            dut.satellite_iter().count(),
            5,
            "mask:constell(SBAS) failed"
        );
    }

    #[test]
    fn obs_sv_v3_duth0630() {
        let rnx = Rinex::from_file("data/OBS/V3/DUTH0630.22O").unwrap();
        let mask = Filter::mask(
            MaskOperand::Equals,
            FilterItem::SvItem(vec![
                SV::new(Constellation::GPS, 1),
                SV::new(Constellation::GPS, 3),
            ]),
        );
        let rnx = rnx.filter(&mask);
        assert_eq!(rnx.satellite_iter().count(), 2);
    }

    #[test]
    fn obs_gnss_v3_duth0630() {
        let mut rnx = Rinex::from_file("data/OBS/V3/DUTH0630.22O").unwrap();
        let mask = Filter::mask(
            MaskOperand::Equals,
            FilterItem::ConstellationItem(vec![Constellation::GPS]),
        );
        rnx.filter_mut(&mask);
        assert_eq!(rnx.satellite_iter().count(), 12);
    }
    #[test]
    fn meteo_obsrv_v2_clar0020() {
        let rnx = Rinex::from_file("data/MET/V2/clar0020.00m").unwrap();
        assert_eq!(rnx.observables_iter().count(), 3);

        let pressure = Filter::mask(
            MaskOperand::Equals,
            FilterItem::ComplexItem(vec!["PR".to_string()]),
        );

        let dut = rnx.filter(&pressure);
        assert_eq!(dut.observables_iter().count(), 1);

        let temp = Filter::mask(
            MaskOperand::Equals,
            FilterItem::ComplexItem(vec!["TD".to_string()]),
        );
        let dut = rnx.filter(&temp);
        assert_eq!(dut.observables_iter().count(), 1);

        let moist = Filter::mask(
            MaskOperand::Equals,
            FilterItem::ComplexItem(vec!["HR".to_string()]),
        );
        let dut = rnx.filter(&moist);
        assert_eq!(dut.observables_iter().count(), 1);

        let none = Filter::mask(
            MaskOperand::Equals,
            FilterItem::ComplexItem(vec!["L1C".to_string()]),
        );
        let dut = rnx.filter(&none);
        assert_eq!(dut.observables_iter().count(), 0);
    }

    #[test]
    fn obs_observ_v3_duth0630() {
        let rinex = Rinex::from_file("data/OBS/V3/DUTH0630.22O").unwrap();

        let total = rinex.observables_iter().count();
        assert_eq!(total, 12);

        let pr_l1 = Filter::mask(
            MaskOperand::Equals,
            FilterItem::ComplexItem(vec!["C1C".to_string()]),
        );
        let rnx = rinex.filter(&pr_l1);
        assert_eq!(rnx.observables_iter().count(), 1);

        let pr_dop_l1 = Filter::mask(
            MaskOperand::Equals,
            FilterItem::ComplexItem(vec!["C1C".to_string(), "D1C".to_string()]),
        );
        let rnx = rinex.filter(&pr_dop_l1);
        assert_eq!(rnx.observables_iter().count(), 2);

        let pr_l1_dop_l2 = Filter::mask(
            MaskOperand::Equals,
            FilterItem::ComplexItem(vec!["C1C".to_string(), "D2W".to_string()]),
        );
        let rnx = rinex.filter(&pr_l1_dop_l2);
        assert_eq!(rnx.observables_iter().count(), 2);

        let not_pr_l1 = Filter::mask(
            MaskOperand::NotEquals,
            FilterItem::ComplexItem(vec!["C1C".to_string()]),
        );
        let rnx = rinex.filter(&not_pr_l1);
        assert_eq!(rnx.observables_iter().count(), total - 1);

        let not_pr_l1_dop_l2 = Filter::mask(
            MaskOperand::NotEquals,
            FilterItem::ComplexItem(vec!["C1C".to_string(), "D2W".to_string()]),
        );
        let rnx = rinex.filter(&not_pr_l1_dop_l2);
        assert_eq!(rnx.observables_iter().count(), total - 2);
    }

    #[test]
    fn meteo_time_v2_cari0010() {
        let rnx = Rinex::from_file("data/MET/V2/cari0010.07m").unwrap();

        let mask = Filter::mask(
            MaskOperand::GreaterThan,
            FilterItem::EpochItem(Epoch::from_str("2000-01-02T22:00:00 UTC").unwrap()),
        );
        let dut = rnx.filter(&mask);
        assert_eq!(dut.epoch_iter().count(), 0);

        let mask = Filter::mask(
            MaskOperand::LowerEquals,
            FilterItem::EpochItem(Epoch::from_str("1996-04-02T00:00:00 UTC").unwrap()),
        );
        let dut = rnx.filter(&mask);
        assert_eq!(dut.epoch_iter().count(), 3);

        let mask = Filter::mask(
            MaskOperand::LowerEquals,
            FilterItem::EpochItem(Epoch::from_str("1996-04-01T00:00:30 UTC").unwrap()),
        );
        let dut = rnx.filter(&mask);
        assert_eq!(dut.epoch_iter().count(), 2);

        let mask = Filter::mask(
            MaskOperand::LowerThan,
            FilterItem::EpochItem(Epoch::from_str("1996-04-01T00:00:30 UTC").unwrap()),
        );
        let dut = rnx.filter(&mask);
        assert_eq!(dut.epoch_iter().count(), 1);
    }

    #[test]
    fn obs_epoch_v3_vlns0630() {
        let rinex = Rinex::from_file("data/CRNX/V3/VLNS0630.22D").unwrap();

        let before = Filter::greater_than("2022-03-03T00:00:00 GPST").unwrap();
        let first_eq = Filter::greater_equals("2022-03-04T00:00:00 GPST").unwrap();
        let second = Filter::greater_equals("2022-03-04T00:00:30 GPST").unwrap();

        let dut = rinex.filter(&before);
        assert_eq!(dut.epoch_iter().count(), 2);

        let dut = rinex.filter(&first_eq);
        assert_eq!(dut.epoch_iter().count(), 2);

        let dut = rinex.filter(&second);
        assert_eq!(dut.epoch_iter().count(), 1);

        let numsat = rinex.satellite_iter().collect::<Vec<_>>().len();

        let g01_eq = Filter::equals("G01").unwrap();
        let dut = rinex.filter(&g01_eq);
        let num_dut = dut.satellite_iter().collect::<Vec<_>>().len();
        assert_eq!(num_dut, 1);

        let g01_ineq = Filter::not_equals("G01").unwrap();
        let dut = rinex.filter(&g01_ineq);
        let num_dut = dut.satellite_iter().collect::<Vec<_>>().len();
        assert_eq!(num_dut, numsat - 1);

        let num_gps_sat = rinex
            .satellite_iter()
            .filter(|sv| sv.constellation == Constellation::GPS)
            .collect::<Vec<_>>()
            .len();

        let num_glo_sat = rinex
            .satellite_iter()
            .filter(|sv| sv.constellation == Constellation::Glonass)
            .collect::<Vec<_>>()
            .len();

        let gps_eq = Filter::equals("GPS").unwrap();
        let dut = rinex.filter(&gps_eq);
        let constells = dut.constellations_iter().collect::<Vec<_>>();
        assert_eq!(constells, vec![Constellation::GPS]);
        let num_dut = dut.satellite_iter().collect::<Vec<_>>().len();
        assert_eq!(num_dut, num_gps_sat);

        let glo_eq = Filter::equals("GLO").unwrap();
        let dut = rinex.filter(&glo_eq);
        let constells = dut.constellations_iter().collect::<Vec<_>>();
        assert_eq!(constells, vec![Constellation::Glonass]);
        let num_dut = dut.satellite_iter().collect::<Vec<_>>().len();
        assert_eq!(num_dut, num_glo_sat);

        let above_r01 = Filter::greater_than("R01").unwrap();
        let dut = rinex.filter(&above_r01);
        let constells = dut.constellations_iter().collect::<Vec<_>>();
        assert_eq!(constells, vec![Constellation::GPS, Constellation::Glonass]);
        let num_dut = dut.satellite_iter().collect::<Vec<_>>().len();
        assert_eq!(num_dut, num_gps_sat + num_glo_sat - 1);

        let above_eq_r02 = Filter::greater_equals("R02").unwrap();
        let dut = rinex.filter(&above_eq_r02);
        let constells = dut.constellations_iter().collect::<Vec<_>>();
        assert_eq!(constells, vec![Constellation::GPS, Constellation::Glonass]);
        let num_dut = dut.satellite_iter().collect::<Vec<_>>().len();
        assert_eq!(num_dut, num_gps_sat + num_glo_sat - 1);
    }

    #[test]
    #[cfg(feature = "flate2")]
    fn obs_epoch_v3_esbc00dnk() {
        let rinex = Rinex::from_gzip_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
            .unwrap();
        let last_eq = Filter::equals("2020-06-25T23:59:30 GPST").unwrap();
        let last_geq = Filter::greater_equals("2020-06-25T23:59:30 GPST").unwrap();
        let last_gt = Filter::greater_than("2020-06-25T23:59:30 GPST").unwrap();

        let dut = rinex.filter(&last_eq);
        assert_eq!(dut.epoch_iter().count(), 1);

        let dut = rinex.filter(&last_geq);
        assert_eq!(dut.epoch_iter().count(), 1);

        let dut = rinex.filter(&last_gt);
        assert_eq!(dut.epoch_iter().count(), 0);

        // Test BeiDou GEO marsking
        let bds_eq = Filter::equals("BDS").unwrap();
        let bds_meo_bot = Filter::greater_equals("C07").unwrap();
        let bds_meo_top = Filter::lower_equals("C58").unwrap();

        // BeiDou GEO masking
        let dut = rinex
            .filter(&bds_eq)
            .filter(&bds_meo_bot)
            .filter(&bds_meo_top);
        let constell = dut.constellations_iter().collect::<Vec<_>>();
        assert_eq!(constell, vec![Constellation::BeiDou]);
        for sv in dut.satellite_iter() {
            assert!(sv.prn >= 7);
            assert!(sv.prn <= 54);
            assert!(!sv.is_beidou_geo());
        }

        let bds_geo_bot = Filter::lower_than("C06").unwrap();

        // TODO
        let _bds_geo_top = Filter::greater_than("C58").unwrap();

        // BeiDou MEO masking
        let dut = rinex.filter(&bds_eq).filter(&bds_geo_bot);
        let constell = dut.constellations_iter().collect::<Vec<_>>();
        assert_eq!(constell, vec![Constellation::BeiDou]);
        for sv in dut.satellite_iter() {
            assert!(sv.prn < 6);
            assert!(sv.is_beidou_geo());
        }
    }

    #[test]
    fn obs_signals_v3_duth0630() {
        let rinex = Rinex::from_file("data/OBS/V3/DUTH0630.22O").unwrap();
        let total = rinex.carrier_iter().count();
        assert_eq!(total, 4);

        let carriers = rinex.carrier_iter().sorted().collect::<Vec<_>>();
        assert_eq!(
            carriers,
            vec![
                Carrier::L1,
                Carrier::L2,
                Carrier::G1(None),
                Carrier::G2(None),
            ]
        );

        let l1_only = Filter::mask(
            MaskOperand::Equals,
            FilterItem::ComplexItem(vec!["C1C".to_string(), "D1C".to_string()]),
        );
        let dut = rinex.filter(&l1_only);
        assert_eq!(dut.constellations_iter().count(), 2);

        let carriers = dut.carrier_iter().sorted().collect::<Vec<_>>();
        assert_eq!(carriers, vec![Carrier::L1, Carrier::G1(None)]);

        let glo_l2_only = Filter::mask(
            MaskOperand::Equals,
            FilterItem::ComplexItem(vec!["D2P".to_string()]),
        );
        let dut = rinex.filter(&glo_l2_only);
        assert_eq!(dut.constellations_iter().count(), 1);

        let carriers = dut.carrier_iter().sorted().collect::<Vec<_>>();
        assert_eq!(carriers, vec![Carrier::G2(None)]);
    }
}
