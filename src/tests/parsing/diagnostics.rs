//! Recoverable failures, duplicate records and fatal input errors.
use super::{
    epoch, gps_first_block, navigation_rinex, parse_diagnostics, source_observations,
    synthetic_obs, GLO_BLOCK, GPS_BLOCK, SBAS_BLOCK,
};
use crate::{
    observation::{EpochFlag, ObsKey},
    prelude::*,
};
use std::io::BufReader;

#[test]
fn source_records_have_no_recoverable_diagnostics() {
    for text in [
        source_observations(),
        navigation_rinex("4.02", GPS_BLOCK),
        navigation_rinex("4.00", GLO_BLOCK),
        navigation_rinex("4.00", SBAS_BLOCK),
    ] {
        let (_, diagnostics) = parse_diagnostics(&text);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }
}

#[test]
fn ordinary_parse_matches_diagnostic_parse_on_omissions_and_duplicates() {
    let block = epoch("0.0000000", None, None);
    let changed = block.replace("-12.500", "-13.500");
    for text in [
        navigation_rinex(
            "4.02",
            &format!("{GPS_BLOCK}{GPS_BLOCK}> EPH G03 UNKNOWN\n"),
        ),
        synthetic_obs(
            "3.04",
            &format!(
                "{block}{changed}{}",
                epoch("1.0000000", Some("INVALID"), None)
            ),
        ),
    ] {
        let plain = super::parse(&text).unwrap();
        let (reported, diagnostics) = parse_diagnostics(&text);
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(plain.header, reported.header);
        assert_eq!(plain.record, reported.record);
        assert_eq!(plain.comments, reported.comments);
    }
}

#[test]
fn ordinary_and_diagnostic_parse_share_fatal_error_rules() {
    for (text, expected) in [
        (
            synthetic_obs("4.02", &epoch("0.1234567", None, Some("00001"))),
            "epoch precision is finer than the native nanosecond resolution",
        ),
        ("é\n".to_owned(), "non-ASCII input at line 1, byte column 1"),
    ] {
        let ordinary = super::parse(&text).unwrap_err();
        let reported =
            Rinex::parse_with_diagnostics(&mut BufReader::new(text.as_bytes())).unwrap_err();
        assert_eq!(ordinary.to_string(), expected);
        assert_eq!(reported.to_string(), expected);
    }
}

#[test]
fn synthetic_nav_duplicates_conflicts_and_failure_are_reported() {
    use crate::record::ParsingDiagnosticKind::*;
    let (header, block) = gps_first_block();
    let changed = block.replacen("3.551370464265E-04", "3.651370464265E-04", 1);
    let source = format!("{header}{block}{block}{changed}> EPH G03 UNKNOWN\n");
    let (rnx, diagnostics) = parse_diagnostics(&source);
    assert_eq!(rnx.record.as_nav().unwrap().len(), 1);
    assert_eq!(diagnostics.len(), 3);
    assert_eq!(diagnostics[0].record_line, 10);
    assert_eq!(diagnostics[1].record_line, 19);
    assert_eq!(diagnostics[2].record_line, 28);
    assert!(matches!(
        diagnostics[0].kind,
        DuplicateNavigation {
            conflicting: false,
            ..
        }
    ));
    assert!(matches!(
        diagnostics[1].kind,
        DuplicateNavigation {
            conflicting: true,
            ..
        }
    ));
    assert!(matches!(
        diagnostics[2].kind,
        NavigationFailure(ParsingError::NavMsgType)
    ));
    let eph = rnx
        .record
        .as_nav()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .as_ephemeris()
        .unwrap();
    assert_eq!(eph.clock_bias, 3.651370464265E-04); // last successful record
    for (d, previous) in diagnostics[..2].iter().zip([1, 10]) {
        assert!(matches!(
            d.kind,
            DuplicateNavigation { previous_record_line, .. } if previous_record_line == previous
        ));
    }
    assert_eq!(super::parse(&source).unwrap().record, rnx.record);
}

#[test]
fn synthetic_nav_illegal_field_is_not_misreported_as_missing() {
    use crate::record::ParsingDiagnosticKind::NavigationFailure;
    let (header, block) = gps_first_block();
    for token in [
        "                BAD",
        "                NaN",
        "                inf",
    ] {
        let modified = block.replacen(" 1.100000000000E+02", token, 1);
        let (rnx, diagnostics) = parse_diagnostics(&format!("{header}{modified}"));
        assert!(rnx.record.as_nav().unwrap().is_empty());
        assert_eq!(diagnostics.len(), 1);
        match &diagnostics[0].kind {
            NavigationFailure(ParsingError::NavOrbitParsing { field, .. }) => {
                assert_eq!(field, "iode")
            },
            other => panic!("unexpected diagnostic {other:?}"),
        }
    }
}

