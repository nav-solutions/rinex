//! Navigation values, message times and formatting roundtrips.
use super::{navigation_rinex, parse, GLO_BLOCK, GPS_BLOCK, SBAS_BLOCK};
use crate::{
    navigation::{NavMessageType, OrbitItem},
    prelude::*,
};
use std::{io::BufWriter, str::FromStr};

#[test]
fn navic_sto_irga_and_irgl_preserve_system_pairs() {
    // Original IRGL record, source lines 370-372; SHA-256 of the plain file:
    // 28aa24352328f7537bf78ebb7a27bfab0e4a397d47ae5d0215fd242fe37c6770.
    let source =
        super::navigation_excerpt("BRD400DLR_S_20230710000_01D_MN.rnx", &[(1, 9), (370, 372)]);
    for (pair, rhs) in [("IRGL", TimeScale::UTC), ("IRGA", TimeScale::GST)] {
        // IRGA is a synthetic derivative: only the original pair is changed.
        let text = source.replace("IRGL", pair);
        let (rinex, diagnostics) = super::parse_diagnostics(&text);
        assert!(diagnostics.is_empty(), "{pair}: {diagnostics:?}");
        let records = rinex.record.as_nav().unwrap();
        assert_eq!(records.len(), 1, "{pair}");
        let (key, frame) = records.first_key_value().unwrap();
        let sto = frame.as_system_time().unwrap();
        assert_eq!(key.sv, SV::from_str("I03").unwrap());
        assert_eq!(
            key.epoch,
            Epoch::from_str("2023-03-12T00:04:48 GPST").unwrap()
        );
        assert_eq!(
            key.sto_identity.unwrap().time_system.as_slice(),
            pair.as_bytes()
        );
        assert_eq!(sto.time_system.as_deref(), Some(pair));
        assert_eq!((sto.lhs, sto.rhs), (TimeScale::GPST, rhs));
        assert_eq!(sto.transmission_time, Some(372.0));
        assert_eq!(
            sto.polynomial,
            (5.410402081907e-08, 1.509903313490e-14, 1.321371397717e-19)
        );
        let mut writer = BufWriter::new(Vec::new());
        rinex.format(&mut writer).unwrap();
        let output = writer.into_inner().unwrap();
        let (back, diagnostics) = super::parse_diagnostics(std::str::from_utf8(&output).unwrap());
        assert!(diagnostics.is_empty(), "{pair}: {diagnostics:?}");
        assert_eq!(back.record, rinex.record, "{pair}");
    }
}

#[test]
fn navic_time_system_corr_irga_and_irgl_preserve_system_pairs() {
    // Original NAV3 header: its IRGL correction is at source line 21.
    let source = super::navigation_excerpt("BRDM00DLR_S_20241310000_01D_MN.rnx", &[(1, 26)]);
    for (pair, rhs) in [("IRGL", TimeScale::UTC), ("IRGA", TimeScale::GST)] {
        // IRGA is a synthetic derivative of the unchanged IRGL coefficients.
        let text = source.replace("IRGL", pair);
        let rinex = parse(&text).unwrap();
        let nav = rinex.header.nav.as_ref().unwrap();
        let correction = nav
            .time_offsets
            .iter()
            .find(|offset| offset.time_system.as_deref() == Some(pair))
            .unwrap_or_else(|| panic!("missing {pair} TIME SYSTEM CORR"));
        assert_eq!((correction.lhs, correction.rhs), (TimeScale::GPST, rhs));
        assert_eq!(correction.t_ref, (2313, 517200 * 1_000_000_000));
        assert_eq!(
            correction.polynomial,
            (1.3242242858e-08, -6.172840017e-14, 0.0)
        );
        let mut writer = BufWriter::new(Vec::new());
        rinex.format(&mut writer).unwrap();
        let output = writer.into_inner().unwrap();
        let back = parse(std::str::from_utf8(&output).unwrap()).unwrap();
        assert_eq!(back.header.nav, rinex.header.nav, "{pair}");
    }
}

