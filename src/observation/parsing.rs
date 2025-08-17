//! Observation RINEX parsing
use crate::{
    epoch::{parse_in_timescale as parse_epoch_in_timescale, parse_utc as parse_utc_epoch},
    observation::{
        ClockObservation, EpochFlag, LliFlags, ObsKey, Observations, SignalObservation, SNR,
    },
    prelude::{Constellation, Header, Observable, ParsingError, TimeScale, Version, SV},
};

use std::{
    collections::HashMap,
    str::{FromStr, Lines},
};

// TODO: see if we can get rid of num_integer
use num_integer::div_ceil;

// #[cfg(feature = "log")]
// use log::{debug, error};

/// Returns true if provided content matches the start of a new Observation Epoch
pub fn is_new_epoch(line: &str, v: Version) -> bool {
    if v.major < 3 {
        if line.len() < 30 {
            false
        } else {
            // SPLICE flag handling (still an Observation::flag)
            let significant = !line[0..26].trim().is_empty();
            let epoch = parse_utc_epoch(&line[0..26]);
            let flag = EpochFlag::from_str(line[26..29].trim());
            if significant {
                epoch.is_ok() && flag.is_ok()
            } else if flag.is_err() {
                false
            } else {
                match flag.unwrap() {
                    EpochFlag::AntennaBeingMoved
                    | EpochFlag::NewSiteOccupation
                    | EpochFlag::HeaderInformationFollows
                    | EpochFlag::ExternalEvent => true,
                    _ => false,
                }
            }
        }
    } else {
        // Modern RINEX has a simple marker, like all V4 modern files
        match line.chars().next() {
            Some(c) => {
                c == '>' // epochs always delimited
                         // by this new identifier
            },
            _ => false,
        }
    }
}

/// Parses record entries from readable content
/// ## Input
///   - header: reference to previously parsed [Header]
///   - content: readable content
///   - ts: [TimeScale] defined by [Header]
///   - observations: preallocated [Observations] for performance issue.
/// ## Output
///   - [ObsKey] record indexer
pub fn parse_epoch(
    header: &Header,
    content: &str,
    ts: TimeScale,
    observations: &mut Observations,
) -> Result<ObsKey, ParsingError> {
    let mut lines = content.lines();

    let mut line = match lines.next() {
        Some(l) => l,
        _ => return Err(ParsingError::EmptyEpoch),
    };

    // epoch::
    let mut offset: usize = 2+1 // Y
        +2+1 // d
        +2+1 // m
        +2+1 // h
        +2+1 // m
        +11; // secs

    // V > 2 epoch::year is a 4 digit number
    if header.version.major > 2 {
        offset += 2
    }

    // V > 2 might start with a ">" marker
    if line.starts_with('>') {
        line = line.split_at(1).1;
    }

    let (date, rem) = line.split_at(offset);
    let epoch = parse_epoch_in_timescale(date, ts)?;

    let (flag, rem) = rem.split_at(3);
    let flag = EpochFlag::from_str(flag.trim())?;

    let key = ObsKey { epoch, flag };

    let (num_sat, rem) = rem.split_at(3);
    let num_sat = num_sat
        .trim()
        .parse::<u16>()
        .map_err(|_| ParsingError::NumSatParsing)?;

    // grab possible clock offset
    let offs: Option<&str> = match header.version.major < 2 {
        true => {
            // RINEX 2
            // clock offsets are last 12 characters
            if line.len() > 60 - 12 {
                Some(line.split_at(60 - 12).1.trim())
            } else {
                None
            }
        },
        false => {
            // RINEX 3
            let min_len: usize = 4+1 // y
                +2+1 // m
                +2+1 // d
                +2+1 // h
                +2+1 // m
                +11+1// s
                +3   // flag
                +3; // n_sat
            if line.len() > min_len {
                // RINEX3: clock offset precision was increased
                Some(line.split_at(min_len).1.trim()) // this handles it naturally
            } else {
                None
            }
        },
    };

    if let Some(offset) = offs {
        if let Ok(offset_s) = offset.parse::<f64>() {
            observations
                .with_clock_observation(ClockObservation::default().with_offset_s(epoch, offset_s));
        }
    }

    match flag {
        EpochFlag::Ok | EpochFlag::PowerFailure | EpochFlag::CycleSlip => {
            parse_observations(header, num_sat, rem, lines, &mut observations.signals)?;
        },
        _ => {
            // Hardware events are not supported yet (coming soon)
            return Err(ParsingError::ObsHardwareEvent);
        },
    }

    Ok(key)
}

