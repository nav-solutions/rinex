//! Receiver clock offset decompression, verified against the CRX2RNX output.
use crate::{
    hatanaka::Decompressor,
    prelude::{Constellation, Observable},
};

use std::{
    collections::HashMap,
    fs::read_to_string,
    io::BufReader,
    str::{from_utf8, FromStr},
};

/// Decompresses the record of `crnx_name` line by line and compares
/// the output with the record of `rnx_name`, the CRX2RNX 4.1.0 output.
/// Both files are complete (header included): the record starts after
/// the END OF HEADER line.
///
/// CRX2RNX writes the clock offset without a leading zero (".000000001520"),
/// the decompressor writes "0.000000001520": both are valid F15.12 / F12.9
/// fields. The leading zero is stripped on both sides before comparing.
///
/// With `epoch_lines_only`, only the epoch description lines (the ones
/// carrying the clock offset) are compared.
fn run_clock_raw_test(
    v3: bool,
    epoch_lines_only: bool,
    crnx_name: &str,
    rnx_name: &str,
    constellation: &str,
    specs: &[(&str, &str)],
) {
    let constellation = Constellation::from_str(constellation).unwrap();

    let mut gnss_observables = HashMap::<Constellation, Vec<Observable>>::new();

    for (constell, observables) in specs.iter() {
        let constell = Constellation::from_str(constell.trim()).unwrap();
        let observables = observables
            .split(',')
            .map(|ob| Observable::from_str(ob.trim()).unwrap())
            .collect::<Vec<_>>();
        gnss_observables.insert(constell, observables);
    }

    let input = read_to_string(format!("data/CRNX/{}", crnx_name)).unwrap();
    let output = read_to_string(format!("data/OBS/{}", rnx_name)).unwrap();

    let record_lines = |content: &str| {
        content
            .lines()
            .skip_while(|line| !line.contains("END OF HEADER"))
            .skip(1)
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
    };

    // RINEX 2 epoch description: " YY MM DD hh mm ss.sssssss", the
    // decimal point of the seconds at index 18 (observations have theirs
    // at index 10 + 16 * n)
    let is_v1_epoch_line = |line: &str| line.as_bytes().get(18) == Some(&b'.');

    let input_lines = record_lines(&input);
    let output_lines = record_lines(&output)
        .into_iter()
        .filter(|line| !epoch_lines_only || is_v1_epoch_line(line))
        .collect::<Vec<_>>();
    assert!(!input_lines.is_empty(), "no record in {}", crnx_name);

    // CRX2RNX writes "-.000123456" where we write "-0.000123456"
    let normalized = |line: &str| line.trim_end().replace(" 0.", "  .").replace("-0.", " -.");

    let mut decomp = Decompressor::new(v3, constellation, gnss_observables);

    let mut buf = [0; 4096];
    let mut nth_output = 0;
    let mut clock_lines = 0;

    for (nth_input, input) in input_lines.iter().enumerate() {
        let size = decomp
            .decompress(input, input.len(), &mut buf, 4096)
            .unwrap_or_else(|e| panic!("decompression failed on line {}: {}", nth_input, e));

        let content = from_utf8(&buf[..size]).expect("CRX2RNX should always produce valid UTF-8");

        for line in content.lines() {
            if epoch_lines_only && !is_v1_epoch_line(line) {
                continue;
            }

            let model = output_lines
                .get(nth_output)
                .unwrap_or_else(|| panic!("more output than model (line {})", nth_output));

            assert_eq!(
                normalized(line),
                normalized(model),
                "failed on line {} of {}",
                nth_output,
                rnx_name
            );

            // epoch line carrying a clock offset
            let clock_offset = if v3 { 41 } else { 68 };
            if model.len() > clock_offset && (v3 == model.starts_with('>')) {
                if model[clock_offset..].contains('.') {
                    clock_lines += 1;
                }
            }

            nth_output += 1;
        }
    }

    assert_eq!(nth_output, output_lines.len(), "missing output lines");
    assert!(clock_lines > 0, "the model has no clock offsets");
}

/// Regression test for <https://github.com/nav-solutions/rinex/issues/426>.
/// One hour of the BKG ACRG00GHA_R_20240010000_01D_30S_MO file reduced to a
/// single SV, compressed with RNX2CRX 4.1.0. The receiver clock offset is
/// reset ("3&") at order 3, then compressed, omitted for a few epochs, then
/// reset five more times. The model is the CRX2RNX 4.1.0 output.
#[test]
fn v3_acrg00gha_clock_offsets_raw() {
    run_clock_raw_test(
        true,
        false, // every line compared
        "V3/ACRG00GHA_R_20240010000_01H_30S_MO.crx",
        "V3/ACRG00GHA_R_20240010000_01H_30S_MO.rnx",
        "MIXED",
        &[("IRNSS", "C5A, D5A, L5A, S5A")],
    );
}

/// RINEX 2.11 counterpart: a delf0010.21o extract with receiver clock
/// offsets (F12.9, columns 69-80), compressed with RNX2CRX 4.1.0.
/// Includes a two-character compressed value, a gap in the clock reports
/// and a clock jump. The model is the CRX2RNX 4.1.0 output.
/// Only the epoch lines are compared: the V1 observation lines are not
/// decompressed correctly yet (an extra blank line follows the vehicles
/// without LLI/SNR flags).
#[test]
fn v1_delf0010_clock_offsets_raw() {
    run_clock_raw_test(
        false,
        true, // epoch lines only
        "V1/delf0010_clock.21d",
        "V2/delf0010_clock.21o",
        "MIXED",
        &[("MIXED", "L1, L2, C1, P2, P1, S1, S2")],
    );
}

