//! Complete regression of the three unmodified 2024-131 broadcast products.
//! Inputs are read only; formatting verification uses an in-memory buffer.

use crate::{prelude::*, record::ParsingDiagnosticKind};
use std::io::{BufReader, BufWriter};

#[test]
fn audit_three_original_broadcast_files() {
    // Counts obtained independently from raw record boundaries and the RINEX
    // identity columns, rather than from Rinex::parse output.
    // (previous/current body lines, SV, transmission seconds), checked from
    // raw columns. GLONASS differences: health/status; E14: BGD; C04:
    // e/sqrtA/Cic; J02: ISC. STO GLGP/GLUT differ in A0. Source header
    // lengths are 93 (IGS), 9 (BRD4) and 26 (BRDM) lines.
    const BRD4: &[(usize, usize, &str, f64)] = &[
        (310, 313, "R00", 507766.0),
        (313, 319, "R00", 507766.0),
        (328, 331, "R00", 507766.0),
        (62547, 62553, "R11", 450000.0),
        (62559, 62565, "R11", 451800.0),
        (62571, 62577, "R11", 453600.0),
        (62589, 62595, "R11", 455400.0),
        (89535, 89544, "E14", 495815.0),
        (113934, 113943, "C04", 482370.0),
        (149211, 149221, "J02", 468006.0),
    ];
    const BRDM: &[(usize, usize, &str, f64)] = &[
        (46341, 46345, "R11", 450000.0),
        (46349, 46353, "R11", 451800.0),
        (46357, 46361, "R11", 453600.0),
        (46369, 46373, "R11", 455400.0),
        (69309, 69317, "E14", 495815.0),
        (90997, 91005, "C04", 482370.0),
    ];
    for (name, raw_count, unique_count, duplicates) in [
        ("BRDC00IGS_R_20241310000_01D_MN.rnx", 26031, 26031, &[][..]),
        ("BRD400DLR_S_20241310000_01D_MN.rnx", 21687, 21677, BRD4),
        ("BRDM00DLR_S_20241310000_01D_MN.rnx", 18418, 18412, BRDM),
    ] {
        let version = if name.starts_with("BRD4") { "V4" } else { "V3" };
        let source = super::navigation_text(&format!("data/NAV/{version}/{name}.gz"));
        let body: Vec<_> = source
            .lines()
            .skip_while(|line| !line.contains("END OF HEADER"))
            .skip(1)
            .collect();
        let is_v4 = source.starts_with("     4.");
        let starts: Vec<_> = body
            .iter()
            .enumerate()
            .filter_map(|(index, line)| {
                let boundary = if is_v4 {
                    line.starts_with('>')
                } else {
                    line.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                };
                boundary.then_some(index)
            })
            .collect();
        assert_eq!(starts.len(), raw_count, "{name}: raw record count");

        let (rinex, diagnostics) =
            Rinex::parse_with_diagnostics(&mut BufReader::new(source.as_bytes())).unwrap();
        let records = rinex.record.as_nav().unwrap();
        assert_eq!(records.len(), unique_count, "{name}: distinct identities");
        assert_eq!(
            diagnostics.len(),
            duplicates.len(),
            "{name}: {diagnostics:?}"
        );
        for (diagnostic, &(previous, current, sv, seconds)) in diagnostics.iter().zip(duplicates) {
            let ParsingDiagnosticKind::DuplicateNavigation {
                key,
                conflicting: true,
                previous_record_line,
            } = &diagnostic.kind
            else {
                panic!("{name}: unexpected diagnostic {diagnostic:?}");
            };
            assert_eq!(
                (*previous_record_line, diagnostic.record_line),
                (previous, current)
            );
            assert_eq!(key.sv.to_string(), sv);
            assert_eq!(key.transmission_time.unwrap().seconds(), seconds);
            if sv == "E14" {
                assert_eq!(key.galileo_data_sources, Some(517));
            }
            if sv == "R00" {
                let identity = key.sto_identity.unwrap();
                assert_eq!(
                    &identity.time_system,
                    if previous == 328 { b"GLUT" } else { b"GLGP" }
                );
            }
            assert!(records.contains_key(key));
            assert!(starts.contains(&(previous_record_line - 1)));
            assert!(starts.contains(&(diagnostic.record_line - 1)));
            assert!(previous_record_line < &diagnostic.record_line);
        }
        // Every raw block must be accounted for; no recoverable parse omissions.
        assert_eq!(records.len() + diagnostics.len(), raw_count);

        let ordinary = Rinex::parse(&mut BufReader::new(source.as_bytes()));
        assert_eq!(ordinary.unwrap().record, rinex.record);

        // The retained last-record snapshot must write and read without omissions.
        let mut writer = BufWriter::new(Vec::new());
        rinex.format(&mut writer).unwrap();
        let formatted = writer.into_inner().unwrap();
        let (back, after) =
            Rinex::parse_with_diagnostics(&mut BufReader::new(formatted.as_slice())).unwrap();
        assert!(after.is_empty(), "{name}: write/read diagnostics {after:?}");
        assert_eq!(back.record, rinex.record, "{name}: write/read snapshot");
        println!(
            "{name}: raw={raw_count}, retained={}, conflicts={}, write/read=equal",
            records.len(),
            diagnostics.len()
        );
    }
}