fn parse_observations(
    header: &Header,
    num_sat: u16,
    rem: &str,
    mut lines: Lines<'_>,
    signals: &mut Vec<SignalObservation>,
) -> Result<(), ParsingError> {
    // retrieve header specs
    let constellation = header.constellation;

    let obs = header.obs.as_ref().unwrap();
    let observables = &obs.codes;

    // V1 / V2 tedious case
    let rem = rem.trim();
    let remainder_len = rem.len();

    if header.version.major < 3 {
        // Sets the satellite systems description, which consits in
        //  - end of current line
        //  - possible following lines

        // starts with first line content
        let end = (12 * 3).min(remainder_len);
        let mut systems_str = rem[..end].to_string();
        let mut systems_str_len = systems_str.len();

        // grab following lines (if we need to)
        while systems_str_len / 3 < num_sat.into() {
            if let Some(line) = lines.next() {
                let trimmed = line.trim();
                systems_str_len += trimmed.len();
                systems_str.push_str(trimmed);
            } else {
                return Err(ParsingError::BadV2SatellitesDescription);
            }
        }

        let systems_str_len = systems_str.len();

        parse_signals_v2(
            &systems_str,
            systems_str_len,
            constellation,
            observables,
            lines,
            signals,
        );
    } else {
        parse_signals_v3(observables, lines, signals);
    }

    Ok(())
}

