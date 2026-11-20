//! Describes a `RINEX` file header.
use crate::{
    antex::{HeaderFields as AntexHeader, Pcv},
    clock::{ClockProfileType, HeaderFields as ClockHeader, WorkClock},
    hardware::{Antenna, Receiver, SvAntenna},
    hatanaka::CRINEX,
    header::{DcbCompensation, Header, PcvCompensation},
    leap::Leap,
    marker::MarkerType,
    meteo::{HeaderFields as MeteoHeader, Sensor as MeteoSensor},
    navigation::{HeaderFields as NavigationHeader, IonosphereModel, KbModel, TimeOffset},
    observable::Observable,
    observation::HeaderFields as ObservationHeader,
    parse_f64,
    prelude::{Constellation, Duration, Epoch, ParsingError, TimeScale, COSPAR, DOMES, SV},
    types::Type,
    utils::validate_ascii,
    version::Version,
};

use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Read},
    str::FromStr,
};

impl Header {
    /// Parse [Header] by consuming [BufReader] until end of this section
    /// OBS declarations require GNSS code shapes for their format version,
    /// supported, unique codes and matching declared counts, so unsupported
    /// columns cannot shift subsequent values. This is not a per-constellation
    /// signal catalogue check.
    pub fn parse<R: Read>(reader: &mut BufReader<R>) -> Result<Self, ParsingError> {
        Self::parse_with_base(reader, None)
    }

    pub(crate) fn parse_observation_update(&self, records: &str) -> Result<Self, ParsingError> {
        Self::parse_with_base(&mut BufReader::new(records.as_bytes()), Some(self))
    }