#[test]
fn synthetic_nav_flag_mapping_failure_omits_the_frame_and_keeps_the_cause() {
    use crate::record::ParsingDiagnosticKind::NavigationFailure;
    let mut lines: Vec<_> = GLO_BLOCK.lines().map(str::to_owned).collect();
    // Synthetic derivative: bit 3 is outside the supported FDMA health mask.
    lines[2].replace_range(61..80, &format!("{:>19}", "8.0"));
    let text = navigation_rinex("4.00", &(lines.join("\n") + "\n"));
    let (rnx, diagnostics) = parse_diagnostics(&text);
    assert!(rnx.record.as_nav().unwrap().is_empty());
    assert_eq!(diagnostics.len(), 1);
    match &diagnostics[0].kind {
        NavigationFailure(ParsingError::NavOrbitParsing { field, source }) => {
            assert_eq!(field, "health");
            assert!(matches!(source.as_ref(), ParsingError::NavFlagsMapping));
        },
        other => panic!("unexpected diagnostic {other:?}"),
    }
}

#[test]
fn synthetic_obs_duplicates_and_clock_errors_are_reported() {
    use crate::record::ParsingDiagnosticKind::*;
    let block = epoch("0.0000000", None, None);
    let changed = block.replace("-12.500", "-13.500");
    let invalid = epoch("1.0000000", Some("INVALID"), None);
    let (rnx, diagnostics) = parse_diagnostics(&synthetic_obs(
        "3.04",
        &format!("{block}{block}{changed}{invalid}"),
    ));
    assert_eq!(rnx.observations_iter().count(), 1);
    assert_eq!(diagnostics.len(), 3);
    assert!(matches!(
        diagnostics[0].kind,
        DuplicateObservation {
            conflicting: false,
            ..
        }
    ));
    assert!(matches!(
        diagnostics[1].kind,
        DuplicateObservation {
            conflicting: true,
            ..
        }
    ));
    assert!(matches!(
        diagnostics[2].kind,
        ObservationFailure(ParsingError::ObsClockParsing)
    ));
}

#[test]
fn synthetic_input_io_failure_is_not_successful_eof() {
    struct FailingReader(std::io::Cursor<Vec<u8>>);
    impl std::io::Read for FailingReader {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            let count = std::io::Read::read(&mut self.0, out)?;
            if count == 0 {
                Err(std::io::Error::other("synthetic input failure"))
            } else {
                Ok(count)
            }
        }
    }
    let text = synthetic_obs("4.02", &epoch("0.0000000", None, None));
    let mut reader = BufReader::new(FailingReader(std::io::Cursor::new(text.into_bytes())));
    assert!(matches!(
        Rinex::parse(&mut reader),
        Err(ParsingError::InputIo(_))
    ));
}

#[test]
fn synthetic_nav_structural_failures_are_diagnostics() {
    use crate::record::ParsingDiagnosticKind::NavigationFailure;
    let (header, block) = gps_first_block();
    for (body, expected) in [
        (">\n".to_owned(), "invalid epoch format"),
        ("> EPH G03 LNAV\nG03\n".to_owned(), "invalid epoch format"),
        (
            block.replacen("G03 2024", "G04 2024", 1),
            "nav: ephemeris satellite differs from the record header",
        ),
        (
            block.replacen("3.551370464265E-04", "               NaN", 1),
            "nav: clock parsing",
        ),
    ] {
        let (rnx, diagnostics) = parse_diagnostics(&format!("{header}{body}"));
        assert!(rnx.record.as_nav().unwrap().is_empty());
        assert_eq!(diagnostics.len(), 1);
        match &diagnostics[0].kind {
            NavigationFailure(error) => assert_eq!(error.to_string(), expected),
            other => panic!("unexpected diagnostic {other:?}"),
        }
    }
}

fn assert_non_ascii(error: &ParsingError, line: usize, column: usize) {
    match error {
        ParsingError::NonAscii {
            line: actual,
            byte_column,
        } => {
            assert_eq!((*actual, *byte_column), (line, column));
        },
        other => panic!("expected NonAscii, got {other:?}"),
    }
}