#[test]
fn real_galileo_nav4_orbit_columns_and_writeback() {
    // E01 I/NAV and F/NAV, unchanged source lines 61026-61034/61998-62006.
    // BRD400DLR 2023-071 plain-file SHA-256:
    // 28aa24352328f7537bf78ebb7a27bfab0e4a397d47ae5d0215fd242fe37c6770.
    // Expected fields follow RINEX 4.00/4.02 Table A13 and the raw columns.
    let source = super::navigation_excerpt(
        "BRD400DLR_S_20230710000_01D_MN.rnx",
        &[(1, 9), (61026, 61034), (61998, 62006)],
    );
    for minor in [0, 1, 2] {
        // Only the version envelope changes to cover the supported NAV4 revisions.
        let text = source.replacen("     4.00", &format!("     4.{minor:02}"), 1);
        let (rinex, diagnostics) = super::parse_diagnostics(&text);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(rinex.header.version, Version::new(4, minor));
        let records = rinex.record.as_nav().unwrap();
        assert_eq!(records.len(), 2);
        let mut writer = BufWriter::new(Vec::new());
        rinex.format(&mut writer).unwrap();
        let output = String::from_utf8(writer.into_inner().unwrap()).unwrap();
        let lines: Vec<_> = output.lines().collect();
        for (msg, sources, bgd_e5b, transmit) in [
            (NavMessageType::INAV, 516, -6.984919309616e-10, 664.0),
            (NavMessageType::FNAV, 258, 0.0, 700.0),
        ] {
            let (key, frame) = records.iter().find(|(k, _)| k.msgtype == msg).unwrap();
            assert_eq!(key.sv, SV::from_str("E01").unwrap());
            assert_eq!(
                key.epoch,
                Epoch::from_str("2023-03-12T00:00:00 GST").unwrap()
            );
            assert_eq!(key.galileo_data_sources, Some(sources));
            assert_eq!(key.transmission_time.unwrap().seconds(), transmit);
            let eph = frame.as_ephemeris().unwrap();
            assert_eq!(eph.get_orbit_f64("week"), Some(2253.0));
            assert_eq!(eph.get_orbit_f64("sisa"), Some(3.12));
            assert_eq!(eph.get_orbit_f64("health"), Some(0.0));
            assert_eq!(eph.get_orbit_f64("bgdE5aE1"), Some(-4.656612873077e-10));
            assert_eq!(eph.get_orbit_f64("bgdE5bE1"), Some(bgd_e5b));
            assert_eq!(eph.get_orbit_f64("t_tm"), Some(transmit));
            assert!(eph.orbits.keys().all(|name| !name.starts_with("spare")));

            // Check written columns independently; parse/write symmetry alone
            // could hide an identical shift in the decoder and formatter.
            let marker = format!("> EPH E01 {msg}");
            let start = lines.iter().position(|line| *line == marker).unwrap();
            assert!(lines[start + 6][61..].trim().is_empty());
            for (slot, expected) in [3.12, 0.0, -4.656612873077e-10, bgd_e5b]
                .into_iter()
                .enumerate()
            {
                let field = &lines[start + 7][4 + 19 * slot..23 + 19 * slot];
                assert_eq!(
                    field.trim().parse::<f64>().unwrap(),
                    expected,
                    "{msg} slot {slot}"
                );
            }
            assert_eq!(
                lines[start + 8][4..23].trim().parse::<f64>().unwrap(),
                transmit
            );
            assert!(lines[start + 8][23..].trim().is_empty());
        }
        let (back, diagnostics) = super::parse_diagnostics(&output);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(back.record, rinex.record);
    }
}

