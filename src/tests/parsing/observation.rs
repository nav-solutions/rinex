//! Observation values, indicators, receiver clocks and time precision.
use super::{epoch, parse, source_observations, synthetic_obs, OBS_BODY};
use crate::prelude::*;
use std::str::FromStr;

#[test]
fn real_obs_values_lli_and_terminal_ssi() {
    let rnx = parse(&source_observations()).unwrap();
    let epochs: Vec<_> = rnx.observations_iter().collect();
    assert_eq!(epochs.len(), 2);
    for (index, sv, code, value, lli, ssi) in [
        (0, "G03", "C1C", 20_888_140.504, None, 8),
        (0, "G03", "L1C", 109_767_926.631, Some(0), 8),
        (0, "G03", "L5Q", 81_969_573.129, Some(0), 8),
        (1, "G04", "L1C", 115_873_014.861, Some(1), 7),
        (1, "G04", "L1L", 115_873_015.861, Some(1), 7),
    ] {
        let signal = epochs[index]
            .1
            .signals
            .iter()
            .find(|s| {
                s.sv == SV::from_str(sv).unwrap()
                    && s.observable == Observable::from_str(code).unwrap()
            })
            .unwrap();
        assert_eq!(signal.value, value, "{sv} {code}");
        assert_eq!(signal.lli.map(|v| v.bits()), lli);
        assert_eq!(format!("{:x}", signal.snr.unwrap()), ssi.to_string());
    }
}

#[test]
fn real_gps_selected_epochs_match_source_columns() {
    // A fixed-column source comparison in tests, never in the production reader.
    let rnx = parse(&source_observations()).unwrap();
    let epochs: Vec<_> = rnx.observations_iter().collect();
    let codes = &rnx.header.obs.as_ref().unwrap().codes[&Constellation::GPS];
    let mut epoch_index = None;
    let mut compared = 0;
    for line in OBS_BODY.lines() {
        if line.starts_with('>') {
            epoch_index = Some(epoch_index.map_or(0, |i| i + 1));
        } else if line.starts_with('G') {
            let sv = SV::from_str(&line[..3]).unwrap();
            for (index, raw) in line.as_bytes()[3..].chunks(16).enumerate() {
                let raw = std::str::from_utf8(raw).unwrap();
                let value = raw[..raw.len().min(14)].trim().parse::<f64>();
                let signal = epochs[epoch_index.unwrap()]
                    .1
                    .signals
                    .iter()
                    .find(|s| s.sv == sv && s.observable == codes[index]);
                match value {
                    Ok(value) => {
                        let signal = signal.unwrap();
                        assert_eq!(signal.value, value);
                        assert_eq!(
                            signal.lli.map(|v| v.bits()),
                            raw.get(14..15).and_then(|s| s.parse().ok())
                        );
                        assert_eq!(
                            signal.snr.map(|s| format!("{s:x}")),
                            raw.get(15..16)
                                .filter(|s| !s.trim().is_empty())
                                .map(str::to_owned)
                        );
                        compared += 1;
                    },
                    Err(_) => assert!(signal.is_none()),
                }
            }
        }
    }
    assert_eq!(compared, 15); // nine G03 fields and six G04 fields in the source rows
}

#[test]
fn synthetic_clock_absent_zero_nonzero_and_no_epoch_leak() {
    for version in ["3.04", "4.02"] {
        let body = [
            epoch("0.0000000", None, None),
            epoch("1.0000000", Some("0.000000000000"), None),
            epoch(
                "2.0000000",
                Some("-0.123456789876"),
                Some("00000").filter(|_| version == "4.02"),
            ),
            epoch("3.0000000", None, None),
        ]
        .concat();
        let rnx = parse(&synthetic_obs(version, &body)).unwrap();
        let observations: Vec<_> = rnx.observations_iter().map(|(_, o)| o).collect();
        assert_eq!(observations.len(), 4);
        assert!(observations[0].clock.is_none());
        assert_eq!(observations[1].clock.as_ref().unwrap().offset_s, 0.0);
        assert_eq!(
            observations[2].clock.as_ref().unwrap().offset_s,
            -0.123456789876
        );
        assert!(observations[3].clock.is_none());
    }
}

#[test]
fn synthetic_short_obs_lines_and_numeric_boundaries() {
    let mut body = epoch("0.0000000", None, None);
    body.push_str("\nG\nG03\n");
    let rnx = parse(&synthetic_obs("4.02", &body)).unwrap();
    let (_, observations) = rnx.observations_iter().next().unwrap();
    assert_eq!(observations.signals.len(), 2);
    assert_eq!(observations.signals[0].value, -12.5);
    assert_eq!(observations.signals[0].lli.unwrap().bits(), 1);
    assert_eq!(observations.signals[1].value, 0.0);
}

#[test]
fn synthetic_4_02_nanosecond_extension_is_exact() {
    let body = epoch("0.1234567", None, Some("01000"));
    let rnx = parse(&synthetic_obs("4.02", &body)).unwrap();
    let (key, _) = rnx.observations_iter().next().unwrap();
    assert_eq!(
        key.epoch,
        Epoch::from_str("2024-05-10T00:00:00.123456701 GPST").unwrap()
    );
}

#[test]
fn synthetic_4_02_subnanosecond_extension_is_rejected() {
    let body = epoch("0.1234567", None, Some("00001"));
    assert!(matches!(
        parse(&synthetic_obs("4.02", &body)),
        Err(ParsingError::EpochPrecision)
    ));
}

#[test]
fn synthetic_f11_7_small_fraction_is_not_rescaled() {
    let rnx = parse(&synthetic_obs("3.04", &epoch("0.0000001", None, None))).unwrap();
    let (key, _) = rnx.observations_iter().next().unwrap();
    assert_eq!(
        key.epoch,
        Epoch::from_str("2024-05-10T00:00:00.000000100 GPST").unwrap()
    );
}

#[test]
fn synthetic_lli_bits_never_pollute_the_value() {
    for bit in [0, 1, 2, 4, 7] {
        let body = epoch("0.0000000", None, None).replace("-12.50017", &format!("-12.500{bit}7"));
        let rnx = parse(&synthetic_obs("4.02", &body)).unwrap();
        let signal = &rnx.observations_iter().next().unwrap().1.signals[0];
        assert_eq!(signal.value, -12.5);
        assert_eq!(signal.lli.unwrap().bits(), bit);
        assert_eq!(format!("{:x}", signal.snr.unwrap()), "7");
    }
}