#[test]
fn synthetic_obs_non_ascii_omits_the_whole_epoch_and_recovers() {
    use crate::record::ParsingDiagnosticKind::ObservationFailure;
    let block = epoch("1.0000000", None, None);
    let bad_row = block
        .lines()
        .nth(1)
        .unwrap()
        .replace("G03", "G04")
        .replace("17", "é7");
    let bad = format!("{}{bad_row}\n", block.replacen("  0  1", "  0  2", 1));
    let body = format!(
        "{}{bad}{}",
        epoch("0.0000000", None, None),
        epoch("2.0000000", None, None)
    );
    let (rnx, diagnostics) = parse_diagnostics(&synthetic_obs("3.04", &body));
    let observations = rnx.record.as_obs().unwrap();
    assert_eq!(observations.len(), 2);
    assert!(observations.values().all(|obs| obs.signals.len() == 2));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].record_line, 3);
    match &diagnostics[0].kind {
        ObservationFailure(error) => assert_non_ascii(error, 3, 18),
        other => panic!("unexpected diagnostic {other:?}"),
    }
}

#[test]
fn synthetic_obs_non_ascii_epoch_description_is_reported() {
    use crate::record::ParsingDiagnosticKind::ObservationFailure;
    let bad = epoch("0.0000000", None, None).replace("2024", "2é24");
    let (rnx, diagnostics) = parse_diagnostics(&synthetic_obs("4.02", &bad));
    assert!(rnx.record.as_obs().unwrap().is_empty());
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].record_line, 1);
    match &diagnostics[0].kind {
        ObservationFailure(error) => assert_non_ascii(error, 1, 4),
        other => panic!("unexpected diagnostic {other:?}"),
    }
}

#[test]
fn synthetic_nav_non_ascii_positions_and_recovery() {
    use crate::record::ParsingDiagnosticKind::NavigationFailure;
    for (from, to, line, column) in [
        ("LNAV", "LNAé", 1, 14),
        ("G03 2024", "G03é2024", 2, 4),
        ("1.100000", "é.100000", 3, 6),
    ] {
        let bad = GPS_BLOCK.replacen(from, to, 1);
        let (rnx, diagnostics) =
            parse_diagnostics(&navigation_rinex("4.02", &format!("{bad}{GPS_BLOCK}")));
        assert_eq!(rnx.record.as_nav().unwrap().len(), 1);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].record_line, 1);
        match &diagnostics[0].kind {
            NavigationFailure(error) => assert_non_ascii(error, line, column),
            other => panic!("unexpected diagnostic {other:?}"),
        }
    }
}

#[test]
fn synthetic_legacy_nav_non_ascii_boundary_keeps_neighbouring_blocks() {
    use crate::record::ParsingDiagnosticKind::NavigationFailure;
    let block = GPS_BLOCK.split_once('\n').unwrap().1;
    let bad = block.replacen("G03 ", "G03é", 1);
    let next = block.replacen("00 00 00", "00 00 01", 1);
    let (rnx, diagnostics) =
        parse_diagnostics(&navigation_rinex("3.04", &format!("{block}{bad}{next}")));
    assert_eq!(rnx.record.as_nav().unwrap().len(), 2);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].record_line, 9);
    match &diagnostics[0].kind {
        NavigationFailure(error) => assert_non_ascii(error, 1, 4),
        other => panic!("unexpected diagnostic {other:?}"),
    }
}

#[test]
fn synthetic_legacy_boundary_detection_does_not_slice_utf8() {
    for prefix in [25, 28] {
        let line = format!("{}é{}", " ".repeat(prefix), " ".repeat(30));
        assert!(!crate::observation::is_new_epoch(&line, Version::new(2, 0)));
    }
    for (line, version) in [("1é 24 05 10", 2), ("G0é 2024 05 10", 3)] {
        assert!(!crate::navigation::is_new_epoch(
            line,
            Version::new(version, 0)
        ));
    }
}

#[test]
fn synthetic_non_ascii_header_and_body_comments_are_fatal() {
    // The 60th byte cuts through é, which used to panic at split_at(60).
    let comment = format!("{}éCOMMENT\n", " ".repeat(59));
    let end = format!("{:<60}END OF HEADER\n", "");
    let header = synthetic_obs("4.02", "").replace(&end, &format!("{comment}{end}"));
    for (text, line) in [(header, 4), (synthetic_obs("4.02", &comment), 1)] {
        let error =
            Rinex::parse_with_diagnostics(&mut BufReader::new(text.as_bytes())).unwrap_err();
        assert_non_ascii(&error, line, 60);
    }
}

#[test]
fn synthetic_invalid_utf8_header_and_body_are_input_errors() {
    let mut header = synthetic_obs("4.02", "").into_bytes();
    header[0] = 0xff;
    let mut body = synthetic_obs("4.02", "").into_bytes();
    body.extend_from_slice(b"\xff\n");
    for bytes in [header, body] {
        match Rinex::parse_with_diagnostics(&mut BufReader::new(bytes.as_slice())) {
            Err(ParsingError::InputIo(error)) => {
                assert_eq!(error.kind(), std::io::ErrorKind::InvalidData)
            },
            other => panic!("expected invalid UTF-8 input error, got {other:?}"),
        }
    }
}