#[test]
fn galileo_nav4_zero_unknown_sisa_and_spares_keep_their_columns() {
    let source = super::navigation_excerpt(
        "BRD400DLR_S_20230710000_01D_MN.rnx",
        &[(1, 9), (61026, 61034)],
    );
    for sisa in [0.0, -1.0] {
        // Synthetic boundaries: Table A13's zero/NAPA SISA values and
        // nonnumeric spare contents, which section 6.4 requires ignoring.
        let text = source.replace(" 3.120000000000e+00", &format!("{sisa:19.12E}"));
        let mut lines: Vec<_> = text.lines().map(str::to_owned).collect();
        let spare = format!("{:>19}", "SPARE");
        lines[15].replace_range(61..80, &spare);
        lines[17].replace_range(23..80, &spare.repeat(3));
        let (rinex, diagnostics) = super::parse_diagnostics(&lines.join("\n"));
        assert!(diagnostics.is_empty(), "{sisa}: {diagnostics:?}");
        let records = rinex.record.as_nav().unwrap();
        assert_eq!(records.len(), 1);
        let eph = records.values().next().unwrap().as_ephemeris().unwrap();
        assert_eq!(eph.get_orbit_f64("sisa"), Some(sisa));
        assert_eq!(eph.get_orbit_f64("health"), Some(0.0));
        assert_eq!(eph.get_orbit_f64("bgdE5bE1"), Some(-6.984919309616e-10));
        assert_eq!(eph.get_orbit_f64("t_tm"), Some(664.0));
        assert!(eph.orbits.keys().all(|name| !name.starts_with("spare")));
        let mut writer = BufWriter::new(Vec::new());
        rinex.format(&mut writer).unwrap();
        let output = writer.into_inner().unwrap();
        let (back, diagnostics) = super::parse_diagnostics(std::str::from_utf8(&output).unwrap());
        assert!(diagnostics.is_empty());
        assert_eq!(back.record, rinex.record);
    }
}

#[test]
fn real_nav_keeps_zero_and_blank_distinct() {
    let rnx = parse(&navigation_rinex("4.00", GLO_BLOCK)).unwrap();
    let nav = rnx.record.as_nav().unwrap();
    assert_eq!(nav.len(), 1);
    let (key, frame) = nav.first_key_value().unwrap();
    assert_eq!(
        key.epoch,
        Epoch::from_str("2024-05-10T02:45:00 UTC").unwrap()
    );
    let eph = frame.as_ephemeris().unwrap();
    assert_eq!(
        eph.orbits.get("frameTime"),
        Some(&OrbitItem::F64(441_000.0))
    );
    assert_eq!(eph.clock_drift_rate, 0.0);
    assert_eq!(eph.orbits.get("accelY"), Some(&OrbitItem::F64(0.0)));
    assert_eq!(eph.orbits.get("ageOp"), Some(&OrbitItem::F64(0.0)));
    assert_eq!(eph.orbits.get("channel"), Some(&OrbitItem::I8(-1)));
    let rnx = parse(&navigation_rinex("4.02", GPS_BLOCK)).unwrap();
    let nav = rnx.record.as_nav().unwrap();
    let eph = nav.values().find_map(|f| f.as_ephemeris()).unwrap();
    assert_eq!(eph.orbits.get("fitInt"), Some(&OrbitItem::F64(4.0)));
    // Explicitly modified source: an omitted optional fit interval remains absent.
    let blank = GPS_BLOCK.replacen(" 4.000000000000E+00", "                   ", 1);
    let rnx = parse(&navigation_rinex("4.02", &blank)).unwrap();
    let eph = rnx
        .record
        .as_nav()
        .unwrap()
        .values()
        .find_map(|f| f.as_ephemeris())
        .unwrap();
    assert!(eph.orbits.get("fitInt").is_none());
}

#[test]
fn real_glonass_format_roundtrip_keeps_message_time_and_zero_fields() {
    let rnx = parse(&navigation_rinex("4.00", GLO_BLOCK)).unwrap();
    let mut writer = std::io::BufWriter::new(Vec::new());
    rnx.format(&mut writer).unwrap();
    let bytes = writer.into_inner().unwrap();
    let reparsed = parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
    assert_eq!(rnx.record, reparsed.record);
}

