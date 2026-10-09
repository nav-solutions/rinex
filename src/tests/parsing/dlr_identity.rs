//! DLR regressions. Excerpts are selected in memory from the original file.
use crate::{
    navigation::{NavFrame, NavFrameType},
    prelude::*,
    record::ParsingDiagnosticKind,
};
use std::io::{BufReader, BufWriter};
// Original source lines, SHA-256:
// 9cffb1b1f2978c46e8352d78f603dd089ecd7f32312c1a6c4ae585b5d0e538b6.
fn real() -> String {
    super::navigation_excerpt(
        "BRD400DLR_S_20241310000_01D_MN.rnx",
        &[
            (1, 9),
            (22, 24),
            (94, 96),
            (361, 366),
            (379, 381),
            (385, 387),
            (391, 393),
            (394, 396),
            (1194, 1213),
            (62556, 62567),
            (89544, 89561),
            (113943, 113960),
            (116974, 116995),
            (149220, 149239),
        ],
    )
}
fn read(s: &str) -> (Rinex, Vec<crate::record::ParsingDiagnostic>) {
    Rinex::parse_with_diagnostics(&mut BufReader::new(s.as_bytes())).unwrap()
}
#[test]
fn real_dlr_identity_repairs_keep_unresolved_content_conflicts() {
    let (r, d) = read(&real());
    assert_eq!(d.len(), 4, "{d:?}");
    for diag in &d {
        assert!(matches!(
            diag.kind,
            ParsingDiagnosticKind::DuplicateNavigation {
                conflicting: true,
                ..
            }
        ));
    }
    let records = r.record.as_nav().unwrap();
    assert_eq!(records.len(), 16);
    let cnv: Vec<_> = records
        .iter()
        .filter(|(k, _)| k.sv.to_string() == "C19")
        .collect();
    assert_eq!(cnv.len(), 2);
    assert_eq!(cnv[0].0.epoch, cnv[1].0.epoch);
    assert_ne!(cnv[0].0.transmission_time, cnv[1].0.transmission_time);
    let navic: Vec<_> = records
        .iter()
        .filter_map(|(k, f)| f.as_system_time().filter(|_| k.sv.to_string() == "I02"))
        .collect();
    assert_eq!(navic.len(), 4);
    assert!(navic
        .iter()
        .any(|s| s.time_system.as_deref() == Some("IRGL")));
    assert!(navic.iter().any(|s| s.utc.as_deref() == Some("UTCIRN")));
    assert!(navic.iter().any(|s| s.utc.as_deref() == Some("UTC(NPLI)")));
    let gps: Vec<_> = records
        .iter()
        .filter_map(|(k, f)| {
            f.as_system_time()
                .filter(|_| k.sv.constellation == Constellation::GPS)
        })
        .collect();
    assert_eq!(gps.len(), 2);
    assert_eq!(gps[0].transmission_time, Some(-167010.0));
    assert_eq!(gps[1].transmission_time, Some(-115260.0));
    assert_eq!(
        Rinex::parse(&mut BufReader::new(real().as_bytes()))
            .unwrap()
            .record,
        r.record
    );
}
#[test]
fn dlr_write_read_preserves_signed_transmit_and_sto_indicators() {
    let (r, d) = read(&real());
    assert_eq!(d.len(), 4);
    let mut w = BufWriter::new(Vec::new());
    r.format(&mut w).unwrap();
    let bytes = w.into_inner().unwrap();
    let (back, d) = read(std::str::from_utf8(&bytes).unwrap());
    assert!(d.is_empty(), "{d:?}");
    assert_eq!(back.record, r.record);
    let records = r.record.as_nav().unwrap();
    let mut lhs = std::collections::BTreeMap::new();
    lhs.extend(records.clone());
    assert_eq!(&lhs, records);
}
#[test]
fn sbas_and_utc_columns_are_separate_and_roundtrip() {
    // Synthetic Table A30 header using actual DLR GPUT data coefficients.
    let header = super::navigation_rinex("4.00", "");
    let body=format!("> STO S   SBAS\n    2024 05 10 00 00 00 {:<18} {:<18} {:<18}\n     0.000000000000e+00 9.313225746155e-10 8.881784197001e-15 0.000000000000e+00\n","SBUT","WAAS","UTC(USNO)");
    let (r, d) = read(&(header + &body));
    assert!(d.is_empty(), "{d:?}");
    let (key, frame) = r.record.as_nav().unwrap().first_key_value().unwrap();
    assert_eq!(key.frmtype, NavFrameType::SystemTimeOffset);
    assert_eq!(
        key.transmission_time,
        Some(crate::navigation::TransmissionTime::new(0.0).unwrap())
    );
    let NavFrame::STO(sto) = frame else {
        panic!("STO")
    };
    assert_eq!(sto.sbas.as_deref(), Some("WAAS"));
    assert_eq!(sto.utc.as_deref(), Some("UTC(USNO)"));
    let mut w = BufWriter::new(Vec::new());
    r.format(&mut w).unwrap();
    let bytes = w.into_inner().unwrap();
    let (back, d) = read(std::str::from_utf8(&bytes).unwrap());
    assert!(d.is_empty());
    assert_eq!(back.record, r.record);
}
#[test]
fn invalid_sto_transmission_and_missing_programmatic_value_do_not_guess() {
    let header = super::navigation_rinex("4.00", "");
    for scalar in [
        "                   ",
        "                NaN",
        "                inf",
    ] {
        let text=format!("{header}> STO G   LNAV\n    2024 05 10 00 00 00 GPUT\n    {scalar} 9.313225746155e-10 8.881784197001e-15 0.000000000000e+00\n");
        let (r, d) = read(&text);
        assert!(r.record.as_nav().unwrap().is_empty());
        assert!(matches!(
            d[0].kind,
            ParsingDiagnosticKind::NavigationFailure(ParsingError::NavTransmissionTime)
        ));
    }
    let mut sto = crate::navigation::TimeOffset::from_epoch(
        "2024-05-10T00:00:00 GPST".parse().unwrap(),
        TimeScale::GPST,
        TimeScale::UTC,
        (0.0, 0.0, 0.0),
    );
    let mut w = BufWriter::new(Vec::new());
    assert!(matches!(
        sto.format_v4(&mut w),
        Err(crate::FormattingError::NavMissingTransmissionTime)
    ));
    sto.transmission_time = Some(-0.5);
    sto.format_v4(&mut w).unwrap();
}
#[test]
#[cfg(feature = "serde")]
fn legacy_serialized_key_defaults_missing_identity_fields() {
    let r = super::parse(&super::navigation_rinex("4.02", super::GPS_BLOCK)).unwrap();
    let key = r.record.as_nav().unwrap().keys().next().unwrap();
    let mut j = serde_json::to_value(key).unwrap();
    j.as_object_mut().unwrap().remove("transmission_time");
    j.as_object_mut().unwrap().remove("sto_identity");
    let old: crate::navigation::NavKey = serde_json::from_value(j).unwrap();
    assert_eq!(old.transmission_time, None);
    assert_eq!(old.sto_identity, None);
}
#[test]
fn eop_distinct_transmissions_are_retained() {
    let excerpt =
        super::navigation_excerpt("BRD400DLR_S_20241310000_01D_MN.rnx", &[(1, 9), (415, 418)]);
    let lines: Vec<_> = excerpt.split_inclusive('\n').collect();
    let header = lines[..9].concat();
    let block = lines[9..].concat();
    // Synthetic retransmission: same reference epoch, 60 seconds later.
    let changed = block.replace("4.322400000000e+05", "4.323000000000e+05");
    assert_ne!(changed, block);
    let source = format!("{header}{block}{changed}");
    let (rinex, diagnostics) =
        Rinex::parse_with_diagnostics(&mut BufReader::new(source.as_bytes())).unwrap();
    assert_eq!(rinex.record.as_nav().unwrap().len(), 2);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn finite_transmission_identity_orders_hashes_and_preserves_subnanoseconds() {
    use crate::navigation::TransmissionTime as T;
    use std::hash::{Hash, Hasher};
    let hash = |v: T| {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        v.hash(&mut h);
        h.finish()
    };
    let p = T::new(0.0).unwrap();
    let n = T::new(-0.0).unwrap();
    assert_eq!(p, n);
    assert_eq!(p.cmp(&n), std::cmp::Ordering::Equal);
    assert_eq!(hash(p), hash(n));
    assert_eq!(n.seconds().to_bits(), 0.0_f64.to_bits());
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(T::new(v).is_err());
    }
    let a = T::new(1e-10).unwrap();
    let b = T::new(4e-10).unwrap();
    assert_ne!(a, b);
    assert!(a < b);
    let sorted = [-604800.25, -0.5, 0.0, 1e-10, 4e-10, 604800.5, 1e99].map(|s| T::new(s).unwrap());
    for pair in sorted.windows(2) {
        assert!(pair[0] < pair[1]);
    }
    #[cfg(feature = "serde")]
    {
        for v in sorted {
            assert_eq!(
                serde_json::from_str::<T>(&serde_json::to_string(&v).unwrap()).unwrap(),
                v
            );
        }
        assert!(serde_json::from_str::<T>("1e999").is_err());
        use serde::de::value::{Error, F64Deserializer};
        for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                <T as serde::Deserialize>::deserialize(F64Deserializer::<Error>::new(v)).is_err()
            );
        }
    }
}