/// Parses all [SignalObservation]s as described by following V2 content.
/// Old format is tedious:
///   - vehicle description is contained in first line
///   - wrapped in as many lines as needed
/// Inputs
///   - system_str: first line description
///   - constellation: [Constellation] specs defined in [Header]
///   - observables: reference to [Observable]s specs defined in [Header]
///   - lines: remaing [Lines] Iterator
fn parse_signals_v2(
    systems_str: &str,
    systems_str_len: usize,
    head_constellation: Option<Constellation>,
    head_observables: &HashMap<Constellation, Vec<Observable>>,
    lines: Lines<'_>,
    signals: &mut Vec<SignalObservation>,
) {
    const SVNN_SIZE: usize = 3; // SVNN standard
    const MAX_OBSERVABLES_LINE: usize = 5; // max in a single line
    const OBSERVABLE_F14_WIDTH: usize = 14;
    const OBSERVABLE_WIDTH: usize = OBSERVABLE_F14_WIDTH + 2; // data +lli +snr +1separator

    // basic check that avoid entering the loop for nothing
    if systems_str_len < SVNN_SIZE {
        // does not look good (=rubbish first line)
        //#[cfg(feature = "log")]
        //error!("parse_sig_v2(aborted): empty content");
        return;
    }

    // sv pointer
    let mut sv = SV::default();
    let mut sv_ptr = 0;
    let mut sv_identified = false;
    // let numsat = systems_str_len / SVNN_SIZE;

    // observable pointer
    let mut obs_ptr = 0;
    let mut obs_identified = false;
    let mut observables = &Vec::<Observable>::default();

    // browse all lines
    for line in lines {
        // [SV] identification
        //  1. on first line (not defined yet!)
        //  2. every time one SV is concluded
        if !sv_identified {
            // identify new SV
            let sv_end = (sv_ptr + SVNN_SIZE).min(systems_str_len);
            let system = &systems_str[sv_ptr..sv_end].trim();

            // actual parsing
            if let Ok(found) = SV::from_str(system) {
                sv = found;
            } else {
                // This may fail on very old RINEX mono GNSS
                // that omit the constellation in the description
                match head_constellation {
                    Some(Constellation::Mixed) | None => {
                        //#[cfg(feature = "log")]
                        //error!("parse_sig_v2(abort): no constell specs");
                        break;
                    },
                    Some(gnss) => {
                        if let Ok(prn) = system.trim().parse::<u8>() {
                            sv = SV {
                                prn,
                                constellation: gnss,
                            };
                        } else {
                            //#[cfg(feature = "log")]
                            //error!("parse_sig_v2(abort): invalid sv");
                            break;
                        }
                    },
                }
            }

            // move on to next
            sv_ptr += SVNN_SIZE;
            sv_identified = true;
            obs_identified = false;
        }

        // [Observable] identification
        //  - locate [Observable]s specs for this [SV]
        if !obs_identified {
            observables = if sv.constellation.is_sbas() {
                if let Some(observables) = head_observables.get(&Constellation::SBAS) {
                    observables
                } else {
                    //#[cfg(feature = "log")]
                    //error!("parse_sig_v2 (sbas): no specs");
                    break;
                }
            } else {
                if let Some(observables) = head_observables.get(&sv.constellation) {
                    observables
                } else {
                    //#[cfg(feature = "log")]
                    //error!("parse_sig_v2 ({}): no specs", sv.constellation);
                    break;
                }
            };

            obs_ptr = 0;
            obs_identified = true;

            //#[cfg(feature = "log")]
            //debug!("{}: {:?}", sv, observables);
        }

        let line_width = line.len();
        let trimmed_len = line.trim().len();

        if trimmed_len == 0 {
            //#[cfg(feature = "log")]
            //error!("empty line: \"{}\"", line);

            obs_ptr += MAX_OBSERVABLES_LINE;

            // this concludes this vehicle
            if obs_ptr >= observables.len() {
                sv_identified = false;
                obs_identified = false;
                if sv_ptr == systems_str_len {
                    // we're done
                    return;
                }
            }

            continue;
        }

        // num obs contained this line
        let num_obs = div_ceil(line_width, OBSERVABLE_WIDTH);

        let mut offset = 0;

        //#[cfg(feature = "log")]
        //debug!(
        //    "line: \"{}\" [sv={}/{} obs={}/{}]",
        //    line,
        //    sv_ptr,
        //    systems_str_len,
        //    obs_ptr,
        //    observables.len()
        //);

        // process all of them
        for _ in 0..num_obs {
            if obs_ptr >= observables.len() {
                // line is abnormally long (trailing whitespaces): abort
                return;
            }

            let end = (offset + OBSERVABLE_WIDTH).min(line_width);
            let slice = &line[offset..end];

            //#[cfg(feature = "log")]
            //println!("observation: [{}] \"{}\" {}", sv, slice, observables[obs_ptr]);

            // parse possible LLI
            let mut lli = Option::<LliFlags>::None;

            if slice.len() > OBSERVABLE_F14_WIDTH {
                let lli_slice = &slice[OBSERVABLE_F14_WIDTH..OBSERVABLE_F14_WIDTH + 1];
                //println!("lli: \"{}\"", lli_slice);
                match lli_slice.parse::<u8>() {
                    Ok(unsigned) => {
                        lli = LliFlags::from_bits(unsigned);
                    },
                    Err(_) => {},
                }
            }

            // parse possible SNR
            let mut snr = Option::<SNR>::None;

            if slice.len() > OBSERVABLE_F14_WIDTH + 1 {
                let snr_slice = &slice[OBSERVABLE_F14_WIDTH + 1..OBSERVABLE_F14_WIDTH + 2];
                match SNR::from_str(snr_slice) {
                    Ok(found) => {
                        //println!("SNR (OK): \"{}\" [{:?}]", snr_slice, observables[obs_ptr]); // DEBUG
                        snr = Some(found);
                    },
                    Err(_) => {
                        // println!("SNR (ERR): {:?} [{:?}]", e, observables[obs_ptr]); // DEBUG
                    },
                }
            }

            // parse observed value
            let end = slice.len().min(OBSERVABLE_F14_WIDTH);

            if let Ok(value) = slice[..end].trim().parse::<f64>() {
                signals.push(SignalObservation {
                    sv,
                    snr,
                    lli,
                    value,
                    observable: observables[obs_ptr].clone(),
                });
            }

            obs_ptr += 1;
            offset += OBSERVABLE_F14_WIDTH + 2;

            if obs_ptr == observables.len() {
                //#[cfg(feature = "log")]
                //debug!("{} completed", sv);

                sv_identified = false;
                obs_identified = false;

                if sv_ptr == systems_str_len {
                    // we're done
                    return;
                }
            } else {
                //#[cfg(feature = "log")]
                //debug!("{}/{}", obs_ptr, observables.len());
            }
        } //num_obs

        if num_obs < MAX_OBSERVABLES_LINE {
            obs_ptr += MAX_OBSERVABLES_LINE - num_obs;
        }
    } // for all lines provided
}

