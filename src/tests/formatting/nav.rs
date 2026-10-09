//! Navigation RINEX formatting, compared to the original files.
use crate::{navigation::NavFrameType, prelude::Rinex};

use std::collections::HashMap;

/// Independent raw-column grouping. A complete block supplies its identity;
/// same identity replaces the preceding block in source order. No production
/// key constructor or parser participates in this formatting oracle.
fn v4_record_blocks(content: &str) -> HashMap<String, Vec<String>> {
    fn insert(blocks: &mut HashMap<String, Vec<String>>, header: &str, block: Vec<String>) {
        if block.is_empty() {
            return;
        }
        let mut key = format!("{}\n{}", header, &block[0][..23]);
        let number = |line: &str, start: usize| {
            let text = line
                .get(start..(start + 19).min(line.len()))
                .unwrap_or("")
                .trim();
            if text.is_empty() {
                "absent".to_string()
            } else {
                format!(
                    "{:e}",
                    text.replace(['D', 'd'], "E").parse::<f64>().unwrap()
                )
            }
        };
        if header.starts_with("> STO") {
            key += &format!(
                "|{}|{}",
                block[0].get(24..).unwrap_or("").trim_end(),
                number(&block[1], 4)
            );
        } else if header.starts_with("> EOP") {
            key += &format!("|{}", number(&block[2], 4));
        } else if header.starts_with("> EPH") {
            let c = header.as_bytes()[6] as char;
            let msg = header.split_ascii_whitespace().nth(3).unwrap();
            // Transmission slot indices come from the published message tables.
            let row = match (c, msg) {
                ('R', "FDMA") | ('S', _) => 0,
                ('G' | 'J', "CNAV") => 8,
                ('G' | 'J', "CNV2") | ('C', "CNV1" | "CNV2") => 9,
                ('C', "CNV3") | ('R', "L1OC" | "L3OC") | ('I', "L1NV") => 8,
                _ => 7,
            };
            key += &format!(
                "|{}",
                number(
                    block.get(row).map(String::as_str).unwrap_or(""),
                    if row == 0 || c == 'R' { 61 } else { 4 }
                )
            );
            if c == 'E' {
                key += &format!("|{}", number(&block[5], 23));
            }
        }
        blocks.insert(key, block);
    }
    let mut blocks = HashMap::new();
    let mut header = None;
    let mut block = Vec::new();
    for line in content
        .lines()
        .skip_while(|l| !l.contains("END OF HEADER"))
        .skip(1)
    {
        let line = line.trim_end().to_string();
        if line.starts_with("> ") {
            if let Some(previous) = header.replace(line) {
                insert(&mut blocks, &previous, std::mem::take(&mut block));
            }
        } else {
            let sto_header =
                header.as_ref().is_some_and(|h| h.starts_with("> STO")) && block.is_empty();
            block.push(if sto_header {
                line
            } else {
                line.replace(['e', 'd', 'D'], "E")
            });
        }
    }
    if let Some(header) = header {
        insert(&mut blocks, &header, block);
    }
    blocks
}

fn audited_snapshot(path: &str) -> Rinex {
    let text = crate::tests::parsing::navigation_text(path);
    let (rinex, diagnostics) =
        Rinex::parse_with_diagnostics(&mut std::io::BufReader::new(text.as_bytes())).unwrap();
    assert!(
        diagnostics.iter().all(|d| matches!(
            d.kind,
            crate::record::ParsingDiagnosticKind::DuplicateNavigation { .. }
        )),
        "{path}: {diagnostics:?}"
    );
    assert_eq!(
        Rinex::parse(&mut std::io::BufReader::new(text.as_bytes()))
            .unwrap()
            .record,
        rinex.record
    );
    rinex
}

