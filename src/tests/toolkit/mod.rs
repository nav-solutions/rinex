use crate::*;
use rand::{distr::Alphanumeric, Rng};

use crate::hardware::Antenna;

// OBS RINEX dedicated tools
mod observation;
pub use observation::{
    generic_comparison as generic_observation_comparison, generic_observation_epoch_decoding_test,
    generic_observation_rinex_test, SignalDataPoint,
};

// NAV RINEX dedicated tools
#[cfg(feature = "nav")]
mod nav;

#[cfg(feature = "nav")]
pub use nav::{
    generic_comparison as generic_navigation_comparison, generic_test as generic_navigation_test,
};

// Meteo RINEX dedicated tests
#[cfg(feature = "meteo")]
mod meteo;

#[cfg(feature = "meteo")]
pub use meteo::{generic_comparison as generic_meteo_comparison, generic_meteo_rinex_test};

pub mod timeframe;
pub use timeframe::TimeFrame;

pub mod csv;
pub use csv::{gnss_csv, observables_csv, sv_csv};

/// Random name generator
pub fn random_name(size: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(size)
        .map(char::from)
        .collect()
}

/// Generic sampling test
pub fn generic_timeframe_test(dut: &Rinex, tf: TimeFrame) {
    // grab epoch iter
    let mut dut = dut.epoch_iter();
    let mut model = tf.into_iter();

    while let Some(model) = model.next() {
        if let Some(dut) = dut.next() {
            assert_eq!(model, dut);
        } else {
            panic!("missing temporal data {:?}", model);
        }
    }

    let next = dut.next();
    assert!(
        next.is_none(),
        "timeframe exceeded! unexpected content {:?}",
        next.unwrap()
    );
}

/// RX antenna tb
/// {0, 0, 0} and No coordinates have the same meaning
fn rcvr_antenna_comparison(dut: &Option<Antenna>, model: &Option<Antenna>) {
    if let Some(dut) = dut {
        if let Some(model) = model {
            assert_eq!(dut.model, model.model);
            assert_eq!(dut.sn, model.sn);

            let model_h = model.height.unwrap_or(0.0);
            let dut_h = dut.height.unwrap_or(0.0);
            assert_eq!(dut_h, model_h);

            assert_eq!(dut.coords, model.coords);

            let (model_east, model_north) =
                (model.eastern.unwrap_or(0.0), model.northern.unwrap_or(0.0));
            let (dut_east, dut_north) = (dut.eastern.unwrap_or(0.0), dut.northern.unwrap_or(0.0));

            assert_eq!(dut_east, model_east);
            assert_eq!(dut_north, model_north);
        } else {
            panic!("found unexpected rx-antenna infos!");
        }
    } else {
        if model.is_some() {
            panic!("missing rx-antenna infos!");
        }
    }
}

/// Generic test that may apply to any [Rinex].
/// Verifies
///   - [Version]
///   - [Constellation]
///   - [RinexType]
///   - Possible [TimeFrame] for complex sampling testing
pub fn generic_rinex_test(
    dut: &Rinex,
    version: &str,
    constellation: Option<&str>,
    expected_type: RinexType,
    timeframe: Option<TimeFrame>,
) {
    assert_eq!(dut.header.rinex_type, expected_type);

    let version = Version::from_str(version).unwrap();
    assert_eq!(dut.header.version, version);

    let constellation = constellation.map(|s| Constellation::from_str(s.trim()).unwrap());
    assert_eq!(dut.header.constellation, constellation);

    if let Some(tf) = timeframe {
        generic_timeframe_test(dut, tf);
    }
}

/// Verifies that all contained data is Constant with Epsilon tolerance
pub fn generic_constant_rinex_test(dut: &Rinex, constant: f64, epsilon: f64) {
    if let Some(rec) = dut.record.as_obs() {
        for (k, v) in rec.iter() {
            for signal in v.signals.iter() {
                let err = (signal.value - constant).abs();
                assert!(
                    err < epsilon,
                    "({}{}) {} != {}",
                    k.epoch,
                    signal.sv,
                    signal.observable,
                    constant
                );
            }
        }
    }
}

/// Verifies that all contained data is Null
pub fn generic_null_rinex_test(dut: &Rinex) {
    generic_constant_rinex_test(dut, 0.0, 1.0E-9);
}