/// Parses all [SignalObservation]s described by following [Lines].
fn parse_signals_v3(
    head_observables: &HashMap<Constellation, Vec<Observable>>,
    lines: Lines<'_>,
    signals: &mut Vec<SignalObservation>,
) {
    const SVNN_SIZE: usize = 3;
    const OBSERVABLE_F14_WIDTH: usize = 14;
    const OBSERVABLE_WIDTH: usize = 16; // F14 +2 flags

    let mut sv; // single alloc

    // browse all lines
    for line in lines {
        // identify SV
        let sv_str = &line[0..SVNN_SIZE];
        match SV::from_str(sv_str) {
            Ok(found) => {
                sv = found;
            },
            Err(_) => {
                continue;
            },
        }

        // identify [Observable]s
        let observables = if sv.constellation.is_sbas() {
            head_observables.get(&Constellation::SBAS)
        } else {
            head_observables.get(&sv.constellation)
        };

        if observables.is_none() {
            continue;
        }

        let observables = observables.unwrap();

        let num_obs = line.len() / OBSERVABLE_WIDTH;
        let mut offset = SVNN_SIZE + 1;

        for i in 0..num_obs {
            if i == observables.len() {
                // line is abnormally long (trailing whitespaces)
                break;
            }

            let end = (offset + OBSERVABLE_WIDTH).min(line.len());
            let slice = &line[offset..end];

            let mut lli = Option::<LliFlags>::None;

            if slice.len() > OBSERVABLE_F14_WIDTH {
                let start = offset + OBSERVABLE_F14_WIDTH - 1;
                let lli_slice = &line[start..start + 1];
                match lli_slice.parse::<u8>() {
                    Ok(unsigned) => {
                        lli = LliFlags::from_bits(unsigned);
                    },
                    Err(_e) => {
                        //#[cfg(feature = "log")]
                        //error!("lli: {}", e);
                    },
                }
            }

            let mut snr = Option::<SNR>::None;

            if slice.len() > OBSERVABLE_F14_WIDTH + 1 {
                let start = offset + OBSERVABLE_F14_WIDTH;
                let snr_slice = &line[start..start + 1];

                if let Ok(value) = snr_slice.parse::<u8>() {
                    snr = Some(SNR::from(value));
                }
            }

            let end = OBSERVABLE_F14_WIDTH.min(slice.len());

            if let Ok(value) = slice[..end].trim().parse::<f64>() {
                signals.push(SignalObservation {
                    sv,
                    value,
                    lli,
                    snr,
                    observable: observables[i].clone(),
                });
            }

            offset += OBSERVABLE_F14_WIDTH + 2;
        }
    } //browse all lines
}

#[cfg(test)]
mod test {
    use super::is_new_epoch;
    use crate::{
        observation::{EpochFlag, SignalObservation, SNR},
        prelude::{Constellation, Observable, Version, SV},
        tests::toolkit::generic_observation_epoch_decoding_test,
    };
    use std::str::FromStr;

