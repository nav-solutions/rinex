use std::str::FromStr;

use crate::{
    observation::{
        parse_epoch, ClockObservation, EpochFlag, HeaderFields, ObsKey, Observations,
        SignalObservation,
    },
    prelude::{
        Constellation, Epoch, GeodeticMarker, Header, Observable, Rinex, RinexType, Version, SV,
    },
    tests::toolkit::{
        generic_rinex_test, gnss_csv as gnss_from_csv, observables_csv as observable_from_csv,
        sv_csv as sv_from_csv, TimeFrame,
    },
};

use itertools::Itertools;

pub struct ClockDataPoint {
    pub key: ObsKey,
    pub clock: ClockObservation,
}

impl ClockDataPoint {
    pub fn new(epoch: Epoch, flag: EpochFlag, offset_s: f64) -> Self {
        Self {
            key: ObsKey { epoch, flag },
            clock: ClockObservation::default().with_offset_s(epoch, offset_s),
        }
    }
}

#[derive(Debug)]
pub struct SignalDataPoint {
    pub key: ObsKey,
    pub signal: SignalObservation,
}

impl SignalDataPoint {
    pub fn new(epoch: Epoch, flag: EpochFlag, sv: SV, observable: Observable, value: f64) -> Self {
        let signal = SignalObservation {
            sv,
            observable,
            value,
            lli: None,
            snr: None,
        };
        Self {
            key: ObsKey { epoch, flag },
            signal,
        }
    }
}

/// Basic tests for Observation [Rinex]
fn basic_header_tests(dut: &Header, timeof_first_obs: Option<&str>, timeof_last_obs: Option<&str>) {
    assert!(dut.obs.is_some(),);
    assert!(dut.meteo.is_none(),);
    assert!(dut.ionex.is_none(),);
    assert!(dut.clock.is_none(),);

    let specs = dut.obs.as_ref().expect("missing specific specs");

    if let Some(t) = timeof_first_obs {
        let t = Epoch::from_str(t).unwrap();
        assert_eq!(specs.timeof_first_obs, Some(t));
    }

    if let Some(t) = timeof_last_obs {
        let t = Epoch::from_str(t).unwrap();
        assert_eq!(specs.timeof_last_obs, Some(t));
    }
}

/// Generic test that we can use for Observation [Rinex]
pub fn generic_observation_rinex_test(
    dut: &Rinex,
    version: &str,
    header_constellation: Option<&str>,
    has_clock: bool,
    sv_csv: &str,
    gnss_csv: &str,
    gnss_observ_csv: &[(&str, &str)],
    timeof_first_obs: Option<&str>,
    timeof_last_obs: Option<&str>,
    ground_ref_wgs84_m: Option<(f64, f64, f64)>,
    observer: Option<&str>,
    geodetic_marker: Option<GeodeticMarker>,
    time_frame: TimeFrame,
    signal_points: Vec<SignalDataPoint>,
    clock_points: Vec<ClockDataPoint>,
) {
    assert!(dut.is_observation_rinex());

    let dut_rec = dut.record.as_obs().unwrap();

    generic_rinex_test(
        dut,
        version,
        header_constellation,
        RinexType::ObservationData,
        Some(time_frame),
    );

    basic_header_tests(&dut.header, timeof_first_obs, timeof_last_obs);

    if let Some((x_ecef_m, y_ecef_m, z_ecef_m)) = ground_ref_wgs84_m {
        assert_eq!(dut.header.rx_position, Some((x_ecef_m, y_ecef_m, z_ecef_m)));
    }

    if let Some(observer) = observer {
        let header = dut.header.observer.as_ref().unwrap();
        assert_eq!(header, observer);
    }

    if let Some(marker) = geodetic_marker {
        assert_eq!(dut.header.geodetic_marker, Some(marker));
    }

    // verifies header specs
    let specs = dut.header.obs.as_ref().unwrap();

    for (gnss, observable_csv) in gnss_observ_csv {
        let gnss = Constellation::from_str(gnss).unwrap();

        let mut expected = observable_from_csv(observable_csv);
        expected.sort();

        let found = specs
            .codes
            .get(&gnss)
            .expect(&format!("missing header specs for {}", gnss))
            .into_iter()
            .cloned()
            .sorted()
            .collect::<Vec<_>>();

        assert_eq!(found, expected, "test failed for constellation {}", gnss);
    }

    let clocks = dut.clock_observations_iter().collect::<Vec<_>>();
    if has_clock {
        assert!(clocks.len() > 0, "missing clock data");
    } else {
        assert!(clocks.len() == 0, "found invalid clock data");
    }

    // Check SV content
    let content = dut.sv_iter().collect::<Vec<_>>();
    let expected = sv_from_csv(sv_csv);
    assert_eq!(content, expected);

    // Check GNSS content
    let content = dut.constellations_iter().sorted().collect::<Vec<_>>();
    let expected = gnss_from_csv(gnss_csv);
    assert_eq!(content, expected);

    // Test clock data points
    for point in clock_points {
        let k = point.key;
        let values = dut_rec
            .get(&k)
            .expect(&format!("missing clock data for {:?}", k));
        assert_eq!(values.clock, Some(point.clock));
    }

    // Test signal data points
    for point in signal_points {
        let k = point.key;
        let values = dut_rec
            .get(&k)
            .expect(&format!("missing data point for {:?}", k));

        let mut passed = false;

        for signal in values.signals.iter() {
            if signal.sv == point.signal.sv {
                if signal.observable == point.signal.observable {
                    assert_eq!(signal.value, point.signal.value);
                    assert_eq!(signal.lli, point.signal.lli);
                    assert_eq!(signal.snr, point.signal.snr);
                    passed = true;
                }
            }
        }
        assert!(passed, "missing data point {:?}", point);
    }
}