fn v4_records_write_back(name: &str) {
    let path = format!("data/NAV/V4/{}", name);
    let original = audited_snapshot(&path);

    let mut buffer = std::io::BufWriter::new(Vec::new());
    original.format(&mut buffer).unwrap();
    let written = String::from_utf8(buffer.into_inner().unwrap()).unwrap();

    let original_text = crate::tests::parsing::navigation_text(&path);

    let model = v4_record_blocks(&original_text);
    let dut = v4_record_blocks(&written);

    let mut compared = 0;
    for (key, written) in dut.iter() {
        if key.starts_with("> EPH") {
            continue;
        }
        let block = model
            .get(key)
            .unwrap_or_else(|| panic!("written record not in the original file:\n{}", key));

        let (block, written) =
            if key.starts_with("> ION") && block.len() == 3 && block[2].len() == 23 {
                // Klobuchar blank region retains the established worldwide mapping.
                let mut block = block.clone();
                block[2].push_str(" 0.000000000000E+00");
                (block, written.clone())
            } else {
                (block.clone(), written.clone())
            };

        assert_eq!(written, block, "record differs:\n{}", key);
        compared += 1;
    }

    let expected = original
        .record
        .as_nav()
        .unwrap()
        .iter()
        .filter(|(k, _)| k.frmtype != NavFrameType::Ephemeris)
        .count();

    assert_eq!(
        compared, expected,
        "not every STO, EOP or ION record written"
    );
    assert!(compared > 0, "no STO, EOP or ION record in {}", name);
}

#[test]
fn nav_v4_kms300dnk_records_write_back() {
    v4_records_write_back("KMS300DNK_R_20221591000_01H_MN.rnx.gz");
}

#[test]
fn nav_v4_brd400dlr_records_write_back() {
    v4_records_write_back("BRD400DLR_S_20230710000_01D_MN.rnx.gz");
}

/// RINEX 4.02 specification examples: NavIC and GLONASS CDMA ION
/// records, STO records with the new time system pairs and blank PRNs.
#[test]
fn nav_v4_02_examples_records_write_back() {
    v4_records_write_back("rinex402_examples_MN.rnx");
}

use crate::{
    navigation::{NavFrame, NavKey, NavMessageType},
    prelude::{Constellation, Version},
};

/// Writes `rinex` in the revision of its (modified) header to a
/// temporary file and parses it back.
fn write_and_reparse(rinex: &Rinex, tag: &str) -> Result<Rinex, crate::error::FormattingError> {
    let _ = tag;
    let mut buffer = std::io::BufWriter::new(Vec::new());
    rinex.format(&mut buffer)?;
    let bytes = buffer.into_inner().unwrap();
    Ok(Rinex::parse(&mut std::io::BufReader::new(bytes.as_slice())).unwrap())
}

/// The ephemeris written in another revision must carry the values of
/// the original: same clock fields, every orbit field of the reparsed
/// record equal to the original's (the RINEX 3 definitions may omit
/// fields of the RINEX 4 messages, and name the BeiDou group delays
/// differently).
fn assert_ephemeris_preserved(original: &NavFrame, reparsed: &NavFrame, k: &NavKey) {
    let (a, b) = (
        original.as_ephemeris().unwrap(),
        reparsed.as_ephemeris().unwrap(),
    );
    assert_eq!(a.clock_bias, b.clock_bias, "clock bias {:?}", k);
    assert_eq!(a.clock_drift, b.clock_drift, "clock drift {:?}", k);
    assert_eq!(
        a.clock_drift_rate, b.clock_drift_rate,
        "clock drift rate {:?}",
        k
    );

    fn alias(name: &str) -> &str {
        match name {
            "tgdb1b3" => "tgd1b1b3",
            "tgdb2b3" => "tgd2b2b3",
            "tgd1b1b3" => "tgdb1b3",
            "tgd2b2b3" => "tgdb2b3",
            other => other,
        }
    }

    assert!(!b.orbits.is_empty(), "no orbit field at {:?}", k);
    for (name, value) in b.orbits.iter() {
        let expected = a
            .orbits
            .get(name)
            .or_else(|| a.orbits.get(alias(name)))
            .unwrap_or_else(|| {
                panic!("{} written at {:?} does not exist in the original", name, k)
            });
        assert_eq!(
            value.as_f64(),
            expected.as_f64(),
            "{} differs at {:?}",
            name,
            k
        );
    }
}