    #[test]
    fn test_new_epoch() {
        assert!(is_new_epoch(
            "95 01 01 00 00 00.0000000  0  7 06 17 21 22 23 28 31",
            Version { major: 2, minor: 0 }
        ));
        assert!(!is_new_epoch("      ", Version { major: 2, minor: 0 }));
        assert!(!is_new_epoch(" ", Version { major: 2, minor: 0 }));
        assert!(!is_new_epoch("", Version { major: 2, minor: 0 }));
        assert!(!is_new_epoch(
            "21700656.31447  16909599.97044          .00041  24479973.67844  24479975.23247",
            Version { major: 2, minor: 0 }
        ));
        assert!(!is_new_epoch(
            "  20849594.124   111570400.06907  86776964.96746  20849589.804          49.000",
            Version { major: 2, minor: 0 }
        ));
        assert!(!is_new_epoch(
            "  21911712.315 7  21911713.545 7  21911710.943 8 115146830.08007  89724806.65908",
            Version { major: 2, minor: 0 },
        ));
        assert!(is_new_epoch(
            "95 01 01 11 00 00.0000000  0  8 04 16 18 19 22 24 27 29",
            Version { major: 2, minor: 0 }
        ));
        assert!(!is_new_epoch(
            "95 01 01 11 00 00.0000000  0  8 04 16 18 19 22 24 27 29",
            Version { major: 3, minor: 0 }
        ));
        assert!(is_new_epoch(
            "> 2022 01 09 00 00 30.0000000  0 40",
            Version { major: 3, minor: 0 }
        ));
        assert!(!is_new_epoch(
            "> 2022 01 09 00 00 30.0000000  0 40",
            Version { major: 2, minor: 0 }
        ));
        assert!(!is_new_epoch(
            "G01  22331467.880   117352685.28208        48.950    22331469.28",
            Version { major: 3, minor: 0 }
        ));
    }

    #[test]
    fn test_parse_v3_1() {
        let content =
"> 2022 03 04  0  0  0.0000000  0 22        .000000000000
G01  20832393.682   109474991.854 8        49.500                    20832389.822                                                                    85305196.437 8                                                                        49.500
G03  20342516.786   106900663.487 8        50.000                    20342512.006                                                                    83299201.382 8                                                                        50.000
G04  22448754.952   117969025.322 8        48.250                    22448749.312                                                                    91923884.833 7                                                                        43.750
G06  24827263.216   130468159.526 6        39.750                    24827259.316                                                                   101663482.505 6                                                                        37.250
G09  25493930.890   133971510.403 6        41.250                    25493926.950                                                                   104393373.997 6                                                                        41.750
";
        generic_observation_epoch_decoding_test(
            content,
            3,
            Constellation::GPS,
            &[
                ("GPS", "C1C, L1C, S1C, C2P, C2W, C2S, C2L, C2X, L2P, L2W, L2S, L2L, L2X, S2P, S2W, S2S, S2L, S2X"),
            ],
            "2022-03-04T00:00:00 GPST",
            30,
            "2022-03-04T00:00:00 GPST",
            EpochFlag::Ok,
            None,
            vec![
                SignalObservation {
                    sv: SV::from_str("G01").unwrap(),
                    observable: Observable::from_str("C1C").unwrap(),
                    value: 20832393.682,
                    lli: None,
                    snr: None,
                },
                SignalObservation {
                    sv: SV::from_str("G01").unwrap(),
                    observable: Observable::from_str("L1C").unwrap(),
                    value: 109474991.854,
                    lli: None,
                    snr: Some(SNR::from(8)),
                },
                SignalObservation {
                    sv: SV::from_str("G01").unwrap(),
                    observable: Observable::from_str("S1C").unwrap(),
                    value: 49.5,
                    lli: None,
                    snr: None,
                },
                SignalObservation {
                    sv: SV::from_str("G09").unwrap(),
                    observable: Observable::from_str("L1C").unwrap(),
                    value: 133971510.403,
                    lli: None,
                    snr: Some(SNR::from(6)),
                },
            ],
        );
    }

