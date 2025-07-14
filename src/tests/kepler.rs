use log::info;

use crate::{
    navigation::{NavFrameType, NavMessageType},
    prelude::{Constellation, Epoch, Rinex, TimeScale, SV},
    tests::init_logger,
    tests::toolkit::{generic_navigation_test, TimeFrame},
};

use hifitime::Unit;

use std::{path::PathBuf, str::FromStr};

#[test]
fn v3_kepler_nav() {
    init_logger();

    const GPS_MAX_ERRORS_M: (f64, f64, f64) = (0.99, 0.88, 0.54);
    const GAL_MAX_ERRORS_M: (f64, f64, f64) = (0.13, 0.89, 0.28);
    const BDS_MAX_ERRORS_M: (f64, f64, f64) = (1.51, 1.56, 1.86);
    const BDS_IGSO_MAX_ERRORS_M: (f64, f64, f64) = (0.1, 0.1, 0.1);
    const GEO_MAX_ERRORS_M: (f64, f64, f64) = (0.1, 0.1, 0.1);
    const QZS_GEO_MAX_ERRORS_M: (f64, f64, f64) = (1.82, 1.94, 2.14);
    const QZS_MEO_MAX_ERRORS_M: (f64, f64, f64) = (0.38, 0.42, 0.93);

    let dut = Rinex::from_gzip_file("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    for (t_str, x_km, x_tol_m, y_km, y_tol_m, z_km, z_tol_m, sat) in [
        (
            "2020-06-25T02:00:00 GPST",
            -12792.677331,
            GPS_MAX_ERRORS_M.0,
            -12271.088242,
            GPS_MAX_ERRORS_M.1,
            19940.585214,
            GPS_MAX_ERRORS_M.2,
            "G10",
        ),
        (
            "2020-06-25T02:15:00 GPST",
            -10518.543139,
            GPS_MAX_ERRORS_M.0,
            -12708.987728,
            GPS_MAX_ERRORS_M.1,
            20952.929790,
            GPS_MAX_ERRORS_M.2,
            "G10",
        ),
        (
            "2020-06-25T02:30:00 GPST",
            -8177.521591,
            GPS_MAX_ERRORS_M.0,
            -13288.569687,
            GPS_MAX_ERRORS_M.1,
            21609.078377,
            GPS_MAX_ERRORS_M.2,
            "G10",
        ),
        (
            "2020-06-25T04:30:00 GPST",
            14868.084242,
            GAL_MAX_ERRORS_M.0,
            -25589.499327,
            GAL_MAX_ERRORS_M.1,
            -398.009486,
            GAL_MAX_ERRORS_M.2,
            "E30",
        ),
        (
            "2020-06-25T04:45:00 GPST",
            14735.913798,
            GAL_MAX_ERRORS_M.0,
            -25561.045360,
            GAL_MAX_ERRORS_M.1,
            2342.239237,
            GAL_MAX_ERRORS_M.2,
            "E30",
        ),
        (
            "2020-06-25T05:00:00 GPST",
            14502.624816,
            GAL_MAX_ERRORS_M.0,
            -25300.181660,
            GAL_MAX_ERRORS_M.1,
            5053.341253,
            GAL_MAX_ERRORS_M.2,
            "E30",
        ),
        (
            "2020-06-25T01:45:00 GPST",
            -2497.165639,
            BDS_MAX_ERRORS_M.0,
            26286.913334,
            BDS_MAX_ERRORS_M.1,
            33029.820242,
            BDS_MAX_ERRORS_M.2,
            "C10",
        ),
        (
            "2020-06-25T02:00:00 GPST",
            -3513.683409,
            GPS_MAX_ERRORS_M.0,
            26466.369443,
            BDS_MAX_ERRORS_M.1,
            32771.053986,
            BDS_MAX_ERRORS_M.2,
            "C10",
        ),
        (
            "2020-06-25T02:15:00 GPST",
            -4498.027141,
            GPS_MAX_ERRORS_M.0,
            26778.191103,
            BDS_MAX_ERRORS_M.1,
            32372.243103,
            BDS_MAX_ERRORS_M.2,
            "C10",
        ),
        (
            "2020-06-25T02:30:00 GPST",
            -5433.166490,
            BDS_MAX_ERRORS_M.0,
            27216.896111,
            BDS_MAX_ERRORS_M.1,
            31834.918658,
            BDS_MAX_ERRORS_M.2,
            "C10",
        ),
        (
            "2020-06-25T02:45:00 GPST",
            -6302.923067,
            BDS_MAX_ERRORS_M.0,
            27774.815371,
            BDS_MAX_ERRORS_M.1,
            31161.204998,
            BDS_MAX_ERRORS_M.2,
            "C10",
        ),
        (
            "2020-06-25T01:40:16 GPST",
            7.328315920000e+03,
            GEO_MAX_ERRORS_M.0,
            4.148652960000e+04,
            GEO_MAX_ERRORS_M.1,
            1.668688000000e+03,
            GEO_MAX_ERRORS_M.2,
            "S44",
        ),
        (
            "2020-06-25T01:40:15 GPST",
            7.328315920000e+03,
            GEO_MAX_ERRORS_M.0,
            4.148652960000e+04,
            GEO_MAX_ERRORS_M.1,
            1.668688000000e+03,
            GEO_MAX_ERRORS_M.2,
            "S44",
        ),
        (
            "2020-06-25T01:40:17 GPST",
            7.328315920000e+03,
            GEO_MAX_ERRORS_M.0,
            4.148652960000e+04,
            GEO_MAX_ERRORS_M.1,
            1.668688000000e+03,
            GEO_MAX_ERRORS_M.2,
            "S44",
        ),
        (
            "2020-06-25T01:41:30 GPST",
            7.328070160000e+03,
            GEO_MAX_ERRORS_M.0,
            4.148595608000e+04,
            GEO_MAX_ERRORS_M.1,
            1.681703600000e+03,
            GEO_MAX_ERRORS_M.2,
            "S44",
        ),
        (
            "2020-06-25T01:42:00 GPST",
            7.328070160000e+03,
            GEO_MAX_ERRORS_M.0,
            4.148595608000e+04,
            GEO_MAX_ERRORS_M.1,
            1.681703600000e+03,
            GEO_MAX_ERRORS_M.2,
            "S44",
        ),
        (
            "2020-06-25T01:42:15 GPST",
            7.328070160000e+03,
            GEO_MAX_ERRORS_M.0,
            4.148595608000e+04,
            GEO_MAX_ERRORS_M.1,
            1.681703600000e+03,
            GEO_MAX_ERRORS_M.2,
            "S44",
        ),
        (
            "2020-06-25T01:44:00 GPST",
            7.327813680000e+03,
            GEO_MAX_ERRORS_M.0,
            4.148538648000e+04,
            GEO_MAX_ERRORS_M.1,
            1.694572800000e+03,
            GEO_MAX_ERRORS_M.2,
            "S44",
        ),
        (
            "2020-06-25T01:44:30 GPST",
            7.327813680000e+03,
            GEO_MAX_ERRORS_M.0,
            4.148538648000e+04,
            GEO_MAX_ERRORS_M.1,
            1.694572800000e+03,
            GEO_MAX_ERRORS_M.2,
            "S44",
        ),
        (
            "2020-06-25T01:44:31 GPST",
            7.327813680000e+03,
            GEO_MAX_ERRORS_M.0,
            4.148538648000e+04,
            GEO_MAX_ERRORS_M.1,
            1.694572800000e+03,
            GEO_MAX_ERRORS_M.2,
            "S44",
        ),
        (
            "2020-06-25T01:44:32 GPST",
            7.327813680000e+03,
            GEO_MAX_ERRORS_M.0,
            4.148538648000e+04,
            GEO_MAX_ERRORS_M.1,
            1.694572800000e+03,
            GEO_MAX_ERRORS_M.2,
            "S44",
        ),
        (
            "2020-06-25T01:44:33 GPST",
            7.327813680000e+03,
            GEO_MAX_ERRORS_M.0,
            4.148538648000e+04,
            GEO_MAX_ERRORS_M.1,
            1.694572800000e+03,
            GEO_MAX_ERRORS_M.2,
            "S44",
        ),
        (
            "2020-06-25T01:45:00 GPST",
            7.327813680000e+03,
            GEO_MAX_ERRORS_M.0,
            4.148538648000e+04,
            GEO_MAX_ERRORS_M.1,
            1.694572800000e+03,
            GEO_MAX_ERRORS_M.2,
            "S44",
        ),
        (
            "2020-06-25T10:00:00 GPST",
            -26211.575161,
            QZS_GEO_MAX_ERRORS_M.0,
            24731.423594,
            QZS_GEO_MAX_ERRORS_M.1,
            26900.289229,
            QZS_GEO_MAX_ERRORS_M.2,
            "J01",
        ),
        // TODO: J01 is GEO
        // (
        //     "2020-06-25T10:00:15 GPST",
        // -25928.517029,
        //     QZS_GEO_MAX_ERRORS_M.0,
        // 24355.651730 ,
        //     QZS_GEO_MAX_ERRORS_M.1,
        // 27655.563146,
        //     QZS_GEO_MAX_ERRORS_M.2,
        //     "J01",
        // ),
        (
            "2020-06-25T00:00:00 QZSST",
            -30739.759841,
            QZS_MEO_MAX_ERRORS_M.0,
            23143.439761,
            QZS_MEO_MAX_ERRORS_M.1,
            21995.268033,
            QZS_MEO_MAX_ERRORS_M.2,
            "J02",
        ),
        (
            "2020-06-25T00:15:00 QZSST",
            -30979.619845,
            QZS_MEO_MAX_ERRORS_M.0,
            23764.394643,
            QZS_MEO_MAX_ERRORS_M.1,
            20654.453725,
            QZS_MEO_MAX_ERRORS_M.2,
            "J02",
        ),
        // TODO J01 is GEO
        //        (
        //            "2020-06-25T01:00:00 QZSST",
        //            QZS_MEO_MAX_ERRORS_M.0,
        //-25784.804949  ,
        //            QZS_MEO_MAX_ERRORS_M.1,
        //26743.569219 ,
        //            QZS_MEO_MAX_ERRORS_M.2,
        //24929.879665,
        //            "J03",
        //        ),
        //        (
        //            "2020-06-25T01:15:00 QZSST",
        //            QZS_MEO_MAX_ERRORS_M.0,
        //-25418.527916  ,
        //            QZS_MEO_MAX_ERRORS_M.1,
        //26413.769178 ,
        //            QZS_MEO_MAX_ERRORS_M.2,
        //25835.481625  ,
        //            "J03",
        //        ),
        //         (
        //             "2020-06-25T11:00:00 BDT",
        // -15997.510837  ,
        //             BDS_IGSO_MAX_ERRORS_M.0,
        // 38379.433584   ,
        //             BDS_IGSO_MAX_ERRORS_M.1,
        // 8686.890749   ,
        //             BDS_IGSO_MAX_ERRORS_M.2,
        //             "C06",
        //         ),
    ] {
        let t_gpst = Epoch::from_str(t_str).unwrap();

        let sv = SV::from_str(sat).unwrap();

        let (_, _, eph) = dut.nav_ephemeris_selection(sv, t_gpst).unwrap_or_else(|| {
            panic!("{}({}) - failed to select an ephemeris frame", t_str, sat);
        });

        let pos_vel_km = eph
            .sv_position_velocity_km(sv, t_gpst, t_gpst, 10)
            .unwrap_or_else(|e| {
                panic!("{}({}) - kepler solver failed with: {}", t_str, sat, e);
            });

        let (x_err, y_err, z_err) = (
            (pos_vel_km[0] - x_km).abs() * 1.0E3,
            (pos_vel_km[1] - y_km).abs() * 1.0E3,
            (pos_vel_km[2] - z_km).abs() * 1.0E3,
        );

        assert!(
            x_err < x_tol_m,
            "failed for {}({}) x_err={}m",
            t_str,
            sat,
            x_err
        );

        assert!(
            y_err < y_tol_m,
            "failed for {}({}) y_err={}m",
            t_str,
            sat,
            y_err
        );

        assert!(
            z_err < z_tol_m,
            "failed for {}({}) z_err={}m",
            t_str,
            sat,
            z_err
        );

        info!(
            "{}({}) - kepler2pos x_err={}m y_err={}m z_err={}m",
            t_str, sat, x_err, y_err, z_err
        );
    }
}