#[test]
fn synthetic_nav_native_integer_zero_is_a_value() {
    for (kind, expected) in [
        ("u8", OrbitItem::U8(0)),
        ("i8", OrbitItem::I8(0)),
        ("u32", OrbitItem::U32(0)),
        ("f64", OrbitItem::F64(0.0)),
    ] {
        assert_eq!(
            OrbitItem::new(
                "test",
                kind,
                "0.0",
                &NavMessageType::LNAV,
                Constellation::GPS
            )
            .unwrap(),
            expected
        );
        assert!(
            OrbitItem::new("test", kind, "", &NavMessageType::LNAV, Constellation::GPS).is_err()
        );
    }
}

#[test]
fn synthetic_zero_fit_interval_is_present() {
    let modified = GPS_BLOCK.replacen(" 4.000000000000E+00", " 0.000000000000E+00", 1);
    let rnx = parse(&navigation_rinex("4.02", &modified)).unwrap();
    let eph = rnx
        .record
        .as_nav()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .as_ephemeris()
        .unwrap();
    assert_eq!(eph.orbits.get("fitInt"), Some(&OrbitItem::F64(0.0)));
}

#[test]
fn synthetic_nav_invalid_orbit_field_keeps_field_and_cause() {
    let header = Header::basic_nav().with_version(Version::new(4, 2));
    for token in ["BAD", "NaN", "inf"] {
        let modified = GPS_BLOCK.replacen(" 1.100000000000E+02", &format!("{token:>19}"), 1);
        match crate::navigation::parse_epoch(&header, &modified) {
            Err(ParsingError::NavOrbitParsing { field, source }) => {
                assert_eq!(field, "iode");
                assert!(matches!(*source, ParsingError::NavNullOrbit));
            },
            other => panic!("expected invalid orbit field, got {other:?}"),
        }
    }
}

#[test]
fn sbas_real_transmission_time_is_not_week_or_quadratic_clock() {
    let nav = parse(&navigation_rinex("4.00", SBAS_BLOCK)).unwrap();
    assert_eq!(nav.nav_ephemeris_frames_iter().count(), 1);
    let (key, eph) = nav.nav_ephemeris_frames_iter().next().unwrap();
    assert_eq!(key.epoch.time_scale, TimeScale::GPST);
    assert_eq!(eph.get_orbit_f64("t_tm"), Some(442535.0));
    assert_eq!(eph.get_orbit_f64("week"), None);
    assert_eq!(eph.clock_drift_rate, 0.0);
    assert_eq!(eph.get_orbit_f64("health"), Some(0.0));
    assert_eq!(eph.get_orbit_f64("accuracyCode"), Some(4096.0));
    let mut writer = BufWriter::new(Vec::new());
    nav.format(&mut writer).unwrap();
    let bytes = writer.into_inner().unwrap();
    let roundtrip = parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
    assert_eq!(nav.record, roundtrip.record);
}

#[test]
fn sbas_transmission_week_boundary_and_fraction_preserved_in_v3_v4() {
    for tx in [-0.25, 0.0, 604800.25] {
        // Synthetic derivative: only the third scalar and version envelope change.
        let mut lines: Vec<String> = SBAS_BLOCK.lines().map(str::to_owned).collect();
        lines[1].replace_range(61..80, &format!("{tx:19.12e}"));
        for (version, start) in [("3.04", 1), ("4.00", 0)] {
            let body = lines[start..].join("\n") + "\n";
            let nav = parse(&navigation_rinex(version, &body)).unwrap();
            let (_, eph) = nav.nav_ephemeris_frames_iter().next().unwrap();
            assert_eq!(eph.get_orbit_f64("t_tm"), Some(tx));
            assert!(eph.get_orbit_f64("week").is_none());
            let mut writer = BufWriter::new(Vec::new());
            nav.format(&mut writer).unwrap();
            let output = writer.into_inner().unwrap();
            let again = parse(std::str::from_utf8(&output).unwrap()).unwrap();
            assert_eq!(nav.record, again.record);
        }
    }
}
