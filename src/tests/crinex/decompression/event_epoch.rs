// Event epochs (flag 2 to 5) of a V3 stream: their number of satellites is the
// number of special records that follow, which are published as they are.
use std::{collections::HashMap, str::from_utf8};

use crate::{
    hatanaka::Decompressor,
    prelude::{Constellation, Observable},
};

use std::str::FromStr;

fn observables() -> HashMap<Constellation, Vec<Observable>> {
    [Constellation::GPS, Constellation::Glonass]
        .into_iter()
        .map(|c| (c, vec![Observable::from_str("C1C").unwrap()]))
        .collect()
}

fn decompress(lines: &[String]) -> Vec<String> {
    let mut decomp = Decompressor::new(true, Constellation::Mixed, observables());

    let mut buf = [0; 1024];
    let mut output = Vec::new();

    for line in lines {
        let size = decomp
            .decompress(line, line.len(), &mut buf, 1024)
            .unwrap_or_else(|e| panic!("failed to decompress \"{}\": {}", line, e));

        if size > 0 {
            output.push(from_utf8(&buf[..size]).unwrap().to_string());
        }
    }

    output
}

const FIRST_EPOCH: [&str; 3] = [
    "> 2020 06 25 00 00 00.0000000  0  1      G01",
    "",
    "3&24413184520",
];

const HEADER_RECORDS: [&str; 2] = [
    "NEW SITE                                                    MARKER NAME",
    "SPECIAL RECORD ENDING WITH THE KEYWORD                      COMMENT",
];

fn stream(event: &str, next_epoch: [&str; 3]) -> Vec<String> {
    FIRST_EPOCH
        .iter()
        .chain([event].iter())
        .chain(HEADER_RECORDS.iter())
        .chain(next_epoch.iter())
        .map(|line| line.to_string())
        .collect()
}

// An event epoch is published in full, like any epoch that starts with the marker:
// the timestamp may be blank (like in the reported file).
const EVENT: &str = ">                              4  2";

#[test]
fn event_epoch_then_complete_epoch() {
    let output = decompress(&stream(
        EVENT,
        [
            "> 2020 06 25 00 00 30.0000000  0  1      G01",
            "",
            "3&24413184999",
        ],
    ));

    assert_eq!(
        output,
        [
            "> 2020 06 25 00 00 00.0000000  0  1",
            "G01  24413184.520  ",
            EVENT,
            HEADER_RECORDS[0],
            HEADER_RECORDS[1],
            "> 2020 06 25 00 00 30.0000000  0  1",
            "G01  24413184.999  ",
        ]
    );
}

#[test]
fn event_epoch_with_timestamp() {
    let event = "> 2020 06 25 00 00 15.0000000  4  2";

    let output = decompress(&stream(
        event,
        [
            "> 2020 06 25 00 00 30.0000000  0  1      G01",
            "",
            "3&24413184999",
        ],
    ));

    assert_eq!(output.len(), 7);
    assert_eq!(output[2], event);
    assert_eq!(output[5], "> 2020 06 25 00 00 30.0000000  0  1");
    assert_eq!(output[6], "G01  24413184.999  ");
}

// V1: stream and expected output produced by the historical RNX2CRX and CRX2RNX tools,
// the event epoch is published as it is.
#[test]
fn v1_event_epoch() {
    let input = [
        "&21  1  1  0  0  0.0000000  0  1G01",
        "",
        "3&20000000123",
        "&21  1  1  0  0 15.0000000  4  2",
        "MARKER CHANGE                                               COMMENT",
        "SECOND RECORD                                               MARKER NAME",
        "&21  1  1  0  0 30.0000000  0  1G01",
        "",
        "3&20000100456",
        "                6",
        "",
        "100333",
    ];

    let expected = [
        " 21  1  1  0  0  0.0000000  0  1G01",
        "  20000000.123",
        " 21  1  1  0  0 15.0000000  4  2",
        "MARKER CHANGE                                               COMMENT",
        "SECOND RECORD                                               MARKER NAME",
        " 21  1  1  0  0 30.0000000  0  1G01",
        "  20000100.456",
        " 21  1  1  0  0 60.0000000  0  1G01",
        "  20000200.789",
    ];

    let mut observables = HashMap::new();
    observables.insert(
        Constellation::GPS,
        vec![Observable::from_str("C1").unwrap()],
    );

    let mut decomp = Decompressor::new(false, Constellation::GPS, observables);

    let mut buf = [0; 1024];
    let mut output = Vec::new();

    for line in input {
        let size = decomp.decompress(line, line.len(), &mut buf, 1024).unwrap();
        if size > 0 {
            for line in from_utf8(&buf[..size]).unwrap().lines() {
                output.push(line.trim_end().to_string());
            }
        }
    }

    assert_eq!(output, expected);
}

fn header_line(content: &str, label: &str) -> String {
    format!("{:<60}{}\n", content, label)
}

