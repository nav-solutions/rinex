use itertools::Itertools;

use crate::{
    prelude::{Header, Rinex, RinexType},
    tests::toolkit::{generic_rinex_test, observables_csv, TimeFrame},
};

/// [Rinex] against [Rinex] model verification
#[cfg(feature = "meteo")]
pub fn generic_comparison(dut: &Rinex, model: &Rinex) {
    // verify sensors
    let dut_sensors = dut.meteo_sensors_iter().collect::<Vec<_>>();
    let expected_sensors = model.meteo_sensors_iter().collect::<Vec<_>>();
    assert_eq!(dut_sensors, expected_sensors);

    // verify observables
    let dut_content = dut.observables_iter().sorted().collect::<Vec<_>>();
    let expected_content = dut.observables_iter().sorted().collect::<Vec<_>>();
    assert_eq!(dut_content, expected_content);

    let rec_dut = dut
        .record
        .as_meteo()
        .expect("failed to unwrap rinex record");

    let rec_model = model
        .record
        .as_meteo()
        .expect("failed to unwrap rinex record");

    for (k, v) in rec_model.iter() {
        if let Some(dut_v) = rec_dut.get(&k) {
            assert_eq!(v, dut_v);
        } else {
            panic!("missing entry at {}:{}", k.epoch, k.observable);
        }
    }

    for (k, _) in rec_dut.iter() {
        assert!(
            rec_model.get(k).is_some(),
            "found unexpected content: {:?}",
            k
        );
    }
}

/// Basic tests for Observation [Rinex]
fn basic_header_tests(dut: &Header) {
    assert!(dut.obs.is_none(),);
    assert!(dut.meteo.is_some(),);
    assert!(dut.clock.is_none(),);

    let _ = dut.meteo.as_ref().expect("missing specific specs");
}

/// Generic test that we can use for Observation [Rinex]
pub fn generic_meteo_rinex_test(dut: &Rinex, version: &str, obs_csv: &str, _time_frame: TimeFrame) {
    assert!(dut.is_meteo_rinex());

    let mut observables = observables_csv(obs_csv);
    observables.sort();

    let found = dut.observables_iter().sorted().cloned().collect::<Vec<_>>();
    assert_eq!(found, observables);

    generic_rinex_test(
        dut,
        version,
        None,
        RinexType::MeteoData,
        //Some(time_frame), //TODO
        None,
    );

    basic_header_tests(&dut.header);
}