use crate::{
    observation::EpochFlag,
    prelude::{Epoch, Rinex},
    tests::toolkit::{
        generic_observation_comparison, generic_observation_rinex_test, ClockDataPoint, TimeFrame,
    },
};

/// Parses `crnx_name` and the CRX2RNX model `rnx_name`, compares the
/// records, then verifies a few hand-picked `points` (epoch, offset in seconds).
fn run_clock_offsets_test(
    crnx_name: &str,
    rnx_name: &str,
    expected_epochs: usize,
    points: &[(&str, f64)],
) {
    let dut = Rinex::from_file(&format!("data/CRNX/{}", crnx_name)).unwrap();
    let model = Rinex::from_file(&format!("data/OBS/{}", rnx_name)).unwrap();

    // strict point by point comparison of every signal and clock offset
    generic_observation_comparison(&dut, &model);

    let dut_epochs = dut.epoch_iter().count();
    assert_eq!(dut_epochs, expected_epochs, "wrong number of epochs");
    assert_eq!(dut_epochs, model.epoch_iter().count());

    let dut_clocks = dut.clock_observations_iter().collect::<Vec<_>>();
    let model_clocks = model.clock_observations_iter().collect::<Vec<_>>();

    assert!(!model_clocks.is_empty(), "model has no clock offsets");
    assert_eq!(
        dut_clocks.len(),
        model_clocks.len(),
        "wrong number of clock offsets"
    );

    for (epoch, offset_s) in points {
        let epoch = Epoch::from_str(epoch).unwrap();
        let (_, clock) = dut_clocks
            .iter()
            .find(|(k, _)| k.epoch == epoch)
            .unwrap_or_else(|| panic!("missing clock offset at {}", epoch));
        assert_eq!(clock.offset_s, *offset_s, "wrong clock offset at {}", epoch);
    }
}

/// Record level counterpart of `v3_acrg00gha_clock_offsets_raw`: the
/// file parser drives the decompressor with the line termination attached,
/// the path that dropped every kernel reset (gh-426).
#[test]
fn v3_acrg00gha_clock_offsets() {
    let points = [
        // kernel reset: 3&1520
        ("2024-01-01T00:00:00 GPST", 0.000000001520),
        // first compressed value (order 1)
        ("2024-01-01T00:00:30 GPST", 0.000000003404),
        // steady state (order 3)
        ("2024-01-01T00:05:00 GPST", 0.000000001788),
        // last value before the clock is omitted
        ("2024-01-01T00:15:00 GPST", 0.000000000204),
        // kernel reset after the gap: 3&-1459
        ("2024-01-01T00:20:00 GPST", -0.000000001459),
    ];

    run_clock_offsets_test(
        "V3/ACRG00GHA_R_20240010000_01H_30S_MO.crx",
        "V3/ACRG00GHA_R_20240010000_01H_30S_MO.rnx",
        120,
        &points,
    );

    let dut = Rinex::from_file("data/CRNX/V3/ACRG00GHA_R_20240010000_01H_30S_MO.crx").unwrap();

    generic_observation_rinex_test(
        &dut,
        "3.05",
        Some("MIXED"),
        true, // has_clock
        "I02",
        "IRNSS",
        &[("IRNSS", "C5A, D5A, L5A, S5A")],
        Some("2024-01-01T00:00:00 GPST"),
        None,
        None, // ground_ref_wgs84_m
        None, // observer
        None, // geodetic_marker
        TimeFrame::from_inclusive_csv("2024-01-01T00:00:00 GPST, 2024-01-01T00:59:30 GPST, 30 s"),
        vec![], // signals
        points
            .iter()
            .map(|(epoch, offset_s)| {
                ClockDataPoint::new(Epoch::from_str(epoch).unwrap(), EpochFlag::Ok, *offset_s)
            })
            .collect(),
    );
}

/// gh-426: a numerical state that no longer fits in an i64 must end the
/// parsing with what was recovered so far, not panic (debug) nor wrap
/// silently (release). The first kernel reset of the ACRG00GHA extract is
/// replaced by i64::MAX, so the very next compressed value overflows.
#[test]
fn v3_clock_overflow_does_not_panic() {
    let content = read_to_string("data/CRNX/V3/ACRG00GHA_R_20240010000_01H_30S_MO.crx").unwrap();

    assert_eq!(content.matches("3&1520\n").count(), 1);
    let corrupted = content.replace("3&1520\n", "3&9223372036854775807\n");

    let mut reader = BufReader::new(corrupted.as_bytes());

    // parsing stops at the corrupted epoch: at most the epoch carrying
    // the reset itself is recovered, nothing past it
    if let Ok(rinex) = Rinex::parse(&mut reader) {
        assert!(rinex.epoch_iter().count() <= 1);
        for (_, clock) in rinex.clock_observations_iter() {
            assert_eq!(clock.offset_s, i64::MAX as f64 / 1.0E12);
        }
    }
}
