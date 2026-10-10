//! Parser regressions with small inline records and synthetic file envelopes.
mod broadcast;
mod diagnostics;
mod dlr_identity;
mod galileo_identity;
mod navigation;
mod observation;

use crate::{prelude::*, record::ParsingDiagnostic};
use std::io::BufReader;

// Source G03/G04 rows (lines 51/493) are unchanged; each epoch now declares one SV.
// SEPT00ATA_R_20241310000_01H_01S_MO.rnx, SHA-256:
// 79ce9f794590eb38690e61097f8161abee9334f094282444c956b7a5faf15a98.
const OBS_BODY: &str = r#"> 2024 05 10 00 00  0.0000000  0  1
G03  20888140.504 8 109767926.63108  20888140.402 8  20888140.763 8  85533467.98008  20888141.418 7  85533463.98007  20888140.080 8  81969573.12908
> 2024 05 10 00 00  8.0000000  0  1
G04  22049901.763 7 115873014.86117                                                                                  22049903.466 7  86528556.98917  22049901.762 7 115873015.86117
"#;

// First G03 EPH, source lines 14-22 in SEPT00ATA_R_20241310000_01H_MN.rnx.
// SHA-256: 34db1cd071f106bbf64af594025478a7ffd156669f3a6f9ed0b37f9f00cc152d.
pub(super) const GPS_BLOCK: &str = r#"> EPH G03 LNAV
G03 2024 05 10 00 00 00 3.551370464265E-04 1.989519660128E-11 0.000000000000E+00
     1.100000000000E+02-1.118750000000E+02 3.804801342391E-09-2.502756304520E+00
    -5.733221769333E-06 5.448369774967E-03 1.007691025734E-05 5.153735258102E+03
     4.320000000000E+05 1.229345798492E-07-2.837205550508E+00-1.490116119385E-08
     9.845089665102E-01 1.977500000000E+02 1.076704079767E+00-7.452453281735E-09
     2.517962026082E-10 1.000000000000E+00 2.313000000000E+03 0.000000000000E+00
     2.000000000000E+00 0.000000000000E+00 1.396983861923E-09 1.100000000000E+02
     4.248180000000E+05 4.000000000000E+00
"#;

// DLR BRD400DLR_S_20241310000_01D_MN.rnx (DOI: 10.57677/BRD400DLR).
// SHA-256: 9cffb1b1f2978c46e8352d78f603dd089ecd7f32312c1a6c4ae585b5d0e538b6.
// First R12 FDMA record, source lines 62862-62867.
const GLO_BLOCK: &str = r#"> EPH R12 FDMA
R12 2024 05 10 02 45 00 3.995746374130e-05 0.000000000000e+00 4.410000000000e+05
    -1.784736474609e+04 1.332044601440e+00-9.313225746155e-10 0.000000000000e+00
     1.114703759766e+04-1.464355468750e+00 0.000000000000e+00-1.000000000000e+00
    -1.440109765625e+04-2.778277397156e+00 3.725290298462e-09 0.000000000000e+00
     2.470000000000e+02 7.450580596924e-09 1.000000000000e+00 3.000000000000e+00
"#;

// First S27 SBAS record in the same DLR source, lines 20116-20120.
const SBAS_BLOCK: &str = r#"> EPH S27 SBAS
S27 2024 05 10 02 56 00 5.569308996201e-07 8.367351256311e-11 4.425350000000e+05
     2.410735592000e+04-1.813750000000e-03 1.500000000000e-07 0.000000000000e+00
     3.457259992000e+04 1.931250000000e-03 1.250000000000e-08 4.096000000000e+03
    -7.398280000000e+01-3.300000000000e-03 3.750000000000e-07 1.480000000000e+02
"#;

pub(super) fn navigation_text(path: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let file = std::fs::File::open(&path)
        .unwrap_or_else(|error| panic!("NAV input {}: {error}", path.display()));
    let mut text = String::new();
    if path.extension().is_some_and(|ext| ext == "gz") {
        std::io::Read::read_to_string(&mut flate2::read::GzDecoder::new(file), &mut text).unwrap();
    } else {
        std::io::Read::read_to_string(&mut BufReader::new(file), &mut text).unwrap();
    }
    text
}

pub(super) fn navigation_excerpt(name: &str, spans: &[(usize, usize)]) -> String {
    let version = if name.starts_with("BRD4") { "V4" } else { "V3" };
    let text = navigation_text(&format!("data/NAV/{version}/{name}.gz"));
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    spans
        .iter()
        .map(|&(first, last)| lines[first - 1..last].concat())
        .collect()
}

fn parse(text: &str) -> Result<Rinex, ParsingError> {
    Rinex::parse(&mut BufReader::new(text.as_bytes()))
}

fn parse_diagnostics(text: &str) -> (Rinex, Vec<ParsingDiagnostic>) {
    Rinex::parse_with_diagnostics(&mut BufReader::new(text.as_bytes())).unwrap()
}

fn observation_rinex(version: &str, codes: &str, body: &str) -> String {
    // Synthetic envelope; body values retain their supplied column positions.
    format!(
        "{version:>9}           OBSERVATION DATA    M                   RINEX VERSION / TYPE\n\
         {:<60}SYS / # / OBS TYPES\n\
         {:<60}TIME OF FIRST OBS\n\
         {:<60}END OF HEADER\n{body}",
        codes, "  2024     5    10     0     0    0.0000000     GPS", ""
    )
}

fn source_observations() -> String {
    observation_rinex(
        "4.02",
        "G   11 C1C L1C C1W C2W L2W C2L L2L C5Q L5Q C1L L1L",
        OBS_BODY,
    )
}

fn synthetic_obs(version: &str, body: &str) -> String {
    observation_rinex(version, "G    2 L1C C1C", body)
}