    fn parse_with_base<R: Read>(
        reader: &mut BufReader<R>,
        base: Option<&Self>,
    ) -> Result<Self, ParsingError> {
        let mut rinex_type = base.map(|h| h.rinex_type).unwrap_or_default();
        let mut version = base.map(|h| h.version).unwrap_or_default();
        let mut constellation = base.map(|h| h.constellation).unwrap_or_default();

        let mut program = base.map(|h| h.program.clone()).unwrap_or_default();
        let mut run_by = base.map(|h| h.run_by.clone()).unwrap_or_default();
        let mut date = base.map(|h| h.date.clone()).unwrap_or_default();
        let mut observer = base.map(|h| h.observer.clone()).unwrap_or_default();
        let mut agency = base.map(|h| h.agency.clone()).unwrap_or_default();
        let mut license = base.map(|h| h.license.clone()).unwrap_or_default();
        let mut doi = base.map(|h| h.doi.clone()).unwrap_or_default();
        let mut station_url = base.map(|h| h.station_url.clone()).unwrap_or_default();
        let mut geodetic_marker = base.map(|h| h.geodetic_marker.clone()).unwrap_or_default();
        let mut cospar = base.map(|h| h.cospar.clone()).unwrap_or_default();
        let mut glo_channels = base.map(|h| h.glo_channels.clone()).unwrap_or_default();
        let mut rcvr = base.map(|h| h.rcvr.clone()).unwrap_or_default();
        let mut rcvr_antenna = base.map(|h| h.rcvr_antenna.clone()).unwrap_or_default();
        let mut sv_antenna = base.map(|h| h.sv_antenna.clone()).unwrap_or_default();
        let mut leap = base.map(|h| h.leap).unwrap_or_default();
        let mut sampling_interval = base.map(|h| h.sampling_interval).unwrap_or_default();

        let mut rx_position = base.map(|h| h.rx_position).unwrap_or_default();

        let mut dcb_compensations = base
            .map(|h| h.dcb_compensations.clone())
            .unwrap_or_default();
        let mut ionod_corrections = base
            .map(|h| h.ionod_corrections.clone())
            .unwrap_or_default();
        let mut pcv_compensations = base
            .map(|h| h.pcv_compensations.clone())
            .unwrap_or_default();

        let mut comments = Vec::<String>::with_capacity(8);

        // RINEX specific fields
        let mut current_constell: Option<Constellation> = None;
        let mut observation_declarations = Vec::new();

        let mut observation = base.and_then(|h| h.obs.clone()).unwrap_or_default();
        let mut nav = NavigationHeader::default();
        let mut meteo = MeteoHeader::default();
        let mut clock = ClockHeader::default();
        let mut antex = AntexHeader::default();

        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            validate_ascii(&line, index + 1)?;

            if line.len() < 60 {
                continue; // --> invalid header content
            }

            let (content, marker) = line.split_at(60);
            ///////////////////////////////
            // [0] END OF HEADER
            //     --> done parsing
            ///////////////////////////////
            if marker.trim().eq("END OF HEADER") {
                break;
            }
            ///////////////////////////////
            // COMMENTS are stored: "as is"
            ///////////////////////////////
            if marker.trim().eq("COMMENT") {
                // --> storing might be useful
                comments.push(content.trim().to_string());
                continue;

            ///////////////////////////////////////////////////////
            // Handled elsewe: CRINEX specs
            //     handled inside the smart I/O READER
            //     we still have to grab what was idenfied though
            //     and we do this at the end of the Header section
            ///////////////////////////////////////////////////////
            } else if marker.contains("CRINEX VERS") {
                let version = Version::from_str(content[..10].trim())?;
                let crinex = CRINEX::default().with_version(version);

                observation.crinex = Some(crinex);
            } else if marker.contains("CRINEX PROG / DATE") {
                if let Some(ref mut crinex) = observation.crinex {
                    *crinex = crinex.with_prog_date(content)?;
                }

                ///////////////////////////////////////////////////////
                // Unhandled cases: TODO
                ///////////////////////////////////////////////////////
            } else if marker.contains("ANTENNA: B.SIGHT XYZ") {
            } else if marker.contains("ANTENNA: ZERODIR XYZ") {
            } else if marker.contains("ANTENNA: PHASECENTER") {
            } else if marker.contains("CENTER OF MASS: XYZ") {
            } else if marker.contains("PRN / BIAS / RMS") {
            } else if marker.contains("TIME REF STATION") {

                ///////////////////////////////////////////////////////
                // Handled cases
                ///////////////////////////////////////////////////////
            } else if marker.contains("ANTEX VERSION / SYST") {
                let (vers, system) = content.split_at(8);
                let vers = vers.trim();
                version = Version::from_str(vers).or(Err(ParsingError::AntexVersion))?;

                if let Ok(constell) = Constellation::from_str(system.trim()) {
                    constellation = Some(constell)
                }

                rinex_type = Type::AntennaData;
            } else if marker.contains("PCV TYPE / REFANT") {
                let (pcv_str, rem) = content.split_at(20);
                let (rel_type, rem) = rem.split_at(20);
                let (ref_sn, _) = rem.split_at(20);
                if let Ok(mut pcv) = Pcv::from_str(pcv_str.trim()) {
                    if pcv.is_relative() {
                        // try to parse "Relative Type"
                        if !rel_type.trim().is_empty() {
                            pcv = pcv.with_relative_type(rel_type.trim());
                        }
                    }
                    antex = antex.with_pcv_type(pcv);
                }
                if !ref_sn.trim().is_empty() {
                    antex = antex.with_reference_antenna_sn(ref_sn.trim());
                }
            } else if marker.contains("TYPE / SERIAL NO") {
                let items: Vec<&str> = content.split_ascii_whitespace().collect();
                if items.len() == 2 {
                    // Receiver antenna information
                    // like standard RINEX
                    let (model, rem) = content.split_at(20);
                    let (sn, _) = rem.split_at(20);
                    if let Some(a) = &mut rcvr_antenna {
                        *a = a.with_model(model.trim()).with_serial_number(sn.trim());
                    } else {
                        rcvr_antenna = Some(
                            Antenna::default()
                                .with_model(model.trim())
                                .with_serial_number(sn.trim()),
                        );
                    }
                } else if items.len() == 4 {
                    // Space Vehicle antenna information
                    // ANTEX RINEX specific
                    let (model, rem) = content.split_at(10);
                    let (svnn, rem) = rem.split_at(10);
                    let (cospar, _) = rem.split_at(10);
                    if let Ok(sv) = SV::from_str(svnn.trim()) {
                        if let Some(a) = &mut sv_antenna {
                            *a = a
                                .with_sv(sv)
                                .with_model(model.trim())
                                .with_cospar(cospar.trim());
                        } else {
                            sv_antenna = Some(
                                SvAntenna::default()
                                    .with_sv(sv)
                                    .with_model(model.trim())
                                    .with_cospar(cospar.trim()),
                            );
                        }
                    }
                }

            ///////////////////////////////////////
            // ==> from now on
            // RINEX standard / shared attributes
            ///////////////////////////////////////
            } else if marker.contains("RINEX VERSION / TYPE") {
                let (vers, rem) = line.split_at(20);
                let (type_str, rem) = rem.split_at(20);
                let (constell_str, _) = rem.split_at(20);

                let type_str = type_str.trim();
                let constell_str = constell_str.trim();

                // File type identification
                rinex_type = Type::from_str(type_str)?;

                // Determine (file) Constellation
                //  1. NAV SPECIAL CASE
                //  2. OTHER
                match rinex_type {
                    Type::NavigationData => {
                        if type_str.contains("GLONASS") {
                            // old GLONASS NAV : no constellation field
                            constellation = Some(Constellation::Glonass);
                        } else if type_str.contains("GPS NAV DATA") {
                            constellation = Some(Constellation::GPS);
                        } else if type_str.contains("IRNSS NAV DATA") {
                            constellation = Some(Constellation::IRNSS);
                        } else if type_str.contains("GNSS NAV DATA") {
                            constellation = Some(Constellation::Mixed);
                        } else if type_str.eq("NAVIGATION DATA") {
                            if constell_str.is_empty() {
                                // old GPS NAVIGATION DATA
                                constellation = Some(Constellation::GPS);
                            } else {
                                // Modern NAVIGATION DATA
                                if let Ok(c) = Constellation::from_str(constell_str) {
                                    constellation = Some(c);
                                }
                            }
                        }
                    },
                    Type::MeteoData => {
                        // no constellation associated to them
                    },
                    _ => {
                        // any other
                        // regular files
                        if let Ok(c) = Constellation::from_str(constell_str) {
                            constellation = Some(c);
                        }
                    },
                }
                /*
                 * Parse version descriptor
                 */
                let vers = vers.trim();
                version = Version::from_str(vers).or(Err(ParsingError::VersionParsing))?;
            } else if marker.contains("PGM / RUN BY / DATE") {
                let (pgm, rem) = line.split_at(20);
                let pgm = pgm.trim();
                if pgm.len() > 0 {
                    program = Some(pgm.to_string());
                }

                let (runby, rem) = rem.split_at(20);

                let runby = runby.trim();
                if runby.len() > 0 {
                    run_by = Some(runby.to_string());
                }

                let date_str = rem.split_at(20).0.trim();
                if date_str.len() > 0 {
                    date = Some(date_str.to_string());
                }
            } else if marker.contains("MARKER NAME") {
                let name = content.trim();
                geodetic_marker = Some(geodetic_marker.unwrap_or_default().with_name(name));
            } else if marker.contains("MARKER NUMBER") {
                let number = content.split_at(20).0.trim();
                geodetic_marker = Some(geodetic_marker.unwrap_or_default().with_number(number));
            } else if marker.contains("MARKER TYPE") {
                let code = content.split_at(20).0.trim();
                if let Ok(mtype) = MarkerType::from_str(code) {
                    let mut marker = geodetic_marker.unwrap_or_default();
                    marker.marker_type = Some(mtype);
                    geodetic_marker = Some(marker);
                }
            } else if marker.contains("OBSERVER / AGENCY") {
                let (obs, ag) = content.split_at(20);
                let obs = obs.trim();
                let ag = ag.trim();

                if obs.len() > 0 {
                    observer = Some(obs.to_string());
                }

                if ag.len() > 0 {
                    agency = Some(ag.to_string());
                }
            } else if marker.contains("REC # / TYPE / VERS") {
                if let Ok(receiver) = Receiver::from_str(content) {
                    rcvr = Some(receiver);
                }
            } else if marker.contains("SYS / PCVS APPLIED") {
                let (gnss, rem) = content.split_at(2);
                let (program, rem) = rem.split_at(18);
                let (url, _) = rem.split_at(40);

                let gnss = gnss.trim();
                let gnss = Constellation::from_str(gnss.trim())?;

                let pcv = PcvCompensation {
                    program: {
                        let program = program.trim();
                        if program.eq("") {
                            String::from("Unknown")
                        } else {
                            program.to_string()
                        }
                    },
                    constellation: gnss,
                    url: {
                        let url = url.trim();
                        if url.eq("") {
                            String::from("Unknown")
                        } else {
                            url.to_string()
                        }
                    },
                };

                if base.is_some() {
                    pcv_compensations.retain(|entry| entry.constellation != gnss);
                }
                pcv_compensations.push(pcv);
            } else if marker.contains("SYS / DCBS APPLIED") {
                let (gnss, rem) = content.split_at(2);
                let (program, rem) = rem.split_at(18);
                let (url, _) = rem.split_at(40);

                let gnss = gnss.trim();
                let gnss = Constellation::from_str(gnss.trim())?;

                let dcb = DcbCompensation {
                    program: {
                        let program = program.trim();
                        if program.eq("") {
                            String::from("Unknown")
                        } else {
                            program.to_string()
                        }
                    },
                    constellation: gnss,
                    url: {
                        let url = url.trim();
                        if url.eq("") {
                            String::from("Unknown")
                        } else {
                            url.to_string()
                        }
                    },
                };

                if base.is_some() {
                    dcb_compensations.retain(|entry| entry.constellation != gnss);
                }
                dcb_compensations.push(dcb);
            } else if marker.contains("SYS / SCALE FACTOR") {
                // TODO:
                //   This will not work in case several observables
                //   are declaredn which will required to analyze more than 1 line
                let (gnss, rem) = content.split_at(2);
                let gnss = gnss.trim();

                let constell = Constellation::from_str(gnss)?;

                // Parse scaling factor
                let (factor, rem) = rem.split_at(6);
                let factor = factor.trim();
                let scaling = factor
                    .parse::<u16>()
                    .or(Err(ParsingError::SystemScalingFactor))?;

                // parse end of line
                let (_num, rem) = rem.split_at(3);
                for observable_str in rem.split_ascii_whitespace() {
                    let observable = Observable::from_str(observable_str)?;

                    // latch scaling value
                    observation.with_scaling(constell, observable, scaling);
                }
            } else if marker.contains("SENSOR MOD/TYPE/ACC") {
                if let Ok(sensor) = MeteoSensor::from_str(content) {
                    meteo.sensors.push(sensor)
                }
            } else if marker.contains("SENSOR POS XYZ/H") {
                /*
                 * Meteo: sensor position information
                 */
                let (x, rem) = content.split_at(14);
                let (y, rem) = rem.split_at(14);
                let (z, rem) = rem.split_at(14);
                let (h, phys) = rem.split_at(14);

                let phys = phys.trim();
                let observable = Observable::from_str(phys)?;

                let x = x.trim();
                let x = f64::from_str(x).or(Err(ParsingError::SensorCoordinates))?;

                let y = y.trim();
                let y = f64::from_str(y).or(Err(ParsingError::SensorCoordinates))?;

                let z = z.trim();
                let z = f64::from_str(z).or(Err(ParsingError::SensorCoordinates))?;

                let h = h.trim();
                let h = f64::from_str(h).or(Err(ParsingError::SensorCoordinates))?;

                for sensor in meteo.sensors.iter_mut() {
                    if sensor.observable == observable {
                        *sensor = sensor.with_position((x, y, z));
                        *sensor = sensor.with_height(h);
                    }
                }
            } else if marker.contains("LEAP SECOND") {
                let leap_str = content.split_at(40).0;
                let parsed = Leap::from_str(leap_str)?;
                leap = Some(parsed.clone());
            } else if marker.contains("DOI") {
                let content = content.split_at(40).0.trim(); //  TODO: confirm please
                if content.len() > 0 {
                    doi = Some(content.to_string());
                }
            } else if marker.contains("MERGED FILE") {
                //TODO V > 3
                // nb# of merged files
            } else if marker.contains("STATION INFORMATION") {
                let url = content.split_at(40).0.trim(); //TODO confirm please
                if url.len() > 0 {
                    station_url = Some(url.to_string());
                }
            } else if marker.contains("LICENSE OF USE") {
                let lic = content.split_at(40).0.trim(); //TODO confirm please
                if lic.len() > 0 {
                    license = Some(lic.to_string());
                }
            } else if marker.contains("WAVELENGTH FACT L1/2") {
                //TODO
            } else if marker.contains("APPROX POSITION XYZ") {
                let (x, rem) = content.split_at(14);
                let (y, rem) = rem.split_at(14);
                let (z, _) = rem.split_at(14);
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parse_f64(x.trim()),
                    parse_f64(y.trim()),
                    parse_f64(z.trim()),
                ) {
                    rx_position = Some((x, y, z));
                }
            } else if marker.contains("ANT # / TYPE") {
                let (sn, rem) = content.split_at(20);
                let (model, _) = rem.split_at(20);

                rcvr_antenna = Some(
                    rcvr_antenna
                        .unwrap_or_default()
                        .with_model(model.trim())
                        .with_serial_number(sn.trim()),
                );
            } else if marker.contains("ANTENNA: DELTA X/Y/Z") {
                // Antenna Base/Reference Coordinates
                let (x, rem) = content.split_at(14);
                let (y, rem) = rem.split_at(14);
                let (z, _) = rem.split_at(14);
                let x = parse_f64(x.trim()).or(Err(ParsingError::AntennaCoordinates))?;
                let y = parse_f64(y.trim()).or(Err(ParsingError::AntennaCoordinates))?;
                let z = parse_f64(z.trim()).or(Err(ParsingError::AntennaCoordinates))?;

                if let Some(ant) = &mut rcvr_antenna {
                    *ant = ant.with_base_coordinates((x, y, z));
                } else {
                    rcvr_antenna = Some(Antenna::default().with_base_coordinates((x, y, z)));
                }
            } else if marker.contains("ANTENNA: DELTA H/E/N") {
                // Antenna H/E/N eccentricity components
                let (h, rem) = content.split_at(14);
                let (e, rem) = rem.split_at(14);
                let (n, _) = rem.split_at(14);
                if let Ok(h) = parse_f64(h.trim()) {
                    if let Ok(e) = parse_f64(e.trim()) {
                        if let Ok(n) = parse_f64(n.trim()) {
                            if let Some(a) = &mut rcvr_antenna {
                                *a = a
                                    .with_height(h)
                                    .with_eastern_component(e)
                                    .with_northern_component(n);
                            } else {
                                rcvr_antenna = Some(
                                    Antenna::default()
                                        .with_height(h)
                                        .with_eastern_component(e)
                                        .with_northern_component(n),
                                );
                            }
                        }
                    }
                }
            } else if marker.contains("RCV CLOCK OFFS APPL") {
                let value = content.split_at(20).0.trim();
                let n =
                    i32::from_str_radix(value, 10).or(Err(ParsingError::RcvClockOffsApplied))?;

                observation.clock_offset_applied = n > 0;
            } else if marker.contains("# OF SATELLITES") {
                // ---> we don't need this info,
                //     user can determine it by analyzing the record
            } else if marker.contains("PRN / # OF OBS") {
                // ---> we don't need this info,
                //     user can determine it by analyzing the record
            } else if marker.contains("SYS / PHASE SHIFT") {
                //TODO
            } else if marker.contains("SYS / PVCS APPLIED") {
                // RINEX::ClockData specific
                // + satellite system (G/R/E/C/I/J/S)
                // + programe name to apply Phase Center Variation
                // + source of corrections (url)
                // <o repeated for each satellite system
                // <o blank field when no corrections applied
            } else if marker.contains("TIME OF FIRST OBS") {
                let time_of_first_obs = Self::parse_time_of_obs(content)?;
                observation = observation.with_time_of_first_obs(time_of_first_obs);
            } else if marker.contains("TIME OF LAST OBS") {
                let time_of_last_obs = Self::parse_time_of_obs(content)?;
                observation = observation.with_time_of_last_obs(time_of_last_obs);
            } else if marker.contains("TYPES OF OBS") {
                // these observations can serve both Observation & Meteo RINEX
                if rinex_type == Type::ObservationData {
                    Self::validate_observation_declaration(
                        content,
                        false,
                        version,
                        &mut observation_declarations,
                    )?;
                }
                if !content[..6].trim().is_empty() {
                    observation.codes.clear();
                }
                Self::parse_v2_observables(content, constellation, &mut meteo, &mut observation);
            } else if marker.contains("SYS / # / OBS TYPES") {
                if rinex_type == Type::ObservationData {
                    Self::validate_observation_declaration(
                        content,
                        true,
                        version,
                        &mut observation_declarations,
                    )?;
                }
                if let Ok(system) = Constellation::from_str(&content[..1]) {
                    observation.codes.remove(&system);
                }
                Self::parse_v3_observables(content, &mut current_constell, &mut observation);
            } else if marker.contains("ANALYSIS CENTER") {
                let (code, agency) = content.split_at(3);
                clock = clock.igs(code.trim());
                clock = clock.full_name(agency.trim());
            } else if marker.contains("ANALYSIS CLK REF") {
                let ck = WorkClock::parse(version, content);
                clock = clock.work_clock(ck);
            } else if marker.contains("# / TYPES OF DATA") {
                let (n, r) = content.split_at(6);
                let n = n.trim();
                let n = n.parse::<u8>().or(Err(ParsingError::ClockTypeofData))?;

                let mut rem = r;
                for _ in 0..n {
                    let (code, r) = rem.split_at(6);
                    if let Ok(c) = ClockProfileType::from_str(code.trim()) {
                        clock.codes.push(c);
                    }
                    rem = r;
                }
            } else if marker.contains("STATION NAME / NUM") {
                let (name, domes) = content.split_at(4);
                clock = clock.site(name.trim());
                if let Ok(domes) = DOMES::from_str(domes.trim()) {
                    clock = clock.domes(domes);
                }
            } else if marker.contains("STATION CLK REF") {
                clock = clock.refclock(content.trim());
            } else if marker.contains("SIGNAL STRENGHT UNIT") {
                //TODO
            } else if marker.contains("INTERVAL") {
                let intv_str = content.split_at(20).0.trim();
                if let Ok(interval) = f64::from_str(intv_str) {
                    if interval.is_finite() && interval >= 0.0 {
                        // INTERVAL = '0' may exist, in case
                        // of Varying TEC map intervals
                        sampling_interval =
                            (interval > 0.0).then(|| Duration::from_seconds(interval));
                    }
                }
            } else if marker.contains("COSPAR NUMBER") {
                cospar = Some(COSPAR::from_str(content.trim())?);
            } else if marker.contains("GLONASS SLOT / FRQ #") {
                //TODO
                // This should be used when dealing with Glonass carriers

                let slots = content.split_at(4).1.trim();
                for i in 0..num_integer::div_ceil(slots.len(), 7) {
                    let svnn = &slots[i * 7..i * 7 + 4];
                    let chx = &slots[i * 7 + 4..std::cmp::min(i * 7 + 4 + 3, slots.len())];
                    if let Ok(svnn) = SV::from_str(svnn.trim()) {
                        if let Ok(chx) = chx.trim().parse::<i8>() {
                            glo_channels.insert(svnn, chx);
                        }
                    }
                }
            } else if marker.contains("GLONASS COD/PHS/BIS") {
                //TODO
                // This will help RTK solving against GLONASS SV
            } else if marker.contains("ION ALPHA") {
                // RINEX v2 Ionospheric correction. We tolerate BETA/ALPHA order mixup, as per
                // RINEX v2 standards [https://files.igs.org/pub/data/format/rinex211.txt] paragraph 5.2.
                match IonosphereModel::from_rinex2_header(content, marker) {
                    Ok(IonosphereModel::Klobuchar(KbModel {
                        alpha,
                        beta,
                        region,
                    })) => {
                        // Support GPS|GLO|BDS|GAL|QZSS|SBAS|IRNSS
                        for c in [
                            Constellation::GPS,
                            Constellation::Glonass,
                            Constellation::BeiDou,
                            Constellation::Galileo,
                            Constellation::IRNSS,
                            Constellation::QZSS,
                            Constellation::SBAS,
                        ] {
                            if let Some(correction) = ionod_corrections.get_mut(&c) {
                                // Only Klobuchar models in RINEX2
                                let kb_model = correction.as_klobuchar_mut().unwrap();
                                kb_model.alpha = alpha;
                                kb_model.region = region;
                            } else {
                                ionod_corrections.insert(
                                    c,
                                    IonosphereModel::Klobuchar(KbModel {
                                        alpha,
                                        beta,
                                        region,
                                    }),
                                );
                            }
                        }
                    },
                    _ => {},
                }
            } else if marker.contains("ION BETA") {
                // RINEX v2 Ionospheric correction. We are flexible in their order of appearance,
                // RINEX v2 standards do NOT guarantee that (header fields are free order).
                // [https://files.igs.org/pub/data/format/rinex211.txt] paragraph 5.2.
                match IonosphereModel::from_rinex2_header(content, marker) {
                    Ok(IonosphereModel::Klobuchar(KbModel {
                        alpha,
                        beta,
                        region,
                    })) => {
                        // Support GPS|GLO|BDS|GAL|QZSS|SBAS|IRNSS
                        for c in [
                            Constellation::GPS,
                            Constellation::Glonass,
                            Constellation::BeiDou,
                            Constellation::Galileo,
                            Constellation::IRNSS,
                            Constellation::QZSS,
                            Constellation::SBAS,
                        ] {
                            if let Some(correction) = ionod_corrections.get_mut(&c) {
                                // Only Klobuchar models in RINEX2
                                let kb_model = correction.as_klobuchar_mut().unwrap();
                                kb_model.beta = beta;
                            } else {
                                ionod_corrections.insert(
                                    c,
                                    IonosphereModel::Klobuchar(KbModel {
                                        alpha,
                                        beta,
                                        region,
                                    }),
                                );
                            }
                        }
                    },
                    _ => {},
                }
            } else if marker.contains("IONOSPHERIC CORR") {
                /*
                 * RINEX3 IONOSPHERIC CORRECTION
                 * We support both model in all RINEX2|RINEX3 constellations.
                 * RINEX4 replaces that with actual file content (body) for improved correction accuracy.
                 * The description requires 2 lines when dealing with KB model and we tolerate order mixup.
                 */
                let model_id = content.split_at(5).0;
                if model_id.len() < 3 {
                    /* BAD RINEX */
                    continue;
                }
                let constell_id = &model_id[..3];
                let constell = match constell_id {
                    "GPS" => Constellation::GPS,
                    "GAL" => Constellation::Galileo,
                    "BDS" => Constellation::BeiDou,
                    "QZS" => Constellation::QZSS,
                    "IRN" => Constellation::IRNSS,
                    "GLO" => Constellation::Glonass,
                    _ => continue,
                };
                match IonosphereModel::from_rinex3_header(content) {
                    Ok(IonosphereModel::Klobuchar(KbModel {
                        alpha,
                        beta,
                        region,
                    })) => {
                        // KB requires two lines
                        if let Some(ionod_model) = ionod_corrections.get_mut(&constell) {
                            let kb_model = ionod_model.as_klobuchar_mut().unwrap();
                            if model_id.ends_with('A') {
                                kb_model.alpha = alpha;
                                kb_model.region = region;
                            } else {
                                kb_model.beta = beta;
                            }
                        } else {
                            // latch new model
                            ionod_corrections.insert(
                                constell,
                                IonosphereModel::Klobuchar(KbModel {
                                    alpha,
                                    beta,
                                    region,
                                }),
                            );
                        }
                    },
                    Ok(ion) => {
                        ionod_corrections.insert(constell, ion);
                    },
                    _ => {},
                }
            } else if marker.contains("DELTA-UTC") {
                if let Ok(time_offset) = TimeOffset::parse_v2_delta_utc(content) {
                    nav = nav.with_time_offset(time_offset);
                }
            } else if marker.contains("CORR TO SYSTEM TIME") {
                if let Ok(time_offset) = TimeOffset::parse_v2_corr_to_system_time(content) {
                    nav = nav.with_time_offset(time_offset);
                }
            } else if marker.contains("TIME SYSTEM CORR") {
                if let Ok(time_offset) = TimeOffset::parse_v3(content) {
                    nav = nav.with_time_offset(time_offset);
                }
            } else if marker.contains("TIME SYSTEM ID") {
                let timescale = content.trim();
                let ts = TimeScale::from_str(timescale)?;
                clock = clock.timescale(ts);
            }
        }

        for (expected, codes) in observation_declarations {
            if codes.len() != expected || codes.iter().collect::<BTreeSet<_>>().len() != codes.len()
            {
                return Err(ParsingError::BadObservable);
            }
        }

        Ok(Header {
            version,
            rinex_type,
            constellation,
            comments,
            program,
            run_by,
            date,
            geodetic_marker,
            agency,
            observer,
            license,
            doi,
            station_url,
            rcvr,
            cospar,
            glo_channels,
            leap,
            rx_position,
            ionod_corrections,
            dcb_compensations,
            pcv_compensations,
            wavelengths: base.and_then(|h| h.wavelengths),
            sampling_interval,
            rcvr_antenna,
            sv_antenna,
            // RINEX specific
            obs: {
                if rinex_type == Type::ObservationData {
                    Some(observation)
                } else {
                    None
                }
            },
            nav: {
                if rinex_type == Type::NavigationData {
                    Some(nav)
                } else {
                    None
                }
            },
            meteo: {
                if rinex_type == Type::MeteoData {
                    Some(meteo)
                } else {
                    None
                }
            },
            clock: {
                if rinex_type == Type::ClockData {
                    Some(clock)
                } else {
                    None
                }
            },
            antex: {
                if rinex_type == Type::AntennaData {
                    Some(antex)
                } else {
                    None
                }
            },
        })
    }

    fn parse_time_of_obs(content: &str) -> Result<Epoch, ParsingError> {
        let (_, rem) = content.split_at(2);
        let (y, rem) = rem.split_at(4);
        let (m, rem) = rem.split_at(6);
        let (d, rem) = rem.split_at(6);
        let (hh, rem) = rem.split_at(6);
        let (mm, rem) = rem.split_at(6);
        let (ss, rem) = rem.split_at(5);
        let (_dot, rem) = rem.split_at(1);
        let (fraction, rem) = rem.split_at(8);

        // println!("Y \"{}\" M \"{}\" D \"{}\" HH \"{}\" MM \"{}\" SS \"{}\" NS \"{}\"", y, m, d, hh, mm, ss, ns); // DEBUG
        let year = y.trim();
        let mut y = year
            .parse::<u32>()
            .map_err(|_| ParsingError::DatetimeParsing)?;

        // handle OLD RINEX problem
        if year.len() <= 2 && year.bytes().all(|b| b.is_ascii_digit()) {
            y += if y >= 80 { 1900 } else { 2000 };
        }
        if y == 0 {
            return Err(ParsingError::DatetimeParsing);
        }

        let m = m
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DatetimeParsing)?;

        let d = d
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DatetimeParsing)?;

        let hh = hh
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DatetimeParsing)?;

        let mm = mm
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DatetimeParsing)?;

        let ss = ss
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DatetimeParsing)?;

        // fractional seconds: F13.7 gives seven digits, older files
        // may give fewer. Scale the digits to nanoseconds.
        let fraction = fraction.trim();
        let fraction = &fraction[..fraction.len().min(9)];

        let ns = fraction
            .parse::<u32>()
            .map_err(|_| ParsingError::DatetimeParsing)?
            * 10_u32.pow((9 - fraction.len()) as u32);

        /*
         * We set TAI as "default" Timescale.
         * Timescale might be omitted in Old RINEX formats,
         * In this case, we exit with "TAI" and handle that externally.
         */
        let mut ts = TimeScale::TAI;
        let rem = rem.trim();

        /*
         * Handles DORIS measurement special case,
         * offset from TAI, that we will convert back to TAI later
         */
        if !rem.is_empty() && rem != "DOR" {
            ts = TimeScale::from_str(rem.trim())?;
        }

        Epoch::from_str(&format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09} {}",
            y, m, d, hh, mm, ss, ns, ts
        ))
        .map_err(|_| ParsingError::DatetimeParsing)
    }

    // Validate OBS declarations in the common header decoder before unknown
    // tokens can be discarded and shift the observation column assignments.
    fn validate_observation_declaration(
        content: &str,
        modern: bool,
        version: Version,
        declared: &mut Vec<(usize, Vec<Observable>)>,
    ) -> Result<(), ParsingError> {
        if modern != (version.major >= 3) {
            return Err(ParsingError::BadObservable);
        }
        let count = if modern {
            if !content[..1].trim().is_empty() {
                Constellation::from_str(&content[..1]).map_err(|_| ParsingError::BadObservable)?;
                Some(content[3..6].trim())
            } else {
                None
            }
        } else if !content[..6].trim().is_empty() {
            Some(content[..6].trim())
        } else {
            None
        };
        if let Some(count) = count {
            let count = count
                .parse::<usize>()
                .map_err(|_| ParsingError::BadObservable)?;
            if count == 0 {
                return Err(ParsingError::BadObservable);
            }
            declared.push((count, Vec::new()));
        }
        let (_, codes) = declared.last_mut().ok_or(ParsingError::BadObservable)?;
        for name in content[6..].split_ascii_whitespace() {
            // Observable also parses METEO/DORIS tokens and intentionally does
            // not validate GNSS code syntax. Check the OBS context first.
            let code = name.as_bytes();
            let valid_shape = if modern {
                matches!(code, [b'C' | b'L' | b'D' | b'S', b'1'..=b'9', b'A'..=b'Z'])
                    // Unknown attributes were allowed before RINEX 3.02.
                    || (version < Version::new(3, 2)
                        && matches!(code, [b'C' | b'L' | b'D' | b'S', b'1'..=b'9']))
            } else {
                matches!(code, [b'C' | b'P' | b'L' | b'D' | b'S', b'1'..=b'9'])
            };
            if !valid_shape {
                return Err(ParsingError::BadObservable);
            }
            codes.push(Observable::from_str(name).map_err(|_| ParsingError::BadObservable)?);
        }
        Ok(())
    }

    /// Parse list of [Observable]s which applies to both METEO and OBS RINEX
    pub(crate) fn parse_v2_observables(
        line: &str,
        constell: Option<Constellation>,
        meteo: &mut MeteoHeader,
        observation: &mut ObservationHeader,
    ) {
        lazy_static! {
            /*
             *  We support GPS, Glonass, Galileo, SBAS and BDS as per v2.11.
             */
            static ref KNOWN_V2_CONSTELLS: [Constellation; 5] = [
                Constellation::GPS,
                Constellation::SBAS,
                Constellation::Glonass,
                Constellation::Galileo,
                Constellation::BeiDou,
            ];
        }
        let line = line.split_at(6).1;
        for item in line.split_ascii_whitespace() {
            if let Ok(obs) = Observable::from_str(item.trim()) {
                match constell {
                    Some(Constellation::Mixed) => {
                        for constell in KNOWN_V2_CONSTELLS.iter() {
                            if let Some(codes) = observation.codes.get_mut(constell) {
                                codes.push(obs.clone());
                            } else {
                                observation.codes.insert(*constell, vec![obs.clone()]);
                            }
                        }
                    },
                    Some(c) => {
                        if let Some(codes) = observation.codes.get_mut(&c) {
                            codes.push(obs.clone());
                        } else {
                            observation.codes.insert(c, vec![obs.clone()]);
                        }
                    },
                    None => meteo.codes.push(obs),
                }
            }
        }
    }

    /// Parse list of [Observable]s which applies to both METEO and OBS RINEX
    fn parse_v3_observables(
        line: &str,
        current_constell: &mut Option<Constellation>,
        observation: &mut ObservationHeader,
    ) {
        let (possible_counter, items) = line.split_at(6);
        if !possible_counter.is_empty() {
            let code = &possible_counter[..1];
            if let Ok(c) = Constellation::from_str(code) {
                *current_constell = Some(c);
            }
        }
        if let Some(constell) = current_constell {
            // system correctly identified
            for item in items.split_ascii_whitespace() {
                if let Ok(observable) = Observable::from_str(item) {
                    if let Some(codes) = observation.codes.get_mut(constell) {
                        codes.push(observable);
                    } else {
                        observation.codes.insert(*constell, vec![observable]);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::{Epoch, Header, TimeScale};
    use std::str::FromStr;

    #[test]
    fn parse_time_of_obs() {
        let content = "  2021    12    21     0     0    0.0000000     GPS";
        let parsed = Header::parse_time_of_obs(&content).unwrap();
        assert_eq!(parsed, Epoch::from_str("2021-12-21T00:00:00 GPST").unwrap());

        let content = "  1995    01    01    00    00   00.000000             ";
        let parsed = Header::parse_time_of_obs(&content).unwrap();
        assert_eq!(parsed, Epoch::from_str("1995-01-01T00:00:00 TAI").unwrap());

        // fractional seconds (F13.7): the digits were read as a plain
        // integer and formatted over eight positions, so .9890000 became
        // 0.0989 s
        let content = "  2024     9    20     8    17   50.9890000     GPS";
        let parsed = Header::parse_time_of_obs(&content).unwrap();
        assert_eq!(
            parsed,
            Epoch::from_str("2024-09-20T08:17:50.989 GPST").unwrap()
        );

        let content = "  2024     9    20     8    17   50.0000001     GPS";
        let parsed = Header::parse_time_of_obs(&content).unwrap();
        assert_eq!(
            parsed,
            Epoch::from_str("2024-09-20T08:17:50.0000001 GPST").unwrap()
        );

        for (year, expected) in [("79", 2079), ("80", 1980), ("0079", 79)] {
            let content = format!("{year:>6}     1     1     0     0    0.0000000     GPS");
            assert_eq!(
                Header::parse_time_of_obs(&content)
                    .unwrap()
                    .to_gregorian(TimeScale::GPST)
                    .0,
                expected
            );
        }
        assert!(
            Header::parse_time_of_obs("  0000     1     1     0     0    0.0000000     GPS")
                .is_err()
        );
    }
}
