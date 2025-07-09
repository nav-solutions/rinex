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

    let dut = Rinex::from_gzip_file("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    for (t_str, x_km, y_km, z_km, sat) in [
        (
            "2020-06-25T02:00:00 GPST",
            -12792.677331,
            -12271.088242,
            19940.585214,
            "G10",
        ),
        (
            "2020-06-25T02:15:00 GPST",
            -10518.543139,
            -12708.987728,
            20952.929790,
            "G10",
        ),
        (
            "2020-06-25T02:30:00 GPST",
            -8177.521591,
            -13288.569687,
            21609.078377,
            "G10",
        ),
        (
            "2020-06-25T04:30:00 GPST",
            14868.084242,
            -25589.499327,
            -398.009486,
            "E30",
        ),
        (
            "2020-06-25T04:45:00 GPST",
            14735.913798,
            -25561.045360,
            2342.239237,
            "E30",
        ),
        (
            "2020-06-25T05:00:00 GPST",
            14502.624816,
            -25300.181660,
            5053.341253,
            "E30",
        ),
        (
            "2020-06-25T01:45:00 GPST",
            -2497.165639,
            26286.913334,
            33029.820242,
            "C10",
        ),
        (
            "2020-06-25T02:00:00 GPST",
            -3513.683409,
            26466.369443,
            32771.053986,
            "C10",
        ),
        (
            "2020-06-25T02:15:00 GPST",
            -4498.027141,
            26778.191103,
            32372.243103,
            "C10",
        ),
        (
            "2020-06-25T02:30:00 GPST",
            -5433.166490,
            27216.896111,
            31834.918658,
            "C10",
        ),
        (
            "2020-06-25T02:45:00 GPST",
            -6302.923067,
            27774.815371,
            31161.204998,
            "C10",
        ),
        (
            "2020-06-25T02:45:00 GPST",
            -6302.923067,
            27774.815371,
            31161.204998,
            "S45",
        ),
        (
            "2020-06-25T02:45:00 GPST",
            -6302.923067,
            27774.815371,
            31161.204998,
            "J01",
        ),
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
            (pos_vel_km[0] - x_km).abs(),
            (pos_vel_km[1] - y_km).abs(),
            (pos_vel_km[2] - z_km).abs(),
        );

        assert!(
            x_err < 1.0E-3,
            "failed for {}({}) x_err={} km",
            t_str,
            sat,
            x_err
        );

        assert!(
            y_err < 1.0E-3,
            "failed for {}({}) y_err={} km",
            t_str,
            sat,
            y_err
        );

        assert!(
            z_err < 1.0E-3,
            "failed for {}({}) z_err={} km",
            t_str,
            sat,
            z_err
        );

        info!(
            "{}({}) - kepler2pos x_err={}km y_err={} z_err={}km",
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
