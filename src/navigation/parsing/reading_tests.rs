use super::parse_epoch;
use crate::{
    navigation::NavDiagnosticKind,
    prelude::{Constellation, Header, ParsingError, Rinex, Version},
};
use std::io::{BufReader, Read};

#[cfg(feature = "flate2")]
use std::io::Write;

// First two RINEX 2.11 GLONASS records from data/NAV/V2/amel0010.21g.
// Mutations below are synthetic and affect only the named fields.
fn sample() -> (String, Vec<String>, Vec<String>) {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/NAV/V2/amel0010.21g"
    ));
    let mut lines = source.lines();
    let mut header = String::new();
    for line in lines.by_ref() {
        header.push_str(line);
        header.push('\n');
        if line.contains("END OF HEADER") {
            break;
        }
    }
    let first = lines.by_ref().take(4).map(str::to_owned).collect();
    let second = lines.take(4).map(str::to_owned).collect();
    (header, first, second)
}

fn parse_file(header: &str, records: &[Vec<String>]) -> Result<Rinex, ParsingError> {
    let mut content = header.to_owned();
    for record in records {
        for line in record {
            content.push_str(line);
            content.push('\n');
        }
    }
    Rinex::parse(&mut BufReader::new(content.as_bytes()))
}

fn record_header() -> Header {
    Header::basic_nav()
        .with_version(Version::new(2, 11))
        .with_constellation(Constellation::Glonass)
}

#[test]
fn blank_and_short_tail_keep_alignment() {
    let (header, mut first, second) = sample();
    // Orbit row 2, slot 3 (accelY) is blank, but slot 4 (channel) remains.
    first[2].replace_range(3 + 2 * 19..3 + 3 * 19, &" ".repeat(19));
    // Orbit row 3 omits its fourth slot (ageOp) at end of line.
    first[3].truncate(3 + 3 * 19);

    let parsed = parse_file(&header, &[first.clone(), second]).unwrap();
    let (_, first_eph) = parsed
        .nav_ephemeris_frames_iter()
        .find(|(key, _)| key.sv.prn == 1)
        .unwrap();
    assert!(first_eph.orbits.get("accelY").is_none());
    assert_eq!(first_eph.glonass_freq_channel(), Some(1));
    assert_eq!(first_eph.get_orbit_f64("satPosZ"), Some(2.193169775390E+04));
    assert!(first_eph.orbits.get("ageOp").is_none());

    // A whitespace-only continuation shorter than the three-column indent is
    // an omitted row, not a panic or a shift into the following record.
    first[3] = "  ".to_string();
    let (_, frame) = parse_epoch(&record_header(), &first.join("\n")).unwrap();
    assert!(frame
        .as_ephemeris()
        .unwrap()
        .orbits
        .get("satPosZ")
        .is_none());

    // A nonblank line this short is malformed and reaches the public caller.
    first[3] = " x".to_string();
    assert!(matches!(
        parse_file(&header, &[first]),
        Err(ParsingError::NavOrbitLineTooShort { line: 4 })
    ));
}

