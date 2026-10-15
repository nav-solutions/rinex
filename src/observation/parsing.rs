//! Observation RINEX parsing
use crate::{
    epoch::{parse_in_timescale as parse_epoch_in_timescale, parse_utc as parse_utc_epoch},
    observation::{ClockObservation, EpochFlag, LliFlags, Observations, SignalObservation, SNR},
    parse_f64,
    prelude::{Constellation, Duration, Header, Observable, ParsingError, TimeScale, Version, SV},
    utils::validate_ascii,
};

#[cfg(test)]
use crate::observation::ObsKey;

use std::str::FromStr;

// TODO: see if we can get rid of num_integer

// #[cfg(feature = "log")]
// use log::{debug, error};

/// Returns true if provided content matches the start of a new Observation Epoch
pub fn is_new_epoch(line: &str, v: Version) -> bool {
    if v.major < 3 {
        if line.len() < 30 {
            false
        } else {
            // SPLICE flag handling (still an Observation::flag)
            let (Some(date), Some(flag)) = (line.get(..26), line.get(26..29)) else {
                return false;
            };
            let significant = !date.trim().is_empty();
            let epoch = parse_utc_epoch(date);
            let flag = EpochFlag::from_str(flag.trim());
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

/// Fields confirmed by the native decoder. An absent epoch or flag must not be
/// guessed. In particular, special events may legitimately have no timestamp.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ObsContext {
    pub flag: Option<EpochFlag>,
    pub epoch: Option<crate::prelude::Epoch>,
}

pub(super) struct EpochDescriptor {
    pub epoch: Option<crate::prelude::Epoch>,
    pub flag: EpochFlag,
    pub count: usize,
    pub clock: Option<f64>,
}

pub(super) fn descriptor_fields(
    line: &str,
    version: Version,
) -> Result<(EpochFlag, usize), ParsingError> {
    let offset = if version.major < 3 { 26 } else { 29 };
    let flag = line
        .get(offset..offset + 3)
        .ok_or(ParsingError::EpochFormat)?;
    let count = line
        .get(offset + 3..offset + 6)
        .ok_or(ParsingError::EpochFormat)?;
    Ok((
        EpochFlag::from_str(flag.trim())?,
        count
            .trim()
            .parse::<u16>()
            .map_err(|_| ParsingError::NumSatParsing)? as usize,
    ))
}

pub(super) fn parse_descriptor(
    header: &Header,
    line: &str,
    ts: TimeScale,
    context: &mut ObsContext,
) -> Result<EpochDescriptor, ParsingError> {
    validate_ascii(line, 1)?;
    if header.version.major >= 3 && !line.starts_with('>') {
        return Err(ParsingError::EpochFormat);
    }
    let offset = if header.version.major < 3 { 26 } else { 29 };
    let date = line
        .get(..offset)
        .ok_or(ParsingError::EpochFormat)?
        .trim_start_matches('>');
    let flag = line
        .get(offset..offset + 3)
        .ok_or(ParsingError::EpochFormat)?;
    let parsed_flag = EpochFlag::from_str(flag.trim());
    context.flag = parsed_flag.as_ref().ok().copied();
    let special = matches!(
        context.flag,
        Some(
            EpochFlag::AntennaBeingMoved
                | EpochFlag::NewSiteOccupation
                | EpochFlag::HeaderInformationFollows
                | EpochFlag::ExternalEvent
        )
    );
    let mut epoch = if special && date.trim().is_empty() {
        // Flag 5 has a significant epoch: RINEX 2.11 Table A2 / 4.02 Table A3.
        if context.flag == Some(EpochFlag::ExternalEvent) {
            return Err(ParsingError::EpochFormat);
        }
        None
    } else {
        Some(parse_epoch_in_timescale(date, ts)?)
    };
    let flag = parsed_flag?;
    let (_, count) = descriptor_fields(line, header.version)?;
    if header.version >= Version::new(4, 2) {
        if let Some(extension) = line.get(57..).map(str::trim).filter(|s| !s.is_empty()) {
            if extension.len() != 5 || !extension.bytes().all(|b| b.is_ascii_digit()) {
                return Err(ParsingError::EpochParsing);
            }
            let picoseconds = extension
                .parse::<u32>()
                .map_err(|_| ParsingError::EpochParsing)?;
            if picoseconds % 1000 != 0 {
                return Err(ParsingError::EpochPrecision);
            }
            if let Some(epoch) = &mut epoch {
                *epoch += Duration::from_nanoseconds((picoseconds / 1000) as f64);
            } else {
                return Err(ParsingError::EpochParsing);
            }
        }
    }
    context.epoch = epoch;
    let clock_start = if header.version.major < 3 { 68 } else { 41 };
    let clock_end = if header.version.major < 3 { 80 } else { 56 };
    let clock = line
        .get(clock_start..line.len().min(clock_end))
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let clock = clock
        .map(|s| parse_f64(s).map_err(|_| ParsingError::ObsClockParsing))
        .transpose()?;
    if clock.is_some_and(|v| !v.is_finite()) {
        return Err(ParsingError::ObsClockParsing);
    }
    Ok(EpochDescriptor {
        epoch,
        flag,
        count,
        clock,
    })
}

// Internal single-block entry point, also used by the existing test toolkit.
// Output is replaced atomically; failed calls leave it empty.
#[cfg(test)]
pub(crate) fn parse_epoch(
    header: &Header,
    content: &str,
    ts: TimeScale,
    observations: &mut Observations,
) -> Result<ObsKey, ParsingError> {
    parse_epoch_with_context(header, content, ts, observations).0
}

#[cfg(test)]
pub(crate) fn parse_epoch_with_context(
    header: &Header,
    content: &str,
    ts: TimeScale,
    observations: &mut Observations,
) -> (Result<ObsKey, ParsingError>, ObsContext) {
    *observations = Observations::default();
    let mut context = ObsContext::default();
    let result = (|| {
        let first = content.lines().next().ok_or(ParsingError::EmptyEpoch)?;
        let descriptor = parse_descriptor(header, first, ts, &mut context)?;
        if !matches!(
            descriptor.flag,
            EpochFlag::Ok | EpochFlag::PowerFailure | EpochFlag::CycleSlip
        ) {
            return Err(ParsingError::ObsHardwareEvent);
        }
        let (data, _) = parse_observations(header, content, &descriptor)?;
        let key = ObsKey {
            epoch: descriptor.epoch.ok_or(ParsingError::EpochFormat)?,
            flag: descriptor.flag,
        };
        *observations = data;
        Ok(key)
    })();
    (result, context)
}

pub(super) fn parse_observations(
    header: &Header,
    content: &str,
    descriptor: &EpochDescriptor,
) -> Result<(Observations, Vec<SV>), ParsingError> {
    validate_ascii(content, 1)?;
    let specs = header
        .obs
        .as_ref()
        .ok_or(ParsingError::BadObsBadTimescaleDefinition)?;
    let mut lines = content
        .lines()
        .enumerate()
        .filter(|(_, line)| !crate::is_rinex_comment(line));
    let (_, first) = lines.next().ok_or(ParsingError::EmptyEpoch)?;
    let mut observations = Observations::default();
    let mut present_satellites = Vec::with_capacity(descriptor.count);
    if let (Some(epoch), Some(offset)) = (descriptor.epoch, descriptor.clock) {
        observations.clock = Some(ClockObservation::default().with_offset_s(epoch, offset));
    }
    if header.version.major < 3 {
        let mut satellites = first.get(32..first.len().min(68)).unwrap_or("").to_owned();
        let extra = descriptor.count.div_ceil(12).saturating_sub(1);
        for _ in 0..extra {
            let (_, line) = lines
                .next()
                .ok_or(ParsingError::BadV2SatellitesDescription)?;
            if !line.get(..32).is_some_and(|s| s.trim().is_empty()) {
                return Err(ParsingError::BadV2SatellitesDescription);
            }
            satellites.push_str(line.get(32..line.len().min(68)).unwrap_or(""));
        }
        let mut seen = std::collections::HashSet::new();
        for index in 0..descriptor.count {
            let system = satellites
                .get(index * 3..index * 3 + 3)
                .ok_or(ParsingError::BadV2SatellitesDescription)?
                .trim();
            let sv = SV::from_str(system)
                .or_else(|_| {
                    let constellation = header
                        .constellation
                        .filter(|c| *c != Constellation::Mixed)
                        .ok_or(ParsingError::BadV2SatellitesDescription)?;
                    Ok::<_, ParsingError>(SV {
                        constellation,
                        prn: system
                            .parse::<u8>()
                            .map_err(|_| ParsingError::BadV2SatellitesDescription)?,
                    })
                })
                .map_err(|_| ParsingError::BadV2SatellitesDescription)?;
            if !seen.insert(sv) {
                return Err(ParsingError::BadV2SatellitesDescription);
            }
            present_satellites.push(sv);
            let constellation = if sv.constellation.is_sbas() {
                Constellation::SBAS
            } else {
                sv.constellation
            };
            let codes = specs
                .codes
                .get(&constellation)
                .ok_or(ParsingError::BadObservable)?;
            for chunk in codes.chunks(5) {
                let (line_index, line) = lines.next().ok_or(ParsingError::ObsSatelliteCount {
                    expected: descriptor.count,
                    found: index,
                })?;
                parse_signal_fields(
                    sv,
                    chunk,
                    line,
                    0,
                    line_index + 1,
                    descriptor.flag == EpochFlag::CycleSlip,
                    &mut observations.signals,
                )?;
            }
        }
        if lines.any(|(_, l)| !l.trim().is_empty()) {
            return Err(ParsingError::ObsSatelliteCount {
                expected: descriptor.count,
                found: descriptor.count + 1,
            });
        }
    } else {
        let mut seen = std::collections::HashSet::new();
        for (line_index, line) in lines {
            if line.trim().is_empty() {
                continue;
            }
            let sv = line.get(..3).and_then(|s| SV::from_str(s).ok()).ok_or(
                ParsingError::ObsSignalParsing {
                    line: line_index + 1,
                    byte_column: 1,
                },
            )?;
            if !seen.insert(sv) {
                return Err(ParsingError::ObsSatelliteCount {
                    expected: descriptor.count,
                    found: seen.len() + 1,
                });
            }
            present_satellites.push(sv);
            let constellation = if sv.constellation.is_sbas() {
                Constellation::SBAS
            } else {
                sv.constellation
            };
            let codes = specs
                .codes
                .get(&constellation)
                .ok_or(ParsingError::BadObservable)?;
            parse_signal_fields(
                sv,
                codes,
                line,
                3,
                line_index + 1,
                descriptor.flag == EpochFlag::CycleSlip,
                &mut observations.signals,
            )?;
        }
        if seen.len() != descriptor.count {
            return Err(ParsingError::ObsSatelliteCount {
                expected: descriptor.count,
                found: seen.len(),
            });
        }
    }
    Ok((observations, present_satellites))
}

fn parse_signal_fields(
    sv: SV,
    codes: &[Observable],
    line: &str,
    start: usize,
    line_number: usize,
    cycle_slip: bool,
    signals: &mut Vec<SignalObservation>,
) -> Result<(), ParsingError> {
    if line
        .get(start + codes.len() * 16..)
        .is_some_and(|s| !s.trim().is_empty())
    {
        return Err(ParsingError::ObsSignalParsing {
            line: line_number,
            byte_column: start + codes.len() * 16 + 1,
        });
    }
    for (index, observable) in codes.iter().enumerate() {
        let offset = start + index * 16;
        let field = line.get(offset..line.len().min(offset + 16)).unwrap_or("");
        let scalar = field.get(..field.len().min(14)).unwrap_or("").trim();
        let error = || ParsingError::ObsSignalParsing {
            line: line_number,
            byte_column: offset + 1,
        };
        let lli = field
            .get(14..15)
            .filter(|s| *s != " ")
            .map(|s| {
                s.parse::<u8>()
                    .ok()
                    .and_then(LliFlags::from_bits)
                    .ok_or_else(error)
            })
            .transpose()?;
        let snr = field
            .get(15..16)
            .filter(|s| *s != " ")
            .map(|s| s.parse::<u8>().map(SNR::from).map_err(|_| error()))
            .transpose()?;
        if cycle_slip
            && (lli.is_some_and(|v| !v.is_empty()) || snr.is_some_and(|v| v != SNR::DbHz0))
        {
            return Err(error());
        }
        if scalar.is_empty() {
            continue;
        }
        let value = parse_f64(scalar).map_err(|_| error())?;
        if !value.is_finite() {
            return Err(error());
        }
        signals.push(SignalObservation {
            sv,
            observable: observable.clone(),
            value,
            lli,
            snr,
        });
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::is_new_epoch;
    use crate::{
        observation::{ClockObservation, EpochFlag, LliFlags, SignalObservation, SNR},
        prelude::{Constellation, Epoch, Observable, Version, SV},
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
        // Five selected source rows; the envelope declares that subset.
        let content =
"> 2022 03 04  0  0  0.0000000  0  5        .000000000000
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
            &[(
                "GPS",
                "C1C, L1C, S1C, C2P, C2W, C2S, C2L, C2X, L2P, L2W, L2S, L2L, L2X, S2P, S2W, S2S, S2L, S2X",
            )],
            "2022-03-04T00:00:00 GPST",
            30,
            "2022-03-04T00:00:00 GPST",
            EpochFlag::Ok,
            // receiver clock offset is present, zero valued
            Some(ClockObservation::default().with_offset_s(
                Epoch::from_str("2022-03-04T00:00:00 GPST").unwrap(),
                0.0,
            )),
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
        // Three rows and eight selected observable columns.
        let content =
"> 2022 03 04 00 00  0.0000000  0  3
G01  20176608.780   106028802.11808     -1009.418          50.250    20176610.080    82619851.24609      -786.562          54.500
G03  20719565.760   108882069.81508       762.203          49.750    20719566.420    84843175.68809       593.922          55.000
G04  21342618.100   112156219.39808      2167.688          48.250    21342617.440    87394474.09607      1689.105          45.500
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
        // Seven selected GPS rows with their explicit final blank record.
        let content = " 21 01 01 00 00 00.0000000  0  7G07G08G10G13G15G16G18
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
        // Source excerpt with synthetic fixed-column padding restored.
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
            &[("GPS", "L1, L2, C1, P1, P2"), ("GLO", "L1, L2, C1, P1, P2")],
            "2019-03-12T16:36:00 GPST",
            58,
            "2019-03-12T16:36:00 GPST",
            EpochFlag::Ok,
            None,
            vec![
                SignalObservation {
                    sv: SV::from_str("G08").unwrap(),
                    observable: Observable::from_str("L1").unwrap(),
                    value: 111525030.927,
                    lli: Some(LliFlags::from_bits(1).unwrap()),
                    snr: Some(SNR::from(8)),
                },
                SignalObservation {
                    sv: SV::from_str("G10").unwrap(),
                    observable: Observable::from_str("L1").unwrap(),
                    value: 113917475.727,
                    lli: Some(LliFlags::from_bits(1).unwrap()),
                    snr: Some(SNR::from(8)),
                },
            ],
        );
    }

    #[test]
    fn test_parse_v2_20th_century() {
        // Source excerpt with synthetic fixed-column padding restored.
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
            &[("GPS", "L1, L2, C1, P1, P2"), ("GLO", "L1, L2, C1, P1, P2")],
            "1995-03-12T16:36:00 GPST",
            58,
            "1995-03-12T16:36:00 GPST",
            EpochFlag::Ok,
            None,
            vec![
                SignalObservation {
                    sv: SV::from_str("G08").unwrap(),
                    observable: Observable::from_str("L1").unwrap(),
                    value: 111525030.927,
                    lli: Some(LliFlags::from_bits(1).unwrap()),
                    snr: Some(SNR::from(8)),
                },
                SignalObservation {
                    sv: SV::from_str("G10").unwrap(),
                    observable: Observable::from_str("L1").unwrap(),
                    value: 113917475.727,
                    lli: Some(LliFlags::from_bits(1).unwrap()),
                    snr: Some(SNR::from(8)),
                },
            ],
        );
    }
}