#[test]
fn synthetic_non_ascii_crinex_is_rejected_before_decompression() {
    let mut header = super::parse(&synthetic_obs("3.04", ""))
        .unwrap()
        .header
        .with_crinex(crate::hatanaka::CRINEX::default());
    let error = crate::record::Record::parse_with_diagnostics(
        &mut header,
        &mut BufReader::new("é\n".as_bytes()),
    )
    .unwrap_err();
    assert_non_ascii(&error, 1, 1);
}

fn synthetic_crinex_event(special_record: &str) -> String {
    // Synthetic V3 stream: one uncompressed event record between two observations.
    let body = format!(
        "> 2024 05 10 00 00 00.0000000  0  1      G03\n\n3&20000000123\n\
         > 2024 05 10 00 00 15.0000000  4  1\n{special_record}\n\
         > 2024 05 10 00 00 30.0000000  0  1      G03\n\n3&20000100456\n"
    );
    format!(
        "{:<60}CRINEX VERS   / TYPE\n{:<60}CRINEX PROG / DATE\n{}",
        "3.0                 COMPACT RINEX FORMAT",
        "RNX2CRX ver.4.1.0                       31-Jan-25 09:24",
        super::observation_rinex("3.04", "G    1 C1C", &body)
    )
}

#[test]
fn synthetic_crinex_event_comments_reach_decompressor_in_both_parse_modes() {
    for special_record in [
        format!("{:<60}COMMENT", "EVENT RECORD"),
        format!("{:<60}MARKER NAME", "COMMENT IN EVENT RECORD"),
    ] {
        let text = synthetic_crinex_event(&special_record);
        let plain = super::parse(&text).unwrap();
        let (reported, diagnostics) = parse_diagnostics(&text);
        assert_eq!(plain.header, reported.header);
        assert_eq!(plain.record, reported.record);
        assert_eq!(plain.comments, reported.comments);
        assert!(reported.comments.is_empty());
        let observations = reported.record.as_obs().unwrap();
        assert_eq!(observations.len(), 2);
        for (second, value) in [(0, 20_000_000.123), (30, 20_000_100.456)] {
            let key = ObsKey {
                epoch: Epoch::from_gregorian(2024, 5, 10, 0, 0, second, 0, TimeScale::GPST),
                flag: EpochFlag::Ok,
            };
            let signals = &observations.get(&key).unwrap().signals;
            assert_eq!(signals.len(), 1);
            assert_eq!(signals[0].value, value);
        }
        // The existing parser reports unsupported hardware events separately.
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].record_line, 4);
        assert!(matches!(
            diagnostics[0].kind,
            crate::record::ParsingDiagnosticKind::ObservationFailure(
                ParsingError::ObsHardwareEvent
            )
        ));
    }
}

#[test]
fn synthetic_non_ascii_crinex_event_record_is_fatal_in_both_parse_modes() {
    // Byte column 60 bisects the UTF-8 character before the COMMENT label.
    let text = synthetic_crinex_event(&format!("{}éCOMMENT", " ".repeat(59)));
    let plain = super::parse(&text).unwrap_err();
    let reported = Rinex::parse_with_diagnostics(&mut BufReader::new(text.as_bytes())).unwrap_err();
    assert_non_ascii(&plain, 5, 60);
    assert_non_ascii(&reported, 5, 60);
}

#[test]
fn navigation_last_successful_record_and_previous_line_follow_each_replacement() {
    use crate::record::ParsingDiagnosticKind::DuplicateNavigation;
    let (header, a) = gps_first_block();
    let b = a.replacen("3.551370464265E-04", "3.651370464265E-04", 1);
    for (last, flags, expected) in [
        (a, [true, true], 3.551370464265E-04),
        (b.as_str(), [true, false], 3.651370464265E-04),
    ] {
        for newline in [true, false] {
            let text = format!("{header}{a}{b}{last}");
            let text = if newline {
                text.as_str()
            } else {
                text.trim_end_matches('\n')
            };
            let (rinex, diagnostics) = parse_diagnostics(text);
            assert_eq!(diagnostics.len(), 2);
            for (d, (previous, current, flag)) in diagnostics
                .iter()
                .zip([(1, 10, flags[0]), (10, 19, flags[1])])
            {
                assert_eq!(d.record_line, current);
                assert!(
                    matches!(d.kind,DuplicateNavigation {previous_record_line,conflicting,..}
                    if previous_record_line==previous && conflicting==flag)
                );
            }
            let frame = rinex.record.as_nav().unwrap().values().next().unwrap();
            assert_eq!(frame.as_ephemeris().unwrap().clock_bias, expected);
            assert_eq!(super::parse(text).unwrap().record, rinex.record);
        }
    }
}