#[test]
fn malformed_known_field_is_observable() {
    let (header, mut first, _) = sample();
    first[1].replace_range(3 + 19..3 + 2 * 19, "       NOT_A_NUMBER");
    let parsed = parse_file(&header, &[first.clone()]).unwrap();
    assert_eq!(parsed.nav_ephemeris_frames_iter().count(), 0);
    assert_eq!(parsed.nav_parse_report().rejected_records(), 1);
    let diagnostic = &parsed.nav_parse_report().diagnostics()[0];
    assert_eq!(diagnostic.kind, NavDiagnosticKind::InvalidField);
    assert_eq!(diagnostic.sv.unwrap().prn, 1);
    assert_eq!(diagnostic.message.unwrap().to_string(), "LNAV");
    assert!(diagnostic.epoch.is_some());
    assert_eq!(diagnostic.record_line, header.lines().count() + 1);
    assert_eq!(diagnostic.field_line, Some(header.lines().count() + 2));
    assert_eq!(diagnostic.slot, Some(2));
    assert_eq!(diagnostic.field.as_deref(), Some("velX"));
    assert_eq!(diagnostic.raw.as_deref(), Some("       NOT_A_NUMBER"));

    let mut content = header.clone();
    content.push_str(&first.join("\n"));
    let err = Rinex::parse_strict(&mut BufReader::new(content.as_bytes())).unwrap_err();
    match err {
        ParsingError::NavOrbitField {
            line,
            slot,
            field,
            value,
            ..
        } => {
            assert_eq!((line, slot), (2, 2));
            assert_eq!(field, "velX");
            assert_eq!(value, "       NOT_A_NUMBER");
        },
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn invalid_known_numeric_forms_are_observable() {
    let (header, first, _) = sample();
    for (row, slot, text, field) in [
        (1, 0, "NaN", "satPosX"),
        (1, 0, "1.000000000000D+999", "satPosX"),
        (2, 3, "1.500000000000D+00", "channel"),
        (2, 3, "1.280000000000D+02", "channel"),
    ] {
        let mut record = first.clone();
        record[row].replace_range(3 + slot * 19..3 + (slot + 1) * 19, &format!("{text:>19}"));
        let parsed = parse_file(&header, &[record]).unwrap();
        assert_eq!(parsed.nav_parse_report().rejected_records(), 1);
        assert_eq!(
            parsed.nav_parse_report().diagnostics()[0].field.as_deref(),
            Some(field)
        );
    }
}

#[test]
fn bad_record_does_not_corrupt_next_record() {
    let (header, mut first, second) = sample();
    let valid_first = first.clone();
    first[1].replace_range(3 + 19..3 + 2 * 19, "       NOT_A_NUMBER");

    let parsed = parse_file(&header, &[valid_first, first, second.clone()]).unwrap();
    assert_eq!(parsed.nav_ephemeris_frames_iter().count(), 2);
    assert_eq!(parsed.nav_parse_report().rejected_records(), 1);
    assert_eq!(parsed.nav_parse_report().unsupported_records(), 0);

    let values: Vec<_> = parsed
        .nav_ephemeris_frames_iter()
        .map(|(key, eph)| {
            (
                key.sv,
                eph.get_orbit_f64("satPosX"),
                eph.orbits.get("accelY").is_some(),
            )
        })
        .collect();

    let mut bytes = std::io::BufWriter::new(Vec::new());
    parsed.format(&mut bytes).unwrap();
    let formatted = bytes.into_inner().unwrap();
    let reread = Rinex::parse(&mut BufReader::new(formatted.as_slice())).unwrap();
    assert_eq!(reread.nav_ephemeris_frames_iter().count(), 2);
    assert_eq!(reread.nav_parse_report().rejected_records(), 0);
    let reread_values: Vec<_> = reread
        .nav_ephemeris_frames_iter()
        .map(|(key, eph)| {
            (
                key.sv,
                eph.get_orbit_f64("satPosX"),
                eph.orbits.get("accelY").is_some(),
            )
        })
        .collect();
    assert_eq!(reread_values, values);

    // The next actual record starts independently and retains its own slots.
    let (key, frame) = parse_epoch(&record_header(), &second.join("\n")).unwrap();
    assert_eq!(key.sv.prn, 2);
    let eph = frame.as_ephemeris().unwrap();
    assert_eq!(eph.get_orbit_f64("satPosX"), Some(-8.955041992190E+03));
    assert_eq!(eph.glonass_freq_channel(), Some(-4));
}

#[test]
fn unknown_future_message_does_not_hide_next_known_record() {
    // Public RINEX 4.02 example file. Only its first message code is changed.
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/NAV/V4/rinex402_examples_MN.rnx"
    ));
    let unknown = source.replacen("> EPH G04 LNAV", "> EPH G04 FUTR", 1);
    let parsed = Rinex::parse(&mut BufReader::new(unknown.as_bytes())).unwrap();
    let messages: Vec<_> = parsed
        .nav_ephemeris_frames_iter()
        .filter(|(key, _)| key.sv.to_string() == "G04")
        .map(|(key, _)| key.msgtype.to_string())
        .collect();
    assert!(!messages.iter().any(|msg| msg == "LNAV"));
    assert!(messages.iter().any(|msg| msg == "CNAV"));
    assert_eq!(parsed.nav_parse_report().unsupported_records(), 1);
    assert_eq!(parsed.nav_parse_report().rejected_records(), 0);
    assert!(matches!(
        Rinex::parse_strict(&mut BufReader::new(unknown.as_bytes())),
        Err(ParsingError::NavMsgType)
    ));
}

struct ErrorAfter<'a> {
    content: &'a [u8],
}