#[test]
fn transmission_source_field_depends_on_glonass_message() {
    // Unmodified RINEX 4.02 specification examples, source lines 37-62.
    let text = super::navigation_excerpt(
        "BRD400DLR_S_20241310000_01D_MN.rnx",
        &[(1, 9), (62862, 62867)],
    );
    let (r, _) = read(&text);
    let (key, frame) = r.record.as_nav().unwrap().first_key_value().unwrap();
    assert_eq!(key.transmission_time.unwrap().seconds(), 441000.0);
    assert_eq!(
        frame.as_ephemeris().unwrap().get_orbit_f64("frameTime"),
        Some(441000.0)
    );
    let source = super::navigation_text("data/NAV/V4/rinex402_examples_MN.rnx");
    let (r, _) = read(&source);
    let cdma: Vec<_> = r
        .record
        .as_nav()
        .unwrap()
        .iter()
        .filter(|(k, _)| {
            matches!(
                k.msgtype,
                crate::navigation::NavMessageType::L1OC | crate::navigation::NavMessageType::L3OC
            )
        })
        .collect();
    assert_eq!(cdma.len(), 2);
    for (k, f) in cdma {
        let eph = f.as_ephemeris().unwrap();
        assert_eq!(k.transmission_time.unwrap().seconds(), 518400.0); // Table A16/A17 raw final column
        assert_eq!(eph.get_orbit_f64("t_tm"), Some(518400.0));
        assert_eq!(eph.get_orbit_f64("frameTime"), None);
        let mut missing = f.clone();
        missing.as_mut_ephemeris().unwrap().orbits.remove("t_tm");
        assert!(crate::navigation::NavKey::from_frame(
            Version::new(4, 2),
            k.epoch,
            k.sv,
            k.msgtype,
            k.subtype,
            &missing
        )
        .is_err());
    }
}