/// A RINEX 4 file mixing modern (CNAV, CNV1 to CNV3) and legacy messages
/// written as RINEX 3 keeps every legacy ephemeris, and only those: one
/// unrepresentable frame must not abort the whole file.
#[test]
fn nav_v4_written_as_v3_keeps_the_legacy_ephemerides() {
    for name in [
        "KMS300DNK_R_20221591000_01H_MN.rnx.gz",
        "BRD400DLR_S_20230710000_01D_MN.rnx.gz",
    ] {
        let mut rinex = audited_snapshot(&format!("data/NAV/V4/{}", name));
        let original = rinex.record.as_nav().unwrap().clone();

        let legacy = original
            .iter()
            .filter(|(k, v)| v.as_ephemeris().is_some() && k.msgtype.is_legacy(k.sv.constellation))
            .collect::<Vec<_>>();
        let modern = original
            .iter()
            .filter(|(k, v)| v.as_ephemeris().is_some() && !k.msgtype.is_legacy(k.sv.constellation))
            .count();
        // BRD400DLR mixes LNAV with CNAV and CNV1 to CNV3
        if name.starts_with("BRD400DLR") {
            assert!(modern > 0, "{} carries no modern message", name);
        }

        // NAV3 retains Galileo's full source identity. Other legacy message
        // names are normalized to the pre-existing NAV3 LNAV representation.
        let expected = legacy
            .iter()
            .map(|(k, _)| (k.epoch, k.sv, k.galileo_data_sources, k.transmission_time))
            .collect::<std::collections::BTreeSet<_>>()
            .len();

        rinex.header.version = Version::new(3, 5);
        let parsed = write_and_reparse(&rinex, "v4-as-v3").unwrap();
        assert_eq!(parsed.header.version, Version::new(3, 5));

        let record = parsed.record.as_nav().unwrap();
        assert_eq!(record.len(), expected, "{}", name);

        for (k, frame) in record.iter() {
            if k.sv.constellation != Constellation::Galileo {
                assert_eq!(k.msgtype, NavMessageType::LNAV);
            }
            // Match the retained full source identity.
            let (_, original) = legacy
                .iter()
                .filter(|(o, _)| {
                    o.epoch == k.epoch
                        && o.sv == k.sv
                        && o.galileo_data_sources == k.galileo_data_sources
                        && o.transmission_time == k.transmission_time
                })
                .next()
                .unwrap_or_else(|| panic!("{:?} was not in the original file", k));
            assert_ephemeris_preserved(original, frame, k);
        }
    }
}

/// The rinexfetch use case: the rapid BRD400DLR product filtered to GPS
/// (LNAV, CNAV and CNV2 messages) written as RINEX 3 keeps every LNAV
/// ephemeris, the modern messages have no RINEX 3 representation.
#[test]
fn nav_v4_gps_only_written_as_v3() {
    let mut rinex = audited_snapshot("data/NAV/V4/BRD400DLR_S_20230710000_01D_MN.rnx.gz");
    rinex
        .record
        .as_mut_nav()
        .unwrap()
        .retain(|k, _| k.sv.constellation == Constellation::GPS);

    let original = rinex.record.as_nav().unwrap().clone();
    let lnav = original
        .iter()
        .filter(|(k, v)| v.as_ephemeris().is_some() && k.msgtype == NavMessageType::LNAV)
        .collect::<Vec<_>>();
    let modern = original
        .iter()
        .filter(|(k, v)| v.as_ephemeris().is_some() && k.msgtype != NavMessageType::LNAV)
        .count();
    assert!(!lnav.is_empty() && modern > 0);

    rinex.header.version = Version::new(3, 5);
    let parsed = write_and_reparse(&rinex, "gps-as-v3").unwrap();

    let record = parsed.record.as_nav().unwrap();
    assert_eq!(record.len(), lnav.len());
    for (k, frame) in record.iter() {
        let (_, original) = lnav
            .iter()
            .find(|(o, _)| {
                o.epoch == k.epoch
                    && o.sv == k.sv
                    && o.galileo_data_sources == k.galileo_data_sources
                    && o.transmission_time == k.transmission_time
            })
            .unwrap_or_else(|| panic!("{:?} was not in the original file", k));
        assert_ephemeris_preserved(original, frame, k);
    }
}