    #[test]
    fn test_parse_v3_2() {
        let content =
"> 2022 03 04 00 00  0.0000000  0  9
G01  20176608.780   106028802.11808     -1009.418          50.250    20176610.080    82619851.24609      -786.562          54.500
G03  20719565.760   108882069.81508       762.203          49.750    20719566.420    84843175.68809       593.922          55.000
G04  21342618.100   112156219.39808      2167.688          48.250    21342617.440    87394474.09607      1689.105          45.500                                                                 104393373.997 6                                                                        41.750
";
        generic_observation_epoch_decoding_test(
            content,
            3,
            Constellation::GPS,
            &[("GPS", "C1C, L1C, D1C, S1C, C2W, L2W, D2W, S2W")],
            "2022-03-04T00:00:00 GPST",
            24,
            "2022-03-04T00:00:00 GPST",
            EpochFlag::Ok,
            None,
            vec![],
        );
    }

    #[test]
    fn test_parse_v2_1() {
        let content = " 21 01 01 00 00 00.0000000  0 24G07G08G10G13G15G16G18G20G21G23G26G27
G30R01R02R03R08R09R15R16R17R18R19R24
  24178026.635 6  24178024.891 6                 127056391.69906  99004963.01703
                  24178026.139 3  24178024.181 3        38.066          22.286

  21866748.928 7  21866750.407 7  21866747.537 8 114910552.08207  89540700.32608
  85809828.27608  21866748.200 8  21866749.482 8        45.759          49.525
  52.161
  21458907.960 8  21458908.454 7  21458905.489 8 112767333.29708  87870655.27209
  84209365.43808  21458907.312 9  21458908.425 9        50.526          55.388
  53.157
  25107711.730 5                                 131941919.38305 102811868.09001
  25107711.069 1  25107709.586 1        33.150           8.952

  24224693.760 6  24224693.174 5                 127301651.00206  99196079.53805
                  24224693.407 5  24224691.898 5        36.121          31.645

  21749627.212 8                                 114295057.63608  89061063.16706
                  21749626.220 6  21749624.795 6        48.078          39.240

  23203962.113 6  23203960.554 6  23203963.222 7 121937655.11806  95016353.74904
  91057352.20207  23203961.787 4  23203960.356 4        41.337          28.313
";

        generic_observation_epoch_decoding_test(
            content,
            2,
            Constellation::GPS,
            &[("GPS", "C1, C2, C5, L1, L2, L5, P1, P2, S1, S2, S5")],
            "2021-01-01T00:00:00 GPST",
            62,
            "2021-01-01T00:00:00 GPST",
            EpochFlag::Ok,
            None,
            vec![],
        );
    }

    #[test]
    fn test_parse_v2_2() {
        // extracted from v2/npaz3550_21o
        let content = " 21 12 21 00 00 30.0000000  0 17G08G10G15G16G18G21G23G26G32R04R05R06
                                R10R12R19R20R21
  22273618.192   117048642.67706  91206745.37646  22273620.492          45.000  
        27.000  
  20679832.284   108673270.42707  84680474.73748  20679834.464          51.000  
        47.000  
  24428574.718   128373032.12605 100030940.88146  24428574.158          40.000  
        26.000  
  21748622.436   114289784.86106  89056973.16346  21748622.736          46.000  
        30.000  
  23159990.646   121706608.75906  94836327.00246  23159990.946          43.000  
        22.000  
  24121222.480   126757874.84305  98772376.29745  24121222.660          40.000  
        17.000  
  21243754.540   111636702.47207  86989641.56447  21243755.220          48.000  
        33.000  
  23730924.382   124706843.12805  97174162.95546  23730927.462          41.000  
        30.000  
  25068865.594   131737775.26304 102652811.50846  25068867.574          38.000  
        24.000  
  21628749.716   115820985.39407  90082976.65346  21628746.496          48.000  
        22.000  
  20263702.068   108321110.70607  84249758.00647  20263698.768          50.000  
        34.000  
  23144036.268   123501011.94300                                        32.000  

  22905715.968   122100341.64505                                        42.000  
                
  23665462.924   126416648.11306  98324062.16346  23665459.604          43.000  
        25.000  
  23266753.448   124461433.23101                                        33.000  
                
  19895900.968   106392328.16604  82749588.85847  19895899.328          39.000  
        33.000  
  21758080.616   116431896.04406  90558129.99946  21758077.536          47.000  
        22.000  ";
        generic_observation_epoch_decoding_test(
            content,
            2,
            Constellation::Mixed,
            &[
                ("GPS", "C1, L1, L2, P2, S1, S2"),
                ("GLO", "C1, L1, L2, P2, S1, S2"),
            ],
            "2021-12-21T00:00:30 GPST",
            93,
            "2021-12-21T00:00:30 GPST",
            EpochFlag::Ok,
            None,
            vec![],
        );
    }