#[test]
fn source_markers_missing_and_zero_follow_message_version_tables() {
    for (version, marker, other) in [
        ("3.04", 999900000.0, 999999999.999),
        ("4.00", 999900000.0, 999999999.999),
        ("4.02", 999999999.999, 999900000.0),
    ] {
        for (value, missing) in [
            (Some(marker), true),
            (None, true),
            (Some(0.0), false),
            (Some(other), false),
            (Some(-0.5), false),
            (Some(604800.25), false),
            (Some(1e-10), false),
            (Some(4e-10), false),
        ] {
            let scalar = value
                .map(|s| crate::navigation::formatting::NavFormatter::new(s).to_string())
                .unwrap_or_else(|| "                   ".to_string());
            let block = super::GPS_BLOCK.replace(" 4.248180000000E+05", &scalar);
            let block = if version.starts_with('3') {
                block.lines().skip(1).collect::<Vec<_>>().join("\n") + "\n"
            } else {
                block
            };
            let (r, d) = read(&super::navigation_rinex(version, &block));
            assert!(d.is_empty(), "{version}: {d:?}");
            let (k, f) = r.record.as_nav().unwrap().first_key_value().unwrap();
            assert_eq!(
                k.transmission_time.is_none(),
                missing,
                "{version}: {value:?}"
            );
            assert_eq!(f.as_ephemeris().unwrap().get_orbit_f64("t_tm"), value);
        }
    }
    // Modern GPS CNAV cannot omit its t_tm just because the file is NAV4.
    let source = super::navigation_excerpt(
        "BRD400DLR_S_20241310000_01D_MN.rnx",
        &[(1, 9), (149220, 149229)],
    );
    let lines: Vec<_> = source.lines().collect();
    let mut body = lines[9..].iter().map(|l| l.to_string()).collect::<Vec<_>>();
    body[9].replace_range(4..23, "                   ");
    let (r, d) = read(&super::navigation_rinex("4.00", &(body.join("\n") + "\n")));
    assert!(r.record.as_nav().unwrap().is_empty());
    assert!(matches!(
        d[0].kind,
        ParsingDiagnosticKind::NavigationFailure(ParsingError::NavTransmissionTime)
    ));
}

