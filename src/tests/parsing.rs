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

        for data in vec![
            "OBS", //"CRNX",
            "MET", "NAV", "CLK", "ATX",
        ] {
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
                                            assert!(ts == timescale,
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
