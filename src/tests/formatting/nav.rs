//! Navigation RINEX formatting, compared to the original files.
use crate::{navigation::NavFrameType, prelude::Rinex};

use std::{collections::HashMap, fs::read_to_string, fs::remove_file};

/// Splits a RINEX 4 navigation record section into blocks, one per
/// "> XXX" record, keyed by the record header, the epoch and, for STO
/// records, the time system pair. Exponent letters are uppercased and
/// trailing blanks trimmed on every line.
fn v4_record_blocks(content: &str) -> HashMap<String, Vec<String>> {
    let mut blocks = HashMap::<String, Vec<String>>::new();
    let mut header = Option::<String>::None;
    let mut current = Option::<String>::None;

    for line in content
        .lines()
        .skip_while(|line| !line.contains("END OF HEADER"))
        .skip(1)
    {
        let line = line.trim_end().replace('e', "E");
        if line.starts_with("> ") {
            header = Some(line.clone());
            current = None;
            continue;
        }
        if let Some(header) = header.take() {
            let key = if header.starts_with("> STO") {
                format!("{}\n{}", header, &line[..28.min(line.len())])
            } else {
                format!("{}\n{}", header, &line[..23.min(line.len())])
            };
            blocks.insert(key.clone(), Vec::new());
            current = Some(key);
        }
        if let Some(key) = &current {
            if let Some(block) = blocks.get_mut(key) {
                block.push(line);
            }
        }
    }

    blocks
}

/// Every STO, EOP and ION record written from `name` must be equal to
/// the original text, and every such record of the parsed file must be
/// written. Records sharing epoch, vehicle and message type overwrite
/// each other in the record, so the original file may hold more.
fn v4_records_write_back(name: &str) {
    let path = format!("data/NAV/V4/{}", name);
    let original = if name.ends_with(".gz") {
        Rinex::from_gzip_file(&path).unwrap()
    } else {
        Rinex::from_file(&path).unwrap()
    };

    let tmp = format!("test-{}.rnx", name);
    original.to_file(&tmp).unwrap();
    let written = read_to_string(&tmp).unwrap();
    let _ = remove_file(&tmp);

    let original_text = if name.ends_with(".gz") {
        let mut reader = flate2::read::GzDecoder::new(std::fs::File::open(&path).unwrap());
        let mut content = String::new();
        std::io::Read::read_to_string(&mut reader, &mut content).unwrap();
        content
    } else {
        read_to_string(&path).unwrap()
    };

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

        let (block, written) = if key.starts_with("> STO") {
            // the message transmission time is not stored: the first
            // field of the second line is not compared
            (
                vec![block[0].clone(), block[1][23..].to_string()],
                vec![written[0].clone(), written[1][23..].to_string()],
            )
        } else if key.starts_with("> ION") && block.len() == 3 && block[2].len() == 23 {
            // Klobuchar: a blank region code is read as worldwide (0)
            // and written as such
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
    let tmp = format!("test-{}-{}.rnx", tag, std::process::id());
    rinex.to_file(&tmp)?;
    let parsed = Rinex::from_file(&tmp).unwrap();
    let _ = remove_file(&tmp);
    Ok(parsed)
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
        let mut rinex = Rinex::from_gzip_file(&format!("data/NAV/V4/{}", name)).unwrap();
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

        // records sharing epoch and vehicle collapse to one RINEX 3 key
        let expected = legacy
            .iter()
            .map(|(k, _)| (k.epoch, k.sv))
            .collect::<std::collections::BTreeSet<_>>()
            .len();

        rinex.header.version = Version::new(3, 5);
        let parsed = write_and_reparse(&rinex, "v4-as-v3").unwrap();
        assert_eq!(parsed.header.version, Version::new(3, 5));

        let record = parsed.record.as_nav().unwrap();
        assert_eq!(record.len(), expected, "{}", name);

        for (k, frame) in record.iter() {
            assert_eq!(k.msgtype, NavMessageType::LNAV);
            // the last legacy message of this epoch and vehicle was written
            let (_, original) = legacy
                .iter()
                .filter(|(o, _)| o.epoch == k.epoch && o.sv == k.sv)
                .last()
                .unwrap_or_else(|| panic!("{:?} was not in the original file", k));
            assert_ephemeris_preserved(original, frame, k);
        }
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
            msgtype: NavMessageType::LNAV,
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