#[test]
fn sto_eop_exponents_signed_fraction_and_bad_fields_are_not_zero_filled() {
    let source =
        super::navigation_excerpt("BRD400DLR_S_20241310000_01D_MN.rnx", &[(1, 9), (415, 418)]);
    let lines: Vec<_> = source.lines().collect();
    let block = lines[9..].join("\n") + "\n";
    for v in [
        -604800.25,
        -0.5,
        -0.0,
        0.0,
        604800.5,
        1e-10,
        4e-10,
        999900000.0,
    ] {
        for exponent in ['E', 'e', 'D', 'd'] {
            let scalar = crate::navigation::formatting::NavFormatter::new(v)
                .to_string()
                .replace('E', &exponent.to_string());
            let text = block
                .replace(" 4.322400000000e+05", &scalar)
                .replace('e', &exponent.to_string());
            let (r, d) = read(&super::navigation_rinex("4.00", &text));
            assert!(d.is_empty(), "{v} {exponent}: {d:?}");
            let (k, f) = r.record.as_nav().unwrap().first_key_value().unwrap();
            assert_eq!(k.transmission_time.unwrap().seconds(), v);
            assert_eq!(f.as_earth_orientation().unwrap().t_tm, v);
        }
    }
    for bad in [
        "                   ",
        "                BAD",
        "                NaN",
        "                inf",
    ] {
        for scalar in [" 4.322400000000e+05", "-2.077859640121e-02"] {
            let text = block.replace(scalar, bad);
            assert_ne!(text, block);
            let (r, d) = read(&super::navigation_rinex("4.00", &text));
            assert!(r.record.as_nav().unwrap().is_empty());
            assert_eq!(d.len(), 1);
        }
    }
    let mut short = block.lines().map(str::to_string).collect::<Vec<_>>();
    short[3].truncate(50);
    let (r, d) = read(&super::navigation_rinex("4.00", &short.join("\n")));
    assert!(r.record.as_nav().unwrap().is_empty());
    assert_eq!(d.len(), 1);
}

#[test]
fn sto_independent_identifiers_and_required_numeric_columns() {
    let line = |pair: &str, sbas: &str, utc: &str| {
        format!("    2024 05 10 00 00 00 {pair:<18} {sbas:<18} {utc:<18}")
    };
    let coefficients =
        "     0.000000000000E+00 9.313225746155E-10 8.881784197001E-15 0.000000000000E+00";
    let mut body = String::new();
    for (pair, sbas, utc) in [
        ("SBUT", "WAAS", "UTC(USNO)"),
        ("GPUT", "WAAS", "UTC(USNO)"),
        ("SBUT", "EGNOS", "UTC(USNO)"),
        ("SBUT", "WAAS", "UTC(NPLI)"),
    ] {
        body += &format!(
            "> STO S   SBAS\n{}\n{coefficients}\n",
            line(pair, sbas, utc)
        );
    }
    let (r, d) = read(&super::navigation_rinex("4.00", &body));
    assert!(d.is_empty());
    assert_eq!(r.record.as_nav().unwrap().len(), 4);
    for value in [-604800.5, -0.5, 0.0, 604800.25, 1e-10, 4e-10] {
        for exponent in ['E', 'e', 'D', 'd'] {
            let mut second = coefficients.to_string();
            second.replace_range(
                4..23,
                &crate::navigation::formatting::NavFormatter::new(value).to_string(),
            );
            let second = second.replace('E', &exponent.to_string());
            let sto =
                crate::navigation::TimeOffset::parse_v4(&line("GPUT", "", "UTC(USNO)"), &second)
                    .unwrap();
            assert_eq!(sto.transmission_time, Some(value));
            assert_eq!(
                sto.polynomial,
                (9.313225746155e-10, 8.881784197001e-15, 0.0)
            );
        }
    }
    for start in [4, 23, 42, 61] {
        for bad in [
            "                   ",
            "                BAD",
            "                NaN",
            "                inf",
        ] {
            let mut second = coefficients.to_string();
            second.replace_range(start..start + 19, bad);
            assert!(
                crate::navigation::TimeOffset::parse_v4(&line("GPUT", "", ""), &second).is_err()
            );
        }
    }
    for (first, second) in [
        ("short".to_string(), coefficients.to_string()),
        (line("GPUT", "", ""), "short".to_string()),
        (
            line("GPUT", "1234567890123456789", "123456789012345678"),
            coefficients.to_string(),
        ),
        (line("GPUT", "非ASCII", ""), coefficients.to_string()),
    ] {
        assert!(crate::navigation::TimeOffset::parse_v4(&first, &second).is_err());
    }
    let (k, frame) = r.record.as_nav().unwrap().first_key_value().unwrap();
    let mut changed = frame.clone();
    let NavFrame::STO(sto) = &mut changed else {
        panic!("STO")
    };
    sto.t_ref.1 += 1_000_000_000;
    assert!(matches!(
        crate::navigation::NavKey::from_frame(
            Version::new(4, 0),
            k.epoch,
            k.sv,
            k.msgtype,
            k.subtype,
            &changed
        ),
        Err(ParsingError::NavFrameIdentity)
    ));
}