/// Verifies two [Header]s are "strictly" identical
pub fn generic_header_comparison(dut: &Header, model: &Header) {
    assert_eq!(dut.version, model.version);
    assert_eq!(dut.rinex_type, model.rinex_type);
    assert_eq!(dut.constellation, model.constellation);
    assert_eq!(dut.comments, model.comments);
    assert_eq!(dut.program, model.program);
    assert_eq!(dut.run_by, model.run_by);
    assert_eq!(dut.date, model.date);
    assert_eq!(dut.station_url, model.station_url);
    assert_eq!(dut.observer, model.observer);
    assert_eq!(dut.agency, model.agency);
    assert_eq!(dut.geodetic_marker, model.geodetic_marker);
    assert_eq!(dut.glo_channels, model.glo_channels);
    assert_eq!(dut.cospar, model.cospar);
    assert_eq!(dut.leap, model.leap);
    if let Some((dut_x, dut_y, dut_z)) = dut.rx_position {
        if let Some((model_x, model_y, model_z)) = model.rx_position {
            assert!(
                (dut_x - model_x).abs() < 1.0E-2,
                "error too large: {}/{}",
                dut_x,
                model_x
            );
            assert!(
                (dut_y - model_y).abs() < 1.0E-2,
                "error too large: {}/{}",
                dut_y,
                model_y
            );
            assert!(
                (dut_z - model_z).abs() < 1.0E-2,
                "error too large: {}/{}",
                dut_z,
                model_z
            );
        } else {
            panic!("got unexpected rx position!");
        }
    } else {
        if model.rx_position.is_some() {
            panic!("missing rx position!");
        }
    }
    assert_eq!(dut.wavelengths, model.wavelengths);
    assert_eq!(dut.sampling_interval, model.sampling_interval);
    assert_eq!(dut.license, model.license);
    assert_eq!(dut.doi, model.doi);
    assert_eq!(dut.rcvr, model.rcvr);

    rcvr_antenna_comparison(&dut.rcvr_antenna, &model.rcvr_antenna);

    assert_eq!(dut.sv_antenna, model.sv_antenna);
    assert_eq!(dut.ionod_corrections, model.ionod_corrections);
    assert_eq!(dut.dcb_compensations, model.dcb_compensations);
    assert_eq!(dut.pcv_compensations, model.pcv_compensations);
    assert_eq!(dut.obs, model.obs);
    assert_eq!(dut.meteo, model.meteo);
    assert_eq!(dut.clock, model.clock);
    assert_eq!(dut.antex, model.antex);
}

/// Compares strict equality between [A, B]
/// for all supported types, with panic on any single error;
/// and meaningful error report
pub fn generic_rinex_comparison(dut: &Rinex, model: &Rinex) {
    generic_header_comparison(&dut.header, &model.header);

    if dut.is_observation_rinex() && model.is_observation_rinex() {
        generic_observation_comparison(&dut, &model);
    } else if dut.is_meteo_rinex() && model.is_meteo_rinex() {
        #[cfg(feature = "meteo")]
        generic_meteo_comparison(&dut, &model);
    } else if dut.is_navigation_rinex() && model.is_navigation_rinex() {
        #[cfg(feature = "nav")]
        generic_navigation_comparison(&dut, &model);
    }
}

/*
 * CLOCK Rinex thorough comparison
 */
fn clocks_against_model(dut: &Rinex, model: &Rinex, filename: &str, _epsilon: f64) {
    let rec_dut = dut
        .record
        .as_clock()
        .expect("failed to unwrap rinex record");
    let rec_model = model
        .record
        .as_clock()
        .expect("failed to unwrap rinex record");
    for (e_model, model_types) in rec_model.iter() {
        if let Some(dut_types) = rec_dut.get(e_model) {
            for (model_data, _model_systems) in model_types.iter() {
                if let Some(_systems) = dut_types.get(model_data) {
                } else {
                    panic!(
                        "\"{}\" - {:?} - missing data {:?}",
                        filename, e_model, model_data
                    );
                }
            }
        } else {
            panic!("\"{}\" - missing epoch {:?}", filename, e_model);
        }
    }
}