impl Read for ErrorAfter<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.content.is_empty() {
            Err(std::io::Error::other("injected read error"))
        } else {
            let count = buf.len().min(self.content.len());
            buf[..count].copy_from_slice(&self.content[..count]);
            self.content = &self.content[count..];
            Ok(count)
        }
    }
}

#[test]
fn io_failures_are_not_eof() {
    let (header, first, _) = sample();
    let body = format!("{header}{}\n", first.join("\n"));
    assert!(matches!(
        Rinex::parse(&mut BufReader::new(ErrorAfter {
            content: body.as_bytes()
        })),
        Err(ParsingError::InputError(_))
    ));
    assert!(matches!(
        Rinex::parse(&mut BufReader::new(ErrorAfter {
            content: header[..10].as_bytes()
        })),
        Err(ParsingError::InputError(_))
    ));
}

#[test]
fn missing_header_end_is_an_error() {
    let (header, _, _) = sample();
    let truncated = header.replace("END OF HEADER", "NOT THE END ");
    assert!(matches!(
        Rinex::parse(&mut BufReader::new(truncated.as_bytes())),
        Err(ParsingError::MissingEndOfHeader)
    ));
}

#[test]
fn invalid_clock_field_rejects_whole_record() {
    let (header, mut first, second) = sample();
    first[0].replace_range(3 + 19..3 + 2 * 19, "       NOT_A_NUMBER");
    let parsed = parse_file(&header, &[first.clone(), second]).unwrap();
    assert_eq!(parsed.nav_ephemeris_frames_iter().count(), 1);
    assert_eq!(parsed.nav_parse_report().rejected_records(), 1);
    let diagnostic = &parsed.nav_parse_report().diagnostics()[0];
    assert_eq!(diagnostic.field.as_deref(), Some("clockBias"));
    assert_eq!(diagnostic.field_line, Some(header.lines().count() + 1));
    assert_eq!(diagnostic.slot, Some(1));
    assert_eq!(diagnostic.raw.as_deref(), Some("       NOT_A_NUMBER"));
    let mut content = header.clone();
    content.push_str(&first.join("\n"));
    assert!(matches!(
        Rinex::parse_strict(&mut BufReader::new(content.as_bytes())),
        Err(ParsingError::NavClockField { .. })
    ));
}

#[test]
fn incomplete_record_fails_instead_of_publishing_partial_ephemeris() {
    let (header, first, second) = sample();
    let truncated = first.into_iter().take(2).collect();
    assert!(matches!(
        parse_file(&header, &[truncated, second]),
        Err(ParsingError::NavOrbitMissingLines { .. })
    ));
}

#[test]
fn broken_v4_record_marker_is_fatal() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/NAV/V4/rinex402_examples_MN.rnx"
    ));
    let broken = source.replacen("> EPH G04 LNAV", ">", 1);
    assert!(matches!(
        Rinex::parse(&mut BufReader::new(broken.as_bytes())),
        Err(ParsingError::NavRecordBoundary)
    ));
}

#[cfg(feature = "flate2")]
#[test]
fn ordinary_file_and_gzip_entrypoints_expose_diagnostics() {
    use flate2::{write::GzEncoder, Compression};
    let (header, mut first, second) = sample();
    first[1].replace_range(3 + 19..3 + 2 * 19, "       NOT_A_NUMBER");
    let body = format!("{header}{}\n{}\n", first.join("\n"), second.join("\n"));
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let plain = std::env::temp_dir().join(format!("rinex-nav-{suffix}.rnx"));
    let compressed = plain.with_extension("rnx.gz");
    std::fs::write(&plain, body.as_bytes()).unwrap();
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(body.as_bytes()).unwrap();
    std::fs::write(&compressed, gzip.finish().unwrap()).unwrap();

    for parsed in [Rinex::from_file(&plain), Rinex::from_gzip_file(&compressed)] {
        let parsed = parsed.unwrap();
        assert_eq!(parsed.nav_ephemeris_frames_iter().count(), 1);
        assert_eq!(parsed.nav_parse_report().rejected_records(), 1);
    }
    assert!(matches!(
        Rinex::from_file_strict(&plain),
        Err(ParsingError::NavOrbitField { .. })
    ));
    assert!(matches!(
        Rinex::from_gzip_file_strict(&compressed),
        Err(ParsingError::NavOrbitField { .. })
    ));
    std::fs::remove_file(plain).unwrap();
    std::fs::remove_file(compressed).unwrap();
}