// Special records are not interpreted, even those that look like comments:
// the file is parsed to the end and the epochs around the event are recovered.
#[test]
fn event_epoch_in_file() {
    use crate::prelude::Rinex;
    use std::io::BufReader;

    let mut content = String::new();
    content.push_str(&header_line(
        "3.0                 COMPACT RINEX FORMAT",
        "CRINEX VERS   / TYPE",
    ));
    content.push_str(&header_line(
        "RNX2CRX ver.4.1.0                       31-Jan-25 09:24",
        "CRINEX PROG / DATE",
    ));
    content.push_str(&header_line(
        "     3.02           OBSERVATION DATA    G: GPS",
        "RINEX VERSION / TYPE",
    ));
    content.push_str(&header_line("G    1 C1C", "SYS / # / OBS TYPES"));
    content.push_str(&header_line(
        "  2020     6    25     0     0    0.0000000     GPS",
        "TIME OF FIRST OBS",
    ));
    content.push_str(&header_line("", "END OF HEADER"));

    for line in stream(
        EVENT,
        [
            "> 2020 06 25 00 00 30.0000000  0  1      G01",
            "",
            "3&24413184999",
        ],
    ) {
        content.push_str(&line);
        content.push('\n');
    }

    let rinex = Rinex::parse(&mut BufReader::new(content.as_bytes())).unwrap();

    let epochs = rinex.epoch_iter().collect::<Vec<_>>();
    assert_eq!(epochs.len(), 2, "both epochs around the event are expected");
}

// Issue 356: HKWS00HKG_R_20250300000_01D_30S_MO, an epoch of 46 satellites is
// followed by an event epoch (flag 4) announcing 213 special records, published
// as the line ">" + 30 blanks + "4213".
const REPORTED_EVENT: &str = ">                              4213";

fn reported_stream() -> Vec<String> {
    let satellites = (1..=32)
        .map(|prn| format!("G{:02}", prn))
        .chain((1..=14).map(|prn| format!("R{:02}", prn)))
        .collect::<Vec<_>>();

    assert_eq!(satellites.len(), 46);

    let mut lines = vec![
        format!(
            "> 2025 01 30 00 00  0.0000000  0 46      {}",
            satellites.join("")
        ),
        String::new(),
    ];

    for (nth, _) in satellites.iter().enumerate() {
        lines.push(format!("3&2441318{:04}", nth));
    }

    lines.push(REPORTED_EVENT.to_string());

    for nth in 0..213 {
        // some of them look like comments
        let label = if nth % 2 == 0 {
            "COMMENT"
        } else {
            "PRN / # OF OBS"
        };
        lines.push(format!(
            "{:<60}{}",
            format!("  {:>4}  {:>4}", nth, 2880),
            label
        ));
    }

    // the stream goes on
    lines.push("> 2025 01 30 00 00 30.0000000  0 46      ".to_string() + &satellites.join(""));
    lines.push(String::new());
    for nth in 0..satellites.len() {
        lines.push(format!("3&2441319{:04}", nth));
    }

    lines
}

#[test]
fn reported_event_epoch() {
    let input = reported_stream();
    let output = decompress(&input);

    // 1 epoch + 46 satellites, the event and its 213 records, 1 epoch + 46 satellites
    assert_eq!(output.len(), 47 + 1 + 213 + 47);

    assert_eq!(output[47], REPORTED_EVENT);
    assert_eq!(output[48..48 + 213], input[49..49 + 213]);
    assert_eq!(output[48 + 213], "> 2025 01 30 00 00 30.0000000  0 46");
}

#[test]
fn reported_event_epoch_in_file() {
    use crate::prelude::Rinex;
    use std::io::BufReader;

    let mut content = String::new();
    content.push_str(&header_line(
        "3.0                 COMPACT RINEX FORMAT",
        "CRINEX VERS   / TYPE",
    ));
    content.push_str(&header_line(
        "RNX2CRX ver.4.1.0                       31-Jan-25 09:24",
        "CRINEX PROG / DATE",
    ));
    content.push_str(&header_line(
        "     3.02           OBSERVATION DATA    M: MIXED",
        "RINEX VERSION / TYPE",
    ));
    content.push_str(&header_line("G    1 C1C", "SYS / # / OBS TYPES"));
    content.push_str(&header_line("R    1 C1C", "SYS / # / OBS TYPES"));
    content.push_str(&header_line(
        "  2025     1    30     0     0    0.0000000     GPS",
        "TIME OF FIRST OBS",
    ));
    content.push_str(&header_line("", "END OF HEADER"));

    for line in reported_stream() {
        content.push_str(&line);
        content.push('\n');
    }

    let rinex = Rinex::parse(&mut BufReader::new(content.as_bytes())).unwrap();

    // both epochs around the event are complete: the special records
    // that look like comments did not disturb the decompressor
    let signals = rinex
        .observations_iter()
        .map(|(_, v)| v.signals.len())
        .collect::<Vec<_>>();

    assert_eq!(signals, [46, 46]);
}