/// [Rinex] against [Rinex] model verification
pub fn generic_comparison(dut: &Rinex, model: &Rinex) {
    // verify SV
    let dut_content = dut.sv_iter().sorted().collect::<Vec<_>>();
    let expected_content = model.sv_iter().sorted().collect::<Vec<_>>();
    assert_eq!(dut_content, expected_content);

    // verify constellations
    let dut_content = dut.constellations_iter().sorted().collect::<Vec<_>>();
    let expected_content = model.constellations_iter().sorted().collect::<Vec<_>>();
    assert_eq!(dut_content, expected_content);

    // verify observables
    let dut_content = dut.observables_iter().sorted().collect::<Vec<_>>();
    let expected_content = dut.observables_iter().sorted().collect::<Vec<_>>();
    assert_eq!(dut_content, expected_content);

    // TODO : verify carriers

    // Point by point strict equality
    let dut = dut.record.as_obs().unwrap();
    let model = model.record.as_obs().unwrap();

    for (k, model_v) in model.iter() {
        if let Some(dut_v) = dut.get(&k) {
            assert_eq!(model_v.clock, dut_v.clock, "invalid clock observation");

            for model_sig in model_v.signals.iter() {
                if let Some(dut_sig) = dut_v
                    .signals
                    .iter()
                    .filter(|sig| sig.sv == model_sig.sv && sig.observable == model_sig.observable)
                    .reduce(|k, _| k)
                {
                    // RINEX garantees 3 digit accuracy
                    let err = (dut_sig.value - model_sig.value).abs();
                    assert!(
                        err < 1E-2,
                        "signal error too large @ {}:{} {}",
                        model_sig.sv,
                        model_sig.observable,
                        k.epoch
                    );

                    // LLI should reach strict equality
                    assert_eq!(
                        dut_sig.lli, model_sig.lli,
                        "invalid signal LLI @ {}:{} {}",
                        model_sig.sv, model_sig.observable, k.epoch
                    );

                    // SNR should reach strict equality
                    assert_eq!(
                        dut_sig.snr, model_sig.snr,
                        "invalid signal SNR @ {}:{} {}",
                        model_sig.sv, model_sig.observable, k.epoch
                    );
                } else {
                    panic!(
                        "missing observation for {}:{} @{}",
                        model_sig.sv, model_sig.observable, k.epoch
                    );
                }
            }
        } else {
            panic!("missing record entry at {:#?}", k);
        }
    }

    for (k, dut_v) in dut.iter() {
        if let Some(model_v) = model.get(&k) {
            for signal in dut_v.signals.iter() {
                if model_v
                    .signals
                    .iter()
                    .filter(|sig| sig.sv == signal.sv && sig.observable == signal.observable)
                    .reduce(|k, _| k)
                    .is_none()
                {
                    panic!(
                        "found unexpected signal observation at {}:{} at {}",
                        signal.sv, signal.observable, k.epoch
                    );
                }
            }
        } else {
            panic!("found unexpected content at {:#?}", k);
        }
    }
}

/// Generic method to use at the epoch decoding level (testing)
pub fn generic_observation_epoch_decoding_test(
    content: &str,
    major: u8,
    header_constell: Constellation,
    header_gnss_obs_csv: &[(&str, &str)],
    timeof_first_obs: &str,
    num_signals: usize,
    key_epoch: &str,
    key_flag: EpochFlag,
    clock: Option<ClockObservation>,
    signals: Vec<SignalObservation>,
) {
    // build [Header]
    let t0 = Epoch::from_str(timeof_first_obs).unwrap();
    let key_epoch = Epoch::from_str(key_epoch).unwrap();

    let ts = t0.time_scale;

    let mut specs = HeaderFields::default().with_timeof_first_obs(t0);

    for (constell, observable_csv) in header_gnss_obs_csv.iter() {
        let constell = Constellation::from_str(constell).unwrap();
        let observables = observable_from_csv(observable_csv);
        specs.codes.insert(constell, observables);
    }

    let header = Header::default()
        .with_version(Version { major, minor: 0 })
        .with_constellation(header_constell)
        .with_observation_fields(specs);

    // PARSE
    let mut obs = Observations::default();

    let key = parse_epoch(&header, content, ts, &mut obs).unwrap();

    assert_eq!(key.epoch, key_epoch);
    assert_eq!(key.flag, key_flag);
    assert_eq!(obs.clock, clock);
    assert_eq!(obs.signals.len(), num_signals);

    for signal in signals.iter() {
        let mut found = false;
        for parsed in obs.signals.iter() {
            if signal.sv == parsed.sv {
                if signal.observable == parsed.observable {
                    assert_eq!(signal.value, parsed.value);
                    assert_eq!(signal.lli, parsed.lli);
                    assert_eq!(signal.snr, parsed.snr);
                    found = true;
                }
            }
        }
        if !found {
            panic!("signal not found={}({})", signal.sv, signal.observable);
        }
    }
}
