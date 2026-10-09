//! IGS NAV3 regressions. Excerpts are selected in memory from the original file.
use crate::{
    navigation::{NavFrameType, NavMessageType},
    prelude::*,
    record::ParsingDiagnosticKind,
};
use std::io::{BufReader, BufWriter};

// Original source lines, SHA-256:
// 5e51e8bfeee1218e5bf2fc0192b5647f805f549039650dbfb44595b9d4451855.
fn real() -> String {
    super::navigation_excerpt(
        "BRDC00IGS_R_20241310000_01D_MN.rnx",
        &[(1, 93), (318, 333), (8634, 8665)],
    )
}

fn read(text: &str) -> (Rinex, Vec<crate::record::ParsingDiagnostic>) {
    Rinex::parse_with_diagnostics(&mut BufReader::new(text.as_bytes())).unwrap()
}

#[test]
fn real_nav3_galileo_keeps_four_sources_and_gps_keys() {
    let (rinex, diagnostics) = read(&real());
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let records = rinex.record.as_nav().unwrap();
    assert_eq!(records.len(), 6);
    let mut galileo: Vec<_> = records
        .iter()
        .filter(|(k, _)| k.sv.constellation == Constellation::Galileo)
        .collect();
    galileo.sort_by_key(|(k, _)| k.galileo_data_sources);
    for ((key, frame), (source, msg, clock, transmit)) in galileo.iter().zip([
        (258, NavMessageType::FNAV, 1.261251745746E-04, 431500.0),
        (513, NavMessageType::INAV, 1.261241850443E-04, 431464.0),
        (516, NavMessageType::INAV, 1.261241850440E-04, 431464.0),
        (517, NavMessageType::INAV, 1.261241850443E-04, 431455.0),
    ]) {
        assert_eq!(key.galileo_data_sources, Some(source));
        assert_eq!(key.msgtype, msg);
        assert_eq!(key.frmtype, NavFrameType::Ephemeris);
        assert_eq!(key.epoch.to_string(), "2024-05-09T23:40:00 GST");
        let eph = frame.as_ephemeris().unwrap();
        assert_eq!(eph.clock_bias, clock);
        assert_eq!(eph.get_orbit_f64("source"), Some(source as f64));
        assert_eq!(eph.get_orbit_f64("t_tm"), Some(transmit));
    }
    assert_eq!(galileo.len(), 4);
    for (key, _) in records
        .iter()
        .filter(|(k, _)| k.sv.constellation == Constellation::GPS)
    {
        assert_eq!(key.galileo_data_sources, None);
        assert_eq!(key.msgtype, NavMessageType::LNAV);
    }
    #[cfg(feature = "serde")]
    {
        let gps_key = records
            .keys()
            .find(|k| k.sv.constellation == Constellation::GPS)
            .unwrap();
        let mut old = serde_json::to_value(gps_key).unwrap();
        assert!(old.get("galileo_data_sources").is_none());
        let back: crate::navigation::NavKey = serde_json::from_value(old.take()).unwrap();
        assert_eq!(&back, gps_key);
    }
}

#[test]
fn real_galileo_nav3_and_nav4_write_read_preserve_every_frame() {
    let (original, _) = read(&real());
    for version in [Version::new(3, 4), Version::new(4, 0)] {
        let mut rinex = original.clone();
        rinex.header.version = version;
        let mut writer = BufWriter::new(Vec::new());
        rinex.format(&mut writer).unwrap();
        let bytes = writer.into_inner().unwrap();
        let (back, diagnostics) = read(std::str::from_utf8(&bytes).unwrap());
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(back.record, original.record, "version {version}");
    }
}

#[test]
fn same_full_galileo_key_still_reports_real_content_conflicts() {
    // Synthetic repetition/change of an unchanged real source block.
    let real = real();
    let lines: Vec<_> = real.split_inclusive('\n').collect();
    let header = lines[..93].concat();
    let block = lines[109..117].concat();
    let changed = block.replacen("1.261251745746E-04", "1.361251745746E-04", 1);
    let text = format!("{header}{block}{block}{changed}");
    let (rinex, diagnostics) = read(&text);
    assert_eq!(rinex.record.as_nav().unwrap().len(), 1);
    assert_eq!(diagnostics.len(), 2);
    assert!(matches!(
        diagnostics[0].kind,
        ParsingDiagnosticKind::DuplicateNavigation {
            conflicting: false,
            previous_record_line: 1,
            ..
        }
    ));
    assert!(matches!(
        diagnostics[1].kind,
        ParsingDiagnosticKind::DuplicateNavigation {
            conflicting: true,
            previous_record_line: 9,
            ..
        }
    ));
    assert_eq!(diagnostics[0].record_line, 9);
    assert_eq!(diagnostics[1].record_line, 17);
    let (_, first) = rinex.record.as_nav().unwrap().first_key_value().unwrap();
    assert_eq!(first.as_ephemeris().unwrap().clock_bias, 1.361251745746E-04);
    assert_eq!(
        Rinex::parse(&mut BufReader::new(text.as_bytes()))
            .unwrap()
            .record,
        rinex.record
    );
}