/// A RINEX 3 file written as RINEX 4 gets the message type of each
/// constellation, and written back as RINEX 3 gives the original record.
#[test]
fn nav_v3_written_as_v4_gets_message_types() {
    let mut rinex = Rinex::from_file("data/NAV/V3/AMEL00NLD_R_20210010000_01D_MN.rnx").unwrap();
    let original = rinex.record.as_nav().unwrap().clone();

    rinex.header.version = Version::new(4, 0);
    let parsed = write_and_reparse(&rinex, "v3-as-v4").unwrap();

    let record = parsed.record.as_nav().unwrap();
    assert_eq!(record.len(), original.len());

    let mut seen = std::collections::BTreeSet::new();
    for (k, frame) in record.iter() {
        let expected = match k.sv.constellation {
            Constellation::GPS => NavMessageType::LNAV,
            Constellation::Glonass => NavMessageType::FDMA,
            Constellation::Galileo => {
                let source = frame
                    .as_ephemeris()
                    .unwrap()
                    .get_orbit_f64("source")
                    .unwrap_or(0.0) as u32;
                if source & 0x02 != 0 {
                    NavMessageType::FNAV
                } else {
                    NavMessageType::INAV
                }
            },
            Constellation::BeiDou => {
                if k.sv.prn <= 5 || k.sv.prn >= 59 {
                    NavMessageType::D2
                } else {
                    NavMessageType::D1
                }
            },
            c if c.is_sbas() => NavMessageType::SBAS,
            _ => NavMessageType::LNAV,
        };
        assert_eq!(k.msgtype, expected, "{:?}", k);
        seen.insert(k.sv.constellation);

        let key = NavKey {
            msgtype: if k.sv.constellation == Constellation::Galileo {
                k.msgtype
            } else {
                NavMessageType::LNAV
            },
            ..*k
        };
        let original = original
            .get(&key)
            .unwrap_or_else(|| panic!("{:?} not found", key));
        assert_ephemeris_preserved(original, frame, k);
    }
    assert!(seen.contains(&Constellation::Glonass) && seen.contains(&Constellation::BeiDou));

    // and back
    let mut parsed = parsed;
    parsed.header.version = Version::new(3, 5);
    let back = write_and_reparse(&parsed, "v4-back-as-v3").unwrap();
    assert_eq!(back.record.as_nav().unwrap(), &original);
}

/// A mixed RINEX 3 file written as a RINEX 2 GPS file keeps the GPS
/// ephemerides only.
#[test]
fn nav_v3_mixed_written_as_v2_gps() {
    let mut rinex = Rinex::from_file("data/NAV/V3/CBW100NLD_R_20210010000_01D_MN.rnx").unwrap();
    let original = rinex.record.as_nav().unwrap().clone();

    let gps = original
        .iter()
        .filter(|(k, _)| k.sv.constellation == Constellation::GPS)
        .count();
    assert!(gps > 0 && gps < original.len());

    rinex.header.version = Version::new(2, 11);
    rinex.header.constellation = Some(Constellation::GPS);
    let parsed = write_and_reparse(&rinex, "v3-as-v2").unwrap();

    let record = parsed.record.as_nav().unwrap();
    assert_eq!(record.len(), gps);
    for (k, frame) in record.iter() {
        assert_eq!(k.sv.constellation, Constellation::GPS);
        assert_ephemeris_preserved(&original[k], frame, k);
    }
}