#[test]
fn v3_clock_nav() {
    init_logger();

    let dut = Rinex::from_gzip_file("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    for (t_str, sat, dt) in [
        ("2020-06-25T02:00:00 GPST", "G10", -12792.677331),
        ("2020-06-25T02:15:00 GPST", "G10", -10518.543139),
        ("2020-06-25T02:30:00 GPST", "G10", 21609.078377),
        ("2020-06-25T04:30:00 GPST", "E30", -398.009486),
        ("2020-06-25T04:45:00 GPST", "E30", 2342.239237),
        ("2020-06-25T05:00:00 GPST", "S30", 14502.624816),
        ("2020-06-25T05:00:00 GPST", "E30", 14502.624816),
        ("2020-06-25T01:45:00 GPST", "C10", -2497.165639),
        ("2020-06-25T02:00:00 GPST", "C10", 32771.053986),
        ("2020-06-25T02:15:00 GPST", "C10", 32372.243103),
        ("2020-06-25T02:30:00 GPST", "C10", 31834.918658),
        ("2020-06-25T02:45:00 GPST", "C10", 31161.204998),
        ("2020-06-25T02:00:00 GPST", "J10", 32771.053986),
        ("2020-06-25T02:15:00 GPST", "J10", 32372.243103),
        ("2020-06-25T02:30:00 GPST", "J10", 31834.918658),
        ("2020-06-25T02:45:00 GPST", "J10", 31161.204998),
    ] {
        let t_gpst = Epoch::from_str(t_str).unwrap();
    }
}
