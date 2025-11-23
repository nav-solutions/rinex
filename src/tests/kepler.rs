use crate::{
    // navigation::{NavFrameType, NavMessageType},
    prelude::{
        //Constellation,
        Epoch,
        Rinex,
        //TimeScale,
        SV,
    },
    tests::init_logger,
    // tests::toolkit::{generic_navigation_test, TimeFrame},
};

// use hifitime::Unit;
use hifitime::Duration;

use std::{
    // path::PathBuf,
    str::FromStr,
};

use log::{debug, error};

#[test]
fn v3_ephemeris_selection() {
    let dut = Rinex::from_gzip_file("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    for (epoch_str, satellite_str, expected_toc_str) in [
        ("2020-06-24T22:00:00 BDT", "C05", "2020-06-24T22:00:00 BDT"),
        ("2020-06-24T22:00:01 BDT", "C05", "2020-06-24T22:00:00 BDT"),
        ("2020-06-24T21:59:59 BDT", "C05", "2020-06-24T22:00:00 BDT"),
        ("2020-06-24T21:00:00 BDT", "C05", "2020-06-24T22:00:00 BDT"),
        ("2020-06-24T22:00:02 BDT", "C05", "2020-06-24T22:00:00 BDT"),
        ("2020-06-24T22:15:02 BDT", "C05", "2020-06-24T22:00:00 BDT"),
        ("2020-06-24T23:00:00 BDT", "C05", "2020-06-24T23:00:00 BDT"),
        (
            "2020-06-25T00:00:00 GPST",
            "G30",
            "2020-06-25T00:00:00 GPST",
        ),
        (
            "2020-06-25T00:00:01 GPST",
            "G30",
            "2020-06-25T00:00:00 GPST",
        ),
        (
            "2020-06-25T00:00:09 GPST",
            "G30",
            "2020-06-25T00:00:00 GPST",
        ),
        (
            "2020-06-25T01:15:09 GPST",
            "G30",
            "2020-06-25T02:00:00 GPST",
        ),
        (
            "2020-06-25T01:15:09 GPST",
            "G30",
            "2020-06-25T02:00:00 GPST",
        ),
        (
            "2020-06-25T01:45:09 GPST",
            "G30",
            "2020-06-25T02:00:00 GPST",
        ),
        (
            "2020-06-25T01:59:58 GPST",
            "G30",
            "2020-06-25T02:00:00 GPST",
        ),
        (
            "2020-06-25T01:59:59 GPST",
            "G30",
            "2020-06-25T02:00:00 GPST",
        ),
        (
            "2020-06-25T02:00:00 GPST",
            "G30",
            "2020-06-25T02:00:00 GPST",
        ),
        (
            "2020-06-25T02:00:01 GPST",
            "G30",
            "2020-06-25T02:00:00 GPST",
        ),
        ("2020-06-24T22:40:00 GST", "E04", "2020-06-24T22:40:00 GST"),
        ("2020-06-24T22:40:01 GST", "E04", "2020-06-24T22:40:00 GST"),
        ("2020-06-24T22:39:59 GST", "E04", "2020-06-24T22:40:00 GST"),
        ("2020-06-24T22:44:59 GST", "E04", "2020-06-24T22:40:00 GST"),
        ("2020-06-24T22:45:00 GST", "E04", "2020-06-24T22:40:00 GST"),
        ("2020-06-24T22:45:01 GST", "E04", "2020-06-24T22:50:00 GST"),
        ("2020-06-24T22:49:58 GST", "E04", "2020-06-24T22:50:00 GST"),
        ("2020-06-24T22:49:59 GST", "E04", "2020-06-24T22:50:00 GST"),
        (
            "2020-06-25T01:40:16 GPST",
            "S44",
            "2020-06-25T01:40:16 GPST",
        ),
        (
            "2020-06-25T01:40:15 GPST",
            "S44",
            "2020-06-25T01:40:16 GPST",
        ),
        (
            "2020-06-25T01:40:17 GPST",
            "S44",
            "2020-06-25T01:40:16 GPST",
        ),
        (
            "2020-06-25T01:42:20 GPST",
            "S44",
            "2020-06-25T01:42:24 GPST",
        ),
        (
            "2020-06-25T01:42:21 GPST",
            "S44",
            "2020-06-25T01:42:24 GPST",
        ),
        (
            "2020-06-25T01:42:22 GPST",
            "S44",
            "2020-06-25T01:42:24 GPST",
        ),
        (
            "2020-06-25T01:42:23 GPST",
            "S44",
            "2020-06-25T01:42:24 GPST",
        ),
        (
            "2020-06-25T01:42:24 GPST",
            "S44",
            "2020-06-25T01:42:24 GPST",
        ),
        (
            "2020-06-25T01:42:25 GPST",
            "S44",
            "2020-06-25T01:42:24 GPST",
        ),
        (
            "2020-06-25T01:42:26 GPST",
            "S44",
            "2020-06-25T01:42:24 GPST",
        ),
        (
            "2020-06-25T01:42:27 GPST",
            "S44",
            "2020-06-25T01:42:24 GPST",
        ),
        (
            "2020-06-25T01:42:28 GPST",
            "S44",
            "2020-06-25T01:42:24 GPST",
        ),
    ] {
        let epoch = Epoch::from_str(epoch_str).unwrap();
        let satellite = SV::from_str(satellite_str).unwrap();
        let expected_toc = Epoch::from_str(expected_toc_str).unwrap();

        let (toc, _toe, _eph) = dut
            .nav_satellite_ephemeris_selection(satellite, epoch)
            .unwrap();

        assert_eq!(
            toc, expected_toc,
            "test failed for {}@{}",
            satellite_str, epoch_str
        );
    }
}