/// Nothing representable in the target revision is an error, not an
/// empty file: a RINEX 4 record reduced to its modern messages cannot
/// be written as RINEX 3.
#[test]
fn nav_nothing_representable_is_an_error() {
    let mut rinex = audited_snapshot("data/NAV/V4/BRD400DLR_S_20230710000_01D_MN.rnx.gz");
    rinex
        .record
        .as_mut_nav()
        .unwrap()
        .retain(|k, _| matches!(k.msgtype, NavMessageType::CNAV | NavMessageType::CNV2));
    assert!(!rinex.record.as_nav().unwrap().is_empty());

    rinex.header.version = Version::new(3, 5);
    let result = write_and_reparse(&rinex, "cnav-as-v3");
    assert!(
        matches!(
            result,
            Err(crate::error::FormattingError::NoRepresentableFrame)
        ),
        "{:?}",
        result.map(|_| ())
    );
}

#[test]
fn nav_key_rebuild_and_write_validation_use_payload_identity() {
    use crate::{
        navigation::{NavKey, OrbitItem, TransmissionTime},
        FormattingError,
    };
    let text = crate::tests::parsing::navigation_rinex("4.02", crate::tests::parsing::GPS_BLOCK);
    let mut rinex = Rinex::parse(&mut std::io::BufReader::new(text.as_bytes())).unwrap();
    let record = rinex.record.as_mut_nav().unwrap();
    let (key, mut frame) = record.pop_first().unwrap();
    assert_eq!(
        NavKey::from_frame(
            Version::new(4, 2),
            key.epoch,
            key.sv,
            key.msgtype,
            key.subtype,
            &frame
        )
        .unwrap(),
        key
    );
    frame
        .as_mut_ephemeris()
        .unwrap()
        .orbits
        .insert("t_tm".to_string(), OrbitItem::F64(-0.5));
    record.insert(key, frame.clone());
    let mut buffer = std::io::BufWriter::new(Vec::new());
    assert!(matches!(
        crate::navigation::format(&mut buffer, record, &rinex.header),
        Err(FormattingError::NavIdentityMismatch)
    ));
    assert!(buffer.into_inner().unwrap().is_empty());
    record.remove(&key);
    let rebuilt = NavKey::from_frame(
        Version::new(4, 2),
        key.epoch,
        key.sv,
        key.msgtype,
        key.subtype,
        &frame,
    )
    .unwrap();
    assert_eq!(
        rebuilt.transmission_time,
        Some(TransmissionTime::new(-0.5).unwrap())
    );
    record.insert(rebuilt, frame.clone());
    let back = write_and_reparse(&rinex, "rebuilt").unwrap();
    assert_eq!(back.record, rinex.record);
    // Frame type, complete identity and finite values are checked before this record.
    for wrong in [
        NavKey {
            frmtype: NavFrameType::EarthOrientation,
            ..rebuilt
        },
        NavKey {
            galileo_data_sources: Some(258),
            ..rebuilt
        },
        NavKey {
            sto_identity: Some(crate::navigation::StoIdentity {
                time_system: *b"GPUT",
                sbas_id: [b' '; 18],
                utc_id: [b' '; 18],
            }),
            ..rebuilt
        },
    ] {
        let record = [(wrong, frame.clone())].into_iter().collect();
        let mut buffer = std::io::BufWriter::new(Vec::new());
        assert!(matches!(
            crate::navigation::format(&mut buffer, &record, &rinex.header),
            Err(FormattingError::NavIdentityMismatch)
        ));
        assert!(buffer.into_inner().unwrap().is_empty());
    }
    for value in [1.23456789012345, 1e100, 1e-100] {
        frame
            .as_mut_ephemeris()
            .unwrap()
            .orbits
            .insert("t_tm".to_string(), OrbitItem::F64(value));
        let k = NavKey::from_frame(
            Version::new(4, 2),
            key.epoch,
            key.sv,
            key.msgtype,
            key.subtype,
            &frame,
        )
        .unwrap();
        let record = [(k, frame.clone())].into_iter().collect();
        let mut buffer = std::io::BufWriter::new(Vec::new());
        assert!(
            matches!(
                crate::navigation::format(&mut buffer, &record, &rinex.header),
                Err(FormattingError::NavUnrepresentableIdentity)
            ),
            "{value}"
        );
        assert!(buffer.into_inner().unwrap().is_empty());
    }
}