pub(super) fn navigation_rinex(version: &str, body: &str) -> String {
    format!(
        "{version:>9}           NAVIGATION DATA     M                   RINEX VERSION / TYPE\n\
         {:<60}END OF HEADER\n{body}",
        ""
    )
}

fn gps_first_block() -> (String, &'static str) {
    (navigation_rinex("4.02", ""), GPS_BLOCK)
}

fn epoch(seconds: &str, clock: Option<&str>, extension: Option<&str>) -> String {
    let base = format!("> 2024 05 10 00 00 {seconds:>10}  0  1");
    let first = if clock.is_some() || extension.is_some() {
        format!(
            "{base:<41}{:>15} {}",
            clock.unwrap_or(""),
            extension.unwrap_or("")
        )
    } else {
        base
    };
    format!("{first}\nG03{:>14}17{:>14} 8\n", "-12.500", "0.000")
}

#[cfg(test)]
#[cfg(feature = "flate2")]
mod test {
    use crate::prelude::*;
    use std::path::PathBuf;

    #[test]
    fn repo_parsing() {
        // Tests entire repository with at least successful parsing
        // and runs a few verifications.
        // For thorough verifications, we have dedicated tests elsewhere.
        let test_resources = PathBuf::new().join(env!("CARGO_MANIFEST_DIR")).join("data");

        for data in vec!["OBS", "CRNX", "MET", "NAV", "CLK", "ATX"] {
            let data_path = test_resources.clone().join(data);
            for revision in std::fs::read_dir(data_path).unwrap() {
                let rev = revision.unwrap();
                let rev_path = rev.path();
                let rev_fullpath = &rev_path.to_str().unwrap();
                for entry in std::fs::read_dir(rev_fullpath).unwrap() {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    let full_path = &path.to_str().unwrap();

                    let filename = entry.file_name().to_str().unwrap().to_string();

                    // discard hidden files
                    if filename.starts_with('.') {
                        continue;
                    }

                    //let is_generated_file = entry.file_name().to_str().unwrap().ends_with("-copy");
                    //if is_generated_file {
                    //    continue; // not a test resource
                    //}

                    // discard .Z compression that we cannot support
                    if filename.ends_with(".Z") {
                        continue;
                    }

                    // parse RINEX file
                    println!("Parsing \"{}\"", full_path);

                    let rinex = if filename.ends_with(".gz") {
                        #[cfg(feature = "flate2")]
                        Rinex::from_gzip_file(full_path)
                    } else {
                        Rinex::from_file(full_path)
                    };

                    assert!(
                        rinex.is_ok(),
                        "error parsing \"{}\": {:?}",
                        full_path,
                        rinex.err().unwrap()
                    );
                    let rinex = rinex.unwrap();

                    match data {
                        "ATX" => {
                            assert!(rinex.is_antex());
                        },
                        "NAV" => {
                            assert!(rinex.is_navigation_rinex());
                            assert!(rinex.epoch_iter().count() > 0); // all files have content
                            assert!(rinex.navigation_keys().count() > 0); // all files have content
                            assert!(rinex.nav_ephemeris_frames_iter().count() > 0);
                            // all files have content
                        },
                        "CRNX" | "OBS" => {
                            assert!(rinex.header.obs.is_some());
                            let obs_header = rinex.header.obs.clone().unwrap();

                            assert!(rinex.is_observation_rinex());
                            assert!(rinex.epoch_iter().count() > 0); // all files have content
                            assert!(rinex.signal_observations_iter().count() > 0); // all files have content

                            if data == "OBS" {
                                let compressed = rinex.rnx2crnx();
                                assert!(
                                    compressed.header.is_crinex(),
                                    "is_crinex() should always be true for compressed rinex!"
                                );
                            } else if data == "CRNX" {
                                let decompressed = rinex.crnx2rnx();
                                assert!(
                                    !decompressed.header.is_crinex(),
                                    "is_crinex() should always be false for readable rinex!"
                                );
                            }

                            /* Timescale validity */
                            for k in rinex.observation_keys() {
                                let ts = k.epoch.time_scale;
                                if let Some(e0) = obs_header.timeof_first_obs {
                                    assert!(
                                        e0.time_scale == ts,
                                        "interpreted wrong timescale: expecting \"{}\", got \"{}\"",
                                        e0.time_scale,
                                        ts
                                    );
                                } else {
                                    match rinex.header.constellation {
                                        Some(Constellation::Mixed) | None => {}, // can't test
                                        Some(c) => {
                                            let timescale = c.timescale().unwrap();
                                            assert!(
                                                ts == timescale,
                                                "interpreted wrong timescale: expecting \"{}\", got \"{}\"",
                                                timescale,
                                                ts
                                            );
                                        },
                                    }
                                }
                            }
                        },
                        "MET" => {
                            assert!(rinex.is_meteo_rinex());
                            assert!(rinex.epoch_iter().count() > 0); // all files have content
                            assert!(rinex.meteo_observation_keys().count() > 0); // all files have content
                            assert!(rinex.meteo_observations_iter().count() > 0); // all files have content

                            for (k, _) in rinex.meteo_observations_iter() {
                                assert!(
                                    k.epoch.time_scale == TimeScale::UTC,
                                    "wrong {} time scale for a METEO RINEX",
                                    k.epoch.time_scale
                                );
                            }
                        },
                        "CLK" => {
                            assert!(rinex.is_clock_rinex(), "badly identified CLK RINEX");
                            assert!(rinex.header.clock.is_some(), "badly formed CLK RINEX");
                            assert!(rinex.epoch_iter().count() > 0); // all files have content
                            let _ = rinex.record.as_clock().unwrap();
                        },
                        _ => unreachable!(),
                    }
                }
            }
        }
    }
}