#[test]
fn missing_or_mixed_galileo_sources_do_not_guess_a_message() {
    let real = real();
    let lines: Vec<_> = real.split_inclusive('\n').collect();
    let header = lines[..93].concat();
    let block = lines[109..117].concat();
    for source in [
        "                   ",
        " 0.000000000000E+00",
        " 2.590000000000E+02",
    ] {
        let changed = block.replace(" 2.580000000000E+02", source);
        let (rinex, diagnostics) = read(&format!("{header}{changed}"));
        assert!(rinex.record.as_nav().unwrap().is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(
            diagnostics[0].kind,
            ParsingDiagnosticKind::NavigationFailure(ParsingError::NavGalileoDataSources)
        ));
    }
}
#[test]
fn galileo_clock_reference_bits_are_mutually_exclusive() {
    let excerpt = super::navigation_excerpt(
        "BRDC00IGS_R_20241310000_01D_MN.rnx",
        &[(1, 93), (8634, 8641)],
    );
    // Synthetic boundary: 770 = F/NAV bit 1 plus BOTH clock bits 8 and 9.
    let changed = excerpt.replace(" 2.580000000000E+02", " 7.700000000000E+02");
    assert_ne!(changed, excerpt);
    let (rinex, diagnostics) =
        Rinex::parse_with_diagnostics(&mut BufReader::new(changed.as_bytes())).unwrap();
    assert!(rinex.record.as_nav().unwrap().is_empty());
    assert!(matches!(
        diagnostics.as_slice(),
        [crate::record::ParsingDiagnostic {
            kind: ParsingDiagnosticKind::NavigationFailure(ParsingError::NavGalileoDataSources),
            ..
        }]
    ));
}

#[test]
fn galileo_sources_do_not_round_or_saturate_and_reserved_bits_survive() {
    let source = super::navigation_excerpt(
        "BRDC00IGS_R_20241310000_01D_MN.rnx",
        &[(1, 93), (8634, 8641)],
    );
    for value in [258.5, -258.0, 4294967296.0, 770.0, 259.0, 263.0, 0.0] {
        let field = format!("{value:19.12E}");
        let text = source.replace(" 2.580000000000E+02", &field);
        let (r, d) = read(&text);
        assert!(r.record.as_nav().unwrap().is_empty(), "{value}");
        assert_eq!(d.len(), 1);
        assert!(matches!(
            d[0].kind,
            ParsingDiagnosticKind::NavigationFailure(_)
        ));
    }
    let text = source.replace(" 2.580000000000E+02", " 2.820000000000E+02"); // 258 + reserved bits 3/4
    let (r, d) = read(&text);
    assert!(d.is_empty());
    let (k, f) = r.record.as_nav().unwrap().first_key_value().unwrap();
    assert_eq!(k.galileo_data_sources, Some(282));
    assert_eq!(
        f.as_ephemeris().unwrap().get_orbit_f64("source"),
        Some(282.0)
    );
    // A direct NAV4 mismatch uses the same unchanged F/NAV payload.
    let block = source.lines().skip(93).collect::<Vec<_>>().join("\n");
    let text = super::navigation_rinex("4.00", &format!("> EPH E02 INAV\n{block}\n"));
    let (_, d) = read(&text);
    assert!(matches!(
        d[0].kind,
        ParsingDiagnosticKind::NavigationFailure(ParsingError::NavGalileoDataSources)
    ));
}

#[test]
#[cfg(feature = "qc")]
fn navigation_merge_inserts_distinct_keys_and_keeps_left_on_conflict() {
    let (original, _) = read(&real());
    let mut lhs = original.clone();
    lhs.record
        .as_mut_nav()
        .unwrap()
        .retain(|k, _| k.galileo_data_sources == Some(258));
    let mut rhs = original;
    for f in rhs.record.as_mut_nav().unwrap().values_mut() {
        f.as_mut_ephemeris().unwrap().clock_bias += 0.001;
    }
    use crate::prelude::qc::Merge;
    lhs.merge_mut(&rhs).unwrap();
    let record = lhs.record.as_nav().unwrap();
    assert_eq!(record.len(), 6);
    let (k, f) = record
        .iter()
        .find(|(k, _)| k.galileo_data_sources == Some(258))
        .unwrap();
    assert_eq!(f.as_ephemeris().unwrap().clock_bias, 1.261251745746E-04);
    let _ = k;
}