#[test]
fn nav_unknown_transmission_maps_to_target_marker_without_becoming_a_time() {
    let text = crate::tests::parsing::navigation_rinex(
        "4.00",
        &crate::tests::parsing::GPS_BLOCK.replace("4.248180000000E+05", "9.999000000000E+08"),
    );
    let rinex = Rinex::parse(&mut std::io::BufReader::new(text.as_bytes())).unwrap();
    assert_eq!(
        rinex
            .record
            .as_nav()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .transmission_time,
        None
    );
    for version in [Version::new(3, 4), Version::new(4, 0), Version::new(4, 2)] {
        let mut source = rinex.clone();
        source.header.version = version;
        let back = write_and_reparse(&source, "unknown").unwrap();
        let (k, f) = back.record.as_nav().unwrap().first_key_value().unwrap();
        assert_eq!(k.transmission_time, None);
        assert_eq!(
            f.as_ephemeris().unwrap().get_orbit_f64("t_tm"),
            Some(if version >= Version::new(4, 2) {
                999999999.999
            } else {
                999900000.0
            })
        );
        source = back;
        source.header.version = Version::new(3, 4);
        assert_eq!(
            write_and_reparse(&source, "back").unwrap().record,
            rinex.record
        );
    }
}

#[test]
fn independent_raw_grouping_preserves_same_epoch_ephemeris_sto_eop_transmissions() {
    // BRD400DLR 2024-131 source lines: one STO, EOP and FDMA EPH.
    let text = crate::tests::parsing::navigation_excerpt(
        "BRD400DLR_S_20241310000_01D_MN.rnx",
        &[(1, 9), (22, 24), (415, 418), (62862, 62867)],
    );
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    let header = lines[..9].concat();
    let body = lines[9..].concat();
    // Synthetic retransmissions with the same reference epochs.
    let later = body
        .replace("4.322400000000e+05", "4.323000000000e+05")
        .replace("4.410000000000e+05", "4.410600000000e+05");
    // Change STO's first scalar independently of the epoch/pair columns.
    let mut rows: Vec<_> = later.lines().map(str::to_string).collect();
    rows[2].replace_range(4..23, " 1.234560000000E+05");
    let source = format!("{header}{body}{}\n", rows.join("\n"));
    let model = v4_record_blocks(&source);
    assert_eq!(model.len(), 6);
    let r = Rinex::parse(&mut std::io::BufReader::new(source.as_bytes())).unwrap();
    assert_eq!(r.record.as_nav().unwrap().len(), 6);
    let mut buffer = std::io::BufWriter::new(Vec::new());
    r.format(&mut buffer).unwrap();
    let text = String::from_utf8(buffer.into_inner().unwrap()).unwrap();
    let output = v4_record_blocks(&text);
    assert_eq!(output.len(), 6);
    for (key, block) in output {
        let expected = model.get(&key).unwrap();
        // FDMA may format trailing blank fields to full width; retained raw
        // identity and three clock scalars still agree independently.
        if key.starts_with("> EPH") {
            assert_eq!(block[0], expected[0]);
        } else {
            assert_eq!(&block, expected);
        }
    }
}