    #[test]
    fn test_parse_v2_barq() {
        // extracted from v2/barq0.19d
        let content = " 19  3 12 16 36  0.0000000  0 15G08G10G14G16G18G20G22G26G27G32R04R05
        R06R19R20
111525030.92718  86902614.11057  21222508.060                    21222505.880
113917475.72718  88766858.51957  21677775.000                    21677773.380
117582228.43417  91622504.36956  22375154.360                    22375150.800
116496701.38017  90776637.01356  22168585.640                    22168581.380
120543799.60817  93930215.60156  22938722.560                    22938718.700
125411881.34917  97723533.46955  23865086.320                    23865082.480
124148906.10717  96739381.30556  23624751.240                    23624745.820
127085060.78216  99027334.17655  24183483.520                    24183486.540
105970568.06418  82574457.73858  20165528.740                    20165526.020
114286506.03418  89054414.91257  21747998.820                    21747997.860
113282224.20717  88108400.20417  21154656.900                    21154656.960
102507243.91718  79727864.12617  19176099.600                    19176101.880
116775184.83016                  21883618.780
112181738.38117  87252468.04516  20971192.900                    20971194.040
110923986.30317  86274222.72417  20743344.820                    20743348.200
";
        generic_observation_epoch_decoding_test(
            content,
            2,
            Constellation::Mixed,
            &[
                ("GPS", "L1,    L2,    C1,    P1,    P2,    P1,    P2"),
                ("GLO", "L1,    L2,    C1,    P1,    P2,    P1,    P2"),
            ],
            "2019-03-12T16:36:00 GPST",
            50,
            "2019-03-12T16:36:00 GPST",
            EpochFlag::Ok,
            None,
            vec![],
        );
    }

    #[test]
    fn test_parse_v2_20th_century() {
        // extracted from v2/barq0.19d
        let content = " 95  3 12 16 36  0.0000000  0 15G08G10G14G16G18G20G22G26G27G32R04R05
        R06R19R20
111525030.92718  86902614.11057  21222508.060                    21222505.880
113917475.72718  88766858.51957  21677775.000                    21677773.380
117582228.43417  91622504.36956  22375154.360                    22375150.800
116496701.38017  90776637.01356  22168585.640                    22168581.380
120543799.60817  93930215.60156  22938722.560                    22938718.700
125411881.34917  97723533.46955  23865086.320                    23865082.480
124148906.10717  96739381.30556  23624751.240                    23624745.820
127085060.78216  99027334.17655  24183483.520                    24183486.540
105970568.06418  82574457.73858  20165528.740                    20165526.020
114286506.03418  89054414.91257  21747998.820                    21747997.860
113282224.20717  88108400.20417  21154656.900                    21154656.960
102507243.91718  79727864.12617  19176099.600                    19176101.880
116775184.83016                  21883618.780
112181738.38117  87252468.04516  20971192.900                    20971194.040
110923986.30317  86274222.72417  20743344.820                    20743348.200
";
        generic_observation_epoch_decoding_test(
            content,
            2,
            Constellation::Mixed,
            &[
                ("GPS", "L1,    L2,    C1,    P1,    P2,    P1,    P2"),
                ("GLO", "L1,    L2,    C1,    P1,    P2,    P1,    P2"),
            ],
            "1995-03-12T16:36:00 GPST",
            50,
            "1995-03-12T16:36:00 GPST",
            EpochFlag::Ok,
            None,
            vec![],
        );
    }
}