#[test]
fn v3_kepler_precision() {
    init_logger();
    let g10 = SV::from_str("G10").unwrap();
    let e30 = SV::from_str("E30").unwrap();
    let c10 = SV::from_str("C10").unwrap();

    let dut = Rinex::from_gzip_file("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    // test navigation
    for (t_gpst, x_km, y_km, z_km) in [
        (
            "2020-06-25T02:00:00 GPST",
            -12792.677331,
            -12271.088242,
            19940.585214,
        ),
        (
            "2020-06-25T02:15:00 GPST",
            -10518.543139,
            -12708.987728,
            20952.929790,
        ),
        (
            "2020-06-25T02:30:00 GPST",
            -8177.521591,
            -13288.569687,
            21609.078377,
        ),
    ] {
        let t_gpst = Epoch::from_str(t_gpst).unwrap();

        let (toc, _, eph) = dut.nav_satellite_ephemeris_selection(g10, t_gpst).unwrap();

        let orbit = eph.resolve_orbital_state(g10, toc, t_gpst, 10).unwrap();

        let pos_vel = orbit.to_cartesian_pos_vel();

        let (x_err, y_err, z_err) = (
            (pos_vel[0] - x_km).abs(),
            (pos_vel[1] - y_km).abs(),
            (pos_vel[2] - z_km).abs(),
        );

        assert!(
            x_err < 1.0E-3,
            "failed for {} G10(x) err={} km",
            t_gpst,
            x_err
        );
        assert!(
            y_err < 1.0E-3,
            "failed for {} G10(y) err={} km",
            t_gpst,
            y_err
        );
        assert!(
            z_err < 1.0E-3,
            "failed for {} G10(z) err={} km",
            t_gpst,
            z_err
        );
    }

    for (t_gpst, x_km, y_km, z_km) in [
        (
            "2020-06-25T04:30:00 GPST",
            14868.084242,
            -25589.499327,
            -398.009486,
        ),
        (
            "2020-06-25T04:45:00 GPST",
            14735.913798,
            -25561.045360,
            2342.239237,
        ),
        (
            "2020-06-25T05:00:00 GPST",
            14502.624816,
            -25300.181660,
            5053.341253,
        ),
    ] {
        let t_gpst = Epoch::from_str(t_gpst).unwrap();

        let (toc, _, eph) = dut.nav_satellite_ephemeris_selection(e30, t_gpst).unwrap();

        let orbit = eph.resolve_orbital_state(e30, toc, t_gpst, 10).unwrap();

        let pos_vel = orbit.to_cartesian_pos_vel();

        let (x_err, y_err, z_err) = (
            (pos_vel[0] - x_km).abs(),
            (pos_vel[1] - y_km).abs(),
            (pos_vel[2] - z_km).abs(),
        );

        assert!(
            x_err < 1.0E-3,
            "failed for {} E30(x) err={} km",
            t_gpst,
            x_err
        );
        assert!(
            y_err < 1.0E-3,
            "failed for {} E30(y) err={} km",
            t_gpst,
            y_err
        );
        assert!(
            z_err < 1.0E-3,
            "failed for {} E30(z) err={} km",
            t_gpst,
            z_err
        );
    }

    for (t_gpst, x_km, y_km, z_km) in [
        (
            "2020-06-25T01:45:00 GPST",
            -2497.165639,
            26286.913334,
            33029.820242,
        ),
        (
            "2020-06-25T02:00:00 GPST",
            -3513.683409,
            26466.369443,
            32771.053986,
        ),
        (
            "2020-06-25T02:15:00 GPST",
            -4498.027141,
            26778.191103,
            32372.243103,
        ),
        (
            "2020-06-25T02:30:00 GPST",
            -5433.166490,
            27216.896111,
            31834.918658,
        ),
        (
            "2020-06-25T02:45:00 GPST",
            -6302.923067,
            27774.815371,
            31161.204998,
        ),
    ] {
        let t_gpst = Epoch::from_str(t_gpst).unwrap();

        let (toc, _, eph) = dut.nav_satellite_ephemeris_selection(c10, t_gpst).unwrap();

        let orbit = eph.resolve_orbital_state(c10, toc, t_gpst, 10).unwrap();

        let pos_vel = orbit.to_cartesian_pos_vel();

        let (x_err, y_err, z_err) = (
            (pos_vel[0] - x_km).abs(),
            (pos_vel[1] - y_km).abs(),
            (pos_vel[2] - z_km).abs(),
        );

        assert!(
            x_err < 5.0E-3,
            "failed for {} C10(x) err={} km",
            t_gpst,
            x_err
        );
        assert!(
            y_err < 5.0E-3,
            "failed for {} C10(y) err={} km",
            t_gpst,
            y_err
        );
        assert!(
            z_err < 5.0E-3,
            "failed for {} C10(z) err={} km",
            t_gpst,
            z_err
        );
    }
}

#[test]
fn v3_kepler() {
    init_logger();
    // verifies we can compute for all entries

    let dut = Rinex::from_gzip_file("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    for (key, frame) in dut.nav_ephemeris_frames_iter() {
        // test at toc
        match frame.resolve_orbital_state(key.sv, key.epoch, key.epoch, 10) {
            Ok(state) => {
                debug!("{}({:x}): {}", key.epoch, key.sv, state);
            },
            Err(e) => {
                error!(
                    "{}({:x}): failed to resolve orbital state: {}",
                    key.epoch, key.sv, e
                );
            },
        }

        for delta in [
            Duration::from_seconds(1.0),
            Duration::from_seconds(3.4),
            Duration::from_seconds(19.0),
            Duration::from_seconds(60.0),
            Duration::from_seconds(600.0),
            Duration::from_seconds(3600.0),
            Duration::from_seconds(-1.0),
            Duration::from_seconds(-3.4),
            Duration::from_seconds(-19.0),
            Duration::from_seconds(-60.0),
            Duration::from_seconds(-600.0),
            Duration::from_seconds(-3600.0),
        ] {
            let epoch = key.epoch + delta;

            match frame.resolve_orbital_state(key.sv, key.epoch, epoch, 10) {
                Ok(state) => {
                    if delta.signum() >= 0 {
                        debug!("{}+{}({:x}): {}", key.epoch, delta, key.sv, state);
                    } else {
                        debug!("{}{}({:x}): {}", key.epoch, delta, key.sv, state);
                    }
                },
                Err(e) => {
                    if delta.signum() >= 0 {
                        error!(
                            "{}+{}({:x}): failed to resolve orbital state: {}",
                            key.epoch, delta, key.sv, e
                        );
                    } else {
                        error!(
                            "{}{}({:x}): failed to resolve orbital state: {}",
                            key.epoch, delta, key.sv, e
                        );
                    }
                },
            }
        }
    }
}

#[test]
fn v4_kepler() {
    init_logger();

    // verifies we can compute for all entries
    let dut = Rinex::from_gzip_file("data/NAV/V4/KMS300DNK_R_20221591000_01H_MN.rnx.gz").unwrap();

    for (key, frame) in dut.nav_ephemeris_frames_iter() {
        // test at toc
        match frame.resolve_orbital_state(key.sv, key.epoch, key.epoch, 10) {
            Ok(state) => {
                debug!("{}({:x}): {}", key.epoch, key.sv, state);
            },
            Err(e) => {
                error!(
                    "{}({:x}): failed to resolve orbital state: {}",
                    key.epoch, key.sv, e
                );
            },
        }

        for delta in [
            Duration::from_seconds(1.0),
            Duration::from_seconds(3.2),
            Duration::from_seconds(19.0),
            Duration::from_seconds(60.0),
            Duration::from_seconds(600.0),
            Duration::from_seconds(3600.0),
            Duration::from_seconds(-1.0),
            Duration::from_seconds(-3.2),
            Duration::from_seconds(-19.0),
            Duration::from_seconds(-60.0),
            Duration::from_seconds(-600.0),
            Duration::from_seconds(-3600.0),
        ] {
            let epoch = key.epoch + delta;

            match frame.resolve_orbital_state(key.sv, key.epoch, epoch, 10) {
                Ok(state) => {
                    if delta.signum() >= 0 {
                        debug!("{}+{}({:x}): {}", key.epoch, delta, key.sv, state);
                    } else {
                        debug!("{}{}({:x}): {}", key.epoch, delta, key.sv, state);
                    }
                },
                Err(e) => {
                    if delta.signum() >= 0 {
                        error!(
                            "{}+{}({:x}): failed to resolve orbital state: {}",
                            key.epoch, delta, key.sv, e
                        );
                    } else {
                        error!(
                            "{}{}({:x}): failed to resolve orbital state: {}",
                            key.epoch, delta, key.sv, e
                        );
                    }
                },
            }
        }
    }
}
