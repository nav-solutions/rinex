//! Parser regressions with small inline records and synthetic file envelopes.
mod observation;

use crate::prelude::*;
use std::io::BufReader;

// Source G03/G04 rows (lines 51/493) are unchanged; each epoch now declares one SV.
// SEPT00ATA_R_20241310000_01H_01S_MO.rnx, SHA-256:
// 79ce9f794590eb38690e61097f8161abee9334f094282444c956b7a5faf15a98.
const OBS_BODY: &str = r#"> 2024 05 10 00 00  0.0000000  0  1
G03  20888140.504 8 109767926.63108  20888140.402 8  20888140.763 8  85533467.98008  20888141.418 7  85533463.98007  20888140.080 8  81969573.12908
> 2024 05 10 00 00  8.0000000  0  1
G04  22049901.763 7 115873014.86117                                                                                  22049903.466 7  86528556.98917  22049901.762 7 115873015.86117
"#;

fn parse(text: &str) -> Result<Rinex, ParsingError> {
    Rinex::parse(&mut BufReader::new(text.as_bytes()))
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
