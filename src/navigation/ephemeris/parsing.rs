use crate::{
    epoch::parse_in_timescale as parse_epoch_in_timescale,
    navigation::{
        ephemeris::orbits::{closest_nav_standards, OrbitItem},
        timescale, Ephemeris, NavMessageType,
    },
    parse_f64,
    prelude::{Constellation, Epoch, ParsingError, TimeScale, Version, SV},
};

use std::{collections::HashMap, str::Lines};

/// Parses all orbital elements.
/// Descriptor is retrieved from database db/NAV/orbits.
/// Empty and spare fields are omitted. Every other field must decode, including
/// flags: a mapping or unsupported-definition error rejects the whole block.
/// ## Inputs
/// - version: database [Version] filter
/// - msgtype: database [NavMessageType] filter
/// - constellation: database [Constellation] filter
fn parse_orbits(
    version: Version,
    msgtype: NavMessageType,
    constell: Constellation,
    lines: Lines<'_>,
) -> Result<HashMap<String, OrbitItem>, ParsingError> {
    // convert SBAS constell to compatible "sbas" (undetermined/general constell)
    let constell = match constell.is_sbas() {
        true => Constellation::SBAS,
        false => constell,
    };

    // Determine data fields to parse, from database
    let nav_standards = match closest_nav_standards(constell, version, msgtype) {
        Some(v) => v,
        _ => return Err(ParsingError::NoNavigationDefinition),
    };

    // println!("FIELD : {:?} \n", nav_standards.items); // DEBUG

    let fields = &nav_standards.items;

    let mut key_index: usize = 0;
    let word_size: usize = 19;
    let mut map: HashMap<String, OrbitItem> = HashMap::new();

    for line in lines {
        // trim first few white spaces
        let padding = if version.major < 3 { 3 } else { 4 };
        let mut line = line.get(padding..).unwrap_or("");

        // number of fields found on this line, blank or not
        let mut nb_fields = 0;

        loop {
            if line.is_empty() {
                // fields omitted at the end of the line
                key_index += 4_usize.saturating_sub(nb_fields);
                break;
            }

            let (val_str, rem) = line.split_at(std::cmp::min(word_size, line.len()));
            let val_str = val_str.trim();
            nb_fields += 1;

            // handle omitted fields
            if val_str.is_empty() {
                // omitted field
                key_index += 1;
                line = rem;
                continue;
            }

            if let Some((name_str, type_str)) = fields.get(key_index) {
                //println!(
                //    "Key \"{}\"(index: {}) | Token \"{}\" | Content \"{}\"",
                //    key,
                //    key_index,
                //    token,
                //    content.trim()
                //); //DEBUG
                // Spare fields must be skipped (RINEX section 6.4).
                if !name_str.starts_with("spare") {
                    let item = OrbitItem::new(name_str, type_str, val_str, &msgtype, constell)
                        .map_err(|source| ParsingError::NavOrbitParsing {
                            field: name_str.to_string(),
                            source: Box::new(source),
                        })?;
                    map.insert(name_str.to_string(), item);
                }
            }

            key_index += 1;
            line = rem;
        }
    }
    Ok(map)
}

impl Ephemeris {
    /// [Ephemeris] parsing attempt, from V<4 content.
    pub(crate) fn parse_v2v3(
        version: Version,
        constellation: Constellation,
        mut lines: Lines<'_>,
    ) -> Result<(Epoch, SV, Self), ParsingError> {
        let line = match lines.next() {
            Some(l) => l,
            _ => return Err(ParsingError::EmptyEpoch),
        };

        let svnn_offset: usize = match version.major < 3 {
            true => 3,
            false => 4,
        };

        let (svnn, rem) = line.split_at(svnn_offset);
        let (date, rem) = rem.split_at(19);
        let (clk_bias, rem) = rem.split_at(19);
        let (clk_dr, clk_drr) = rem.split_at(19);

        let sv = match svnn.trim().parse::<SV>() {
            Ok(sv) => sv,
            Err(_) => {
                // In very old RINex, it is possible to wind up here
                // when constellation is omitted. Yet this is only tolerated
                // in mono constellation contexts, MIXED context are not supported
                // to exist in old revision anyway. We let the possibility to fail
                // on incorrect content here.
                let desc = format!("{:x}{:02}", constellation, svnn.trim());
                desc.parse::<SV>()?
            },
        };

        let ts = timescale(sv.constellation)?;

        let epoch = parse_epoch_in_timescale(date.trim(), ts)?;

        let clock_bias = parse_f64(clk_bias.trim()).map_err(|_| ParsingError::ClockParsing)?;

        let clock_drift = parse_f64(clk_dr.trim()).map_err(|_| ParsingError::ClockParsing)?;

        let mut clock_drift_rate =
            parse_f64(clk_drr.trim()).map_err(|_| ParsingError::ClockParsing)?;

        if ![clock_bias, clock_drift, clock_drift_rate]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err(ParsingError::ClockParsing);
        }

        // parse orbits :
        //  only Legacy Frames in V2 and V3 (old) RINEX
        let mut orbits = parse_orbits(version, NavMessageType::LNAV, sv.constellation, lines)?;

        if sv.constellation == Constellation::Glonass {
            // Legacy GLONASS is FDMA: the third scalar is message frame time,
            // not the quadratic clock coefficient. Its units are UTC-day
            // seconds in RINEX 2, UTC-week seconds in RINEX 3.
            orbits.insert("frameTime".to_string(), OrbitItem::F64(clock_drift_rate));
            clock_drift_rate = 0.0;
        }

        if sv.constellation.is_sbas() {
            // RINEX SBAS third scalar: transmission time in GPS week seconds,
            // including fractional and week-boundary-adjusted values.
            orbits.insert("t_tm".to_string(), OrbitItem::F64(clock_drift_rate));
            clock_drift_rate = 0.0;
        }

        Ok((
            epoch,
            sv,
            Self {
                clock_bias,
                clock_drift,
                clock_drift_rate,
                orbits,
            },
        ))
    }

    /// Parse Ephemeris (V4) from line iterator
    pub(crate) fn parse_v4(
        msg: NavMessageType,
        mut lines: Lines<'_>,
        ts: TimeScale,
    ) -> Result<(Epoch, SV, Self), ParsingError> {
        let line = match lines.next() {
            Some(l) => l,
            _ => return Err(ParsingError::EmptyEpoch),
        };

        let (svnn, rem) = line.split_at(4);
        let sv = svnn.trim().parse::<SV>()?;
        let (epoch, rem) = rem.split_at(19);
        let epoch = parse_epoch_in_timescale(epoch.trim(), ts)?;

        let (clk_bias, rem) = rem.split_at(19);
        let (clk_dr, clk_drr) = rem.split_at(19);

        let clock_bias = parse_f64(clk_bias.trim()).map_err(|_| ParsingError::ClockParsing)?;

        let clock_drift = parse_f64(clk_dr.trim()).map_err(|_| ParsingError::ClockParsing)?;

        let mut clock_drift_rate =
            parse_f64(clk_drr.trim()).map_err(|_| ParsingError::ClockParsing)?;

        if ![clock_bias, clock_drift, clock_drift_rate]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err(ParsingError::ClockParsing);
        }

        let mut orbits =
            parse_orbits(Version { major: 4, minor: 0 }, msg, sv.constellation, lines)?;

        if sv.constellation == Constellation::Glonass && msg == NavMessageType::FDMA {
            // RINEX 4 FDMA: seconds of UTC week (table A15).
            orbits.insert("frameTime".to_string(), OrbitItem::F64(clock_drift_rate));
            clock_drift_rate = 0.0;
        }

        if sv.constellation.is_sbas() {
            // RINEX 4 table A28: transmission time, never a week number.
            orbits.insert("t_tm".to_string(), OrbitItem::F64(clock_drift_rate));
            clock_drift_rate = 0.0;
        }

        Ok((
            epoch,
            sv,
            Self {
                clock_bias,
                clock_drift,
                clock_drift_rate,
                orbits,
            },
        ))
    }
}

#[cfg(test)]
mod test {
    use crate::{
        navigation::{Ephemeris, NavMessageType, OrbitItem},
        prelude::{Constellation, Epoch, TimeScale, Version, SV},
    };

    use std::str::FromStr;

    #[test]
    fn zero_fields_and_non_numeric_spares_remain_distinct() {
        // Explicitly synthetic BDS legacy row 5: IDOT, spare, week, spare.
        let mut content = format!("{}\n", " ".repeat(80)).repeat(4);
        content.push_str(&format!(
            "    {:>19}{:>19}{:>19}{:>19}\n",
            "0.0", "FUTURE", "0.0", "FUTURE"
        ));
        let orbits = parse_orbits(
            Version::new(3, 0),
            NavMessageType::LNAV,
            Constellation::BeiDou,
            content.lines(),
        )
        .unwrap();
        assert_eq!(orbits.get("idot"), Some(&OrbitItem::F64(0.0)));
        assert_eq!(orbits.get("week"), Some(&OrbitItem::U32(0)));
        assert_eq!(orbits.len(), 2);
    }

    use super::parse_orbits;

    // fn build_orbits(
    //     constellation: Constellation,
    //     descriptor: Vec<(&str, &str)>,
    // ) -> HashMap<String, OrbitItem> {
    //     let mut map: HashMap<String, OrbitItem> = HashMap::with_capacity(descriptor.len());
    //     for (key, value) in descriptor.iter() {
    //         if key.contains("week") {
    //             map.insert(
    //                 key.to_string(),
    //                 OrbitItem::new("u32", value, constellation).unwrap(),
    //             );
    //         } else {
    //             map.insert(
    //                 key.to_string(),
    //                 OrbitItem::new("f64", value, constellation).unwrap(),
    //             );
    //         }
    //     }
    //     map
    // }

    #[test]
    fn gal_orbit() {
        let content =
            "     7.500000000000e+01 1.478125000000e+01 2.945479833915e-09-3.955466341850e-01
     8.065253496170e-07 3.683507675305e-04-3.911554813385e-07 5.440603218079e+03
     3.522000000000e+05-6.519258022308e-08 2.295381450845e+00 7.450580596924e-09
     9.883726443393e-01 3.616875000000e+02 2.551413130998e-01-5.907746081337e-09
     1.839362331110e-10 2.580000000000e+02 2.111000000000e+03                   
     3.120000000000e+00 0.000000000000e+00-1.303851604462e-08 0.000000000000e+00
     3.555400000000e+05";
        let orbits = parse_orbits(
            Version::new(3, 0),
            NavMessageType::LNAV,
            Constellation::Galileo,
            content.lines(),
        );
        assert!(orbits.is_ok());
        let orbits = orbits.unwrap();
        let ephemeris = Ephemeris {
            clock_bias: 0.0,
            clock_drift: 0.0,
            clock_drift_rate: 0.0,
            orbits,
        };
        assert_eq!(ephemeris.get_orbit_f64("iodnav"), Some(7.500000000000e+01));
        assert_eq!(ephemeris.get_orbit_f64("crs"), Some(1.478125000000e+01));
        assert_eq!(ephemeris.get_orbit_f64("deltaN"), Some(2.945479833915e-09));
        assert_eq!(ephemeris.get_orbit_f64("m0"), Some(-3.955466341850e-01));

        assert_eq!(ephemeris.get_orbit_f64("cuc"), Some(8.065253496170e-07));
        assert_eq!(ephemeris.get_orbit_f64("e"), Some(3.683507675305e-04));
        assert_eq!(ephemeris.get_orbit_f64("cus"), Some(-3.911554813385e-07));
        assert_eq!(ephemeris.get_orbit_f64("sqrta"), Some(5.440603218079e+03));

        assert_eq!(ephemeris.get_orbit_f64("toe"), Some(3.522000000000e+05));
        assert_eq!(ephemeris.get_orbit_f64("cic"), Some(-6.519258022308e-08));
        assert_eq!(ephemeris.get_orbit_f64("omega0"), Some(2.295381450845e+00));
        assert_eq!(ephemeris.get_orbit_f64("cis"), Some(7.450580596924e-09));

        assert_eq!(ephemeris.get_orbit_f64("i0"), Some(9.883726443393e-01));
        assert_eq!(ephemeris.get_orbit_f64("crc"), Some(3.616875000000e+02));
        assert_eq!(ephemeris.get_orbit_f64("omega"), Some(2.551413130998e-01));
        assert_eq!(
            ephemeris.get_orbit_f64("omegaDot"),
            Some(-5.907746081337e-09)
        );

        assert_eq!(ephemeris.get_orbit_f64("idot"), Some(1.839362331110e-10));
        assert_eq!(ephemeris.get_orbit_f64("source"), Some(2.580000000000e+02));
        assert_eq!(ephemeris.get_week(), Some(2111));

        assert_eq!(ephemeris.get_orbit_f64("sisa"), Some(3.120000000000e+00));
        //assert_eq!(ephemeris.get_orbit_f64("health"), Some(0.000000000000e+00));
        assert_eq!(
            ephemeris.get_orbit_f64("bgdE5aE1"),
            Some(-1.303851604462e-08)
        );
        assert_eq!(ephemeris.get_orbit_f64("bgdE5bE1"), Some(0.0));

        assert_eq!(ephemeris.get_orbit_f64("t_tm"), Some(3.555400000000e+05));
    }

    #[test]
    fn bds_orbit() {
        let content =
            "      .100000000000e+01  .118906250000e+02  .105325815814e-08 -.255139531119e+01
      .169500708580e-06  .401772442274e-03  .292365439236e-04  .649346986580e+04
      .432000000000e+06  .105705112219e-06 -.277512444499e+01 -.211410224438e-06
      .607169709798e-01 -.897671875000e+03  .154887266488e+00 -.871464871438e-10
     -.940753471872e-09  .000000000000e+00  .782000000000e+03  .000000000000e+00
      .200000000000e+01  .000000000000e+00 -.599999994133e-09 -.900000000000e-08
      .432000000000e+06  .000000000000e+00 0.000000000000e+00 0.000000000000e+00";

        let orbits = parse_orbits(
            Version::new(3, 0),
            NavMessageType::LNAV,
            Constellation::BeiDou,
            content.lines(),
        );

        assert!(orbits.is_ok());
        let orbits = orbits.unwrap();

        let ephemeris = Ephemeris {
            clock_bias: 0.0,
            clock_drift: 0.0,
            clock_drift_rate: 0.0,
            orbits,
        };

        assert_eq!(ephemeris.get_orbit_f64("aode"), Some(1.0));
        assert_eq!(ephemeris.get_orbit_f64("crs"), Some(1.18906250000e+01));
        assert_eq!(ephemeris.get_orbit_f64("deltaN"), Some(0.105325815814e-08));
        assert_eq!(ephemeris.get_orbit_f64("m0"), Some(-0.255139531119e+01));

        assert_eq!(ephemeris.get_orbit_f64("cuc"), Some(0.169500708580e-06));
        assert_eq!(ephemeris.get_orbit_f64("e"), Some(0.401772442274e-03));
        assert_eq!(ephemeris.get_orbit_f64("cus"), Some(0.292365439236e-04));
        assert_eq!(ephemeris.get_orbit_f64("sqrta"), Some(0.649346986580e+04));

        assert_eq!(ephemeris.get_orbit_f64("toe"), Some(0.432000000000e+06));
        assert_eq!(ephemeris.get_orbit_f64("cic"), Some(0.105705112219e-06));
        assert_eq!(ephemeris.get_orbit_f64("omega0"), Some(-0.277512444499e+01));
        assert_eq!(ephemeris.get_orbit_f64("cis"), Some(-0.211410224438e-06));

        assert_eq!(ephemeris.get_orbit_f64("i0"), Some(0.607169709798e-01));
        assert_eq!(ephemeris.get_orbit_f64("crc"), Some(-0.897671875000e+03));
        assert_eq!(ephemeris.get_orbit_f64("omega"), Some(0.154887266488e+00));
        assert_eq!(
            ephemeris.get_orbit_f64("omegaDot"),
            Some(-0.871464871438e-10)
        );

        assert_eq!(ephemeris.get_orbit_f64("idot"), Some(-0.940753471872e-09));
        assert_eq!(ephemeris.get_week(), Some(782));
        assert_eq!(
            ephemeris.get_orbit_f64("accuracy"),
            Some(0.200000000000e+01)
        );
        assert!(ephemeris.get_orbit_f64("health").is_some());

        assert_eq!(
            ephemeris.get_orbit_f64("tgd1b1b3"),
            Some(-0.599999994133e-09)
        );

        assert_eq!(
            ephemeris.get_orbit_f64("tgd2b2b3"),
            Some(-0.900000000000e-08)
        );

        assert_eq!(ephemeris.get_orbit_f64("aodc"), Some(0.0));
        assert_eq!(ephemeris.get_orbit_f64("t_tm"), Some(0.432000000000e+06));
    }

    #[test]
    fn glonass_orbit_v2() {
        let content =
            "   -1.488799804690D+03-2.196182250980D+00 3.725290298460D-09 0.000000000000D+00
    1.292880712890D+04-2.049269676210D+00 0.000000000000D+00 1.000000000000D+00
    2.193169775390D+04 1.059645652770D+00-9.313225746150D-10 0.000000000000D+00";
        let orbits = parse_orbits(
            Version::new(2, 0),
            NavMessageType::LNAV,
            Constellation::Glonass,
            content.lines(),
        );
        assert!(orbits.is_ok(), "failed to parse Glonass V2 orbits");
        let orbits = orbits.unwrap();
        let ephemeris = Ephemeris {
            clock_bias: 0.0,
            clock_drift: 0.0,
            clock_drift_rate: 0.0,
            orbits,
        };
        assert_eq!(ephemeris.get_orbit_f64("satPosX"), Some(-1.488799804690E3));
        assert_eq!(ephemeris.get_orbit_f64("satPosY"), Some(1.292880712890E4));
        assert_eq!(ephemeris.get_orbit_f64("satPosZ"), Some(2.193169775390E4));

        let content =
            "    1.817068505860D+04 8.184757232670D-01 2.793967723850D-09 0.000000000000D+00
    1.594814404300D+04 7.562503814700D-01 9.313225746150D-10 5.000000000000D+00
    8.090271484380D+03-3.345663070680D+00-2.793967723850D-09 0.000000000000D+00";

        let orbits = parse_orbits(
            Version::new(2, 0),
            NavMessageType::LNAV,
            Constellation::Glonass,
            content.lines(),
        );

        assert!(orbits.is_ok(), "failed to parse Glonass V2 orbits");
        let orbits = orbits.unwrap();

        let ephemeris = Ephemeris {
            clock_bias: 0.0,
            clock_drift: 0.0,
            clock_drift_rate: 0.0,
            orbits,
        };

        assert_eq!(ephemeris.get_orbit_f64("channel"), Some(5.0));
    }

    #[test]
    fn glonass_orbit_v3() {
        let content =
            "      .783916601562e+04 -.423131942749e+00  .931322574615e-09  .000000000000e+00
     -.216949155273e+05  .145034790039e+01  .279396772385e-08  .300000000000e+01
      .109021518555e+05  .319181251526e+01  .000000000000e+00  .000000000000e+00";
        let orbits = parse_orbits(
            Version::new(3, 0),
            NavMessageType::LNAV,
            Constellation::Glonass,
            content.lines(),
        );
        assert!(orbits.is_ok(), "failed to parse Glonass V3 orbits");
        let orbits = orbits.unwrap();
        let ephemeris = Ephemeris {
            clock_bias: 0.0,
            clock_drift: 0.0,
            clock_drift_rate: 0.0,
            orbits,
        };
        assert_eq!(ephemeris.get_orbit_f64("satPosX"), Some(0.783916601562E4));
        assert_eq!(ephemeris.get_orbit_f64("satPosY"), Some(-0.216949155273E5));
        assert_eq!(ephemeris.get_orbit_f64("satPosZ"), Some(0.109021518555E5));
    }

    #[test]
    fn glonass_orbit_v2_missing_fields() {
        let content =
            "   -1.488799804690D+03                    3.725290298460D-09 0.000000000000D+00
    1.292880712890D+04-2.049269676210D+00 0.000000000000D+00 1.000000000000D+00
    2.193169775390D+04 1.059645652770D+00-9.313225746150D-10 0.000000000000D+00";
        let orbits = parse_orbits(
            Version::new(2, 0),
            NavMessageType::LNAV,
            Constellation::Glonass,
            content.lines(),
        );
        assert!(orbits.is_ok(), "failed to parse Glonass V2 orbits");
        let orbits = orbits.unwrap();
        let ephemeris = Ephemeris {
            clock_bias: 0.0,
            clock_drift: 0.0,
            clock_drift_rate: 0.0,
            orbits,
        };
        assert_eq!(ephemeris.get_orbit_f64("satPosX"), Some(-1.488799804690E3));
        assert_eq!(ephemeris.get_orbit_f64("velX"), None);
        assert_eq!(ephemeris.get_orbit_f64("satPosY"), Some(1.292880712890E4));
        assert_eq!(ephemeris.get_orbit_f64("satPosZ"), Some(2.193169775390E4));
    }

    #[test]
    fn glonass_orbit_v3_missing_fields() {
        let content =
            "      .783916601562e+04                    .931322574615e-09  .000000000000e+00
     -.216949155273e+05  .145034790039e+01  .279396772385e-08  .300000000000e+01
      .109021518555e+05  .319181251526e+01  .000000000000e+00  .000000000000e+00";

        let orbits = parse_orbits(
            Version::new(3, 0),
            NavMessageType::LNAV,
            Constellation::Glonass,
            content.lines(),
        );
        assert!(orbits.is_ok(), "failed to parse Glonass V3 orbits");
        let orbits = orbits.unwrap();
        let ephemeris = Ephemeris {
            clock_bias: 0.0,
            clock_drift: 0.0,
            clock_drift_rate: 0.0,
            orbits,
        };
        assert_eq!(ephemeris.get_orbit_f64("satPosX"), Some(0.783916601562E4));
        assert_eq!(ephemeris.get_orbit_f64("velX"), None);
        assert_eq!(ephemeris.get_orbit_f64("satPosY"), Some(-0.216949155273E5));
        assert_eq!(ephemeris.get_orbit_f64("satPosZ"), Some(0.109021518555E5));
    }

    #[test]
    fn blank_field_on_short_line() {
        // A blank field followed by omitted trailing fields on the same
        // line must not shift the fields of the following lines.
        // GPS LNAV V4 line 5: idot, (blank l2 codes), week, omitted l2p flag
        let content =
            "     9.800000000000e+01-1.718750000000e+00 4.639836124941e-09 2.148941747752e+00
    -1.881271600723e-07 3.355251392350e-04 8.245930075645e-06 5.153800453186e+03
     3.600000000000e+05-1.676380634308e-08 5.171400020311e-01 1.490116119385e-08
     9.601921900531e-01 2.187187500000e+02-1.736906885738e+00-8.044977962767e-09
    -2.932264997750e-10                    2.044000000000e+03
     4.000000000000e+00 6.300000000000e+01-8.847564458847e-09 8.660000000000e+02
     3.553500000000e+05 4.000000000000e+00";
        let orbits = parse_orbits(
            Version::new(4, 0),
            NavMessageType::LNAV,
            Constellation::GPS,
            content.lines(),
        )
        .unwrap();
        let ephemeris = Ephemeris {
            clock_bias: 0.0,
            clock_drift: 0.0,
            clock_drift_rate: 0.0,
            orbits,
        };
        assert_eq!(ephemeris.get_orbit_f64("idot"), Some(-2.932264997750e-10));
        assert_eq!(ephemeris.get_orbit_f64("l2Codes"), None);
        assert_eq!(ephemeris.get_week(), Some(2044));
        assert_eq!(ephemeris.get_orbit_f64("l2p"), None);
        assert_eq!(ephemeris.get_orbit_f64("accuracy"), Some(4.0));
        assert_eq!(ephemeris.get_orbit_f64("tgd"), Some(-8.847564458847e-09));
        assert_eq!(ephemeris.get_orbit_f64("iodc"), Some(866.0));
        assert_eq!(ephemeris.get_orbit_f64("t_tm"), Some(3.553500000000e+05));
        assert_eq!(ephemeris.get_orbit_f64("fitInt"), Some(4.0));
    }

    #[test]
    fn navic_lnav_v4() {
        // RINEX 4.02 Table A32, in GPST (Table A30)
        let content =
            "I02 2020 09 15 02 05 36 6.225099787116e-04 1.773514668457e-11 0.000000000000e+00
     1.690000000000e+02-5.793750000000e+02 4.834487090078e-09-4.281979621524e-01
    -1.904368400574e-05 2.015684265643e-03-3.430992364883e-06 6.493289550781e+03
     1.803360000000e+05 2.495944499969e-07-1.337499015334e+00 7.450580596924e-08
     5.022043764738e-01 1.946250000000e+02-2.970970345572e+00-4.461614415577e-09
    -9.578970431139e-10                    2.123000000000e+03
     2.000000000000e+00 0.000000000000e+00-1.862645149231e-09
     1.804920000000e+05";
        let (epoch, sv, eph) =
            Ephemeris::parse_v4(NavMessageType::LNAV, content.lines(), TimeScale::GPST).unwrap();
        assert_eq!(sv, SV::from_str("I02").unwrap());
        assert_eq!(epoch, Epoch::from_str("2020-09-15T02:05:36 GPST").unwrap());
        assert_eq!(eph.clock_bias, 6.225099787116e-04);
        assert_eq!(eph.get_orbit_f64("iodec"), Some(169.0));
        assert_eq!(eph.get_orbit_f64("sqrta"), Some(6.493289550781e+03));
        assert_eq!(eph.get_week(), Some(2123));
        assert_eq!(eph.get_orbit_f64("accuracy"), Some(2.0));
        assert_eq!(eph.get_orbit_f64("health"), Some(0.0));
        assert_eq!(eph.get_orbit_f64("tgd"), Some(-1.862645149231e-09));
        assert_eq!(eph.get_orbit_f64("t_tm"), Some(1.804920000000e+05));
    }

    #[test]
    fn navic_l1nv_v4() {
        // RINEX 4.02 Table A32
        let content =
            "I10 2023 06 24 00 05 00 1.527369022369e-07 1.364242052659e-12 0.000000000000e+00
     0.000000000000e+00-2.593125000000e+02 7.028864208979e-09 2.300305834983e+00
    -8.691102266312e-06 4.531537415460e-04-3.855675458908e-06 6.493495117188e+03
     7.000000000000e+00 1.341104507446e-07 1.359342162629e-01-7.078051567078e-08
     8.594830530333e-02 1.214375000000e+02-5.136403830694e-02-5.858815471753e-09
    -4.846630453041e-10 0.000000000000e+00                    1.000000000000e+00
     1.500000000000e+01 0.000000000000e+00                   -3.608874976635e-09
                                           6.984919309616e-09 5.995389074087e-09
     5.191380000000e+05";
        let (epoch, sv, eph) =
            Ephemeris::parse_v4(NavMessageType::L1NV, content.lines(), TimeScale::GPST).unwrap();
        assert_eq!(sv, SV::from_str("I10").unwrap());
        assert_eq!(epoch, Epoch::from_str("2023-06-24T00:05:00 GPST").unwrap());
        assert_eq!(eph.get_orbit_f64("crs"), Some(-2.593125000000e+02));
        assert_eq!(eph.get_orbit_f64("iodec"), Some(7.0));
        assert_eq!(eph.get_orbit_f64("rsf"), Some(1.0));
        assert_eq!(eph.get_orbit_f64("urai"), Some(15.0));
        assert_eq!(eph.get_orbit_f64("health"), Some(0.0));
        assert_eq!(eph.get_orbit_f64("tgdL1PL5"), None);
        assert_eq!(eph.get_orbit_f64("tgdSL5"), Some(-3.608874976635e-09));
        assert_eq!(eph.get_orbit_f64("iscL1PS"), Some(6.984919309616e-09));
        assert_eq!(eph.get_orbit_f64("iscL1DS"), Some(5.995389074087e-09));
        assert_eq!(eph.get_orbit_f64("t_tm"), Some(5.191380000000e+05));
    }

    #[test]
    fn glonass_l1oc_v4() {
        // RINEX 4.02 Table A18
        let content =
            "R26 2024 02 03 00 15 00-1.605716170161e-05 1.652011860642e-12-2.081668171172e-17
     1.812154053020e+04-2.071979139000e+00 5.729816621169e-10 1.000000000000e+00
    -2.325615360260e+03 1.285475494340e+00-1.047737896442e-09 1.000000000000e+00
     1.781341854668e+04 2.280306640081e+00-9.458744898438e-10 0.000000000000e+00
     2.000000000000e+00 1.000000000000e+01 7.500000000000e-01 7.500000000000e-01
     0.000000000000e+00 0.000000000000e+00 0.000000000000e+00 0.000000000000e+00
     0.000000000000e+00 0.000000000000e+00 0.000000000000e+00 0.000000000000e+00
     0.000000000000e+00 0.000000000000e+00 0.000000000000e+00 0.000000000000e+00
     1.500000000000e+01 5.000000000000e+00                    5.184000000000e+05";
        for msgtype in [NavMessageType::L1OC, NavMessageType::L3OC] {
            let (epoch, sv, eph) =
                Ephemeris::parse_v4(msgtype, content.lines(), TimeScale::UTC).unwrap();
            assert_eq!(sv, SV::from_str("R26").unwrap());
            assert_eq!(epoch, Epoch::from_str("2024-02-03T00:15:00 UTC").unwrap());
            assert_eq!(eph.get_orbit_f64("satPosX"), Some(1.812154053020e+04));
            assert_eq!(eph.get_orbit_f64("health"), Some(1.0));
            assert_eq!(eph.get_orbit_f64("dataValidity"), Some(1.0));
            assert_eq!(eph.get_orbit_f64("satType"), Some(2.0));
            assert_eq!(eph.get_orbit_f64("sourceFlags"), Some(10.0));
            assert_eq!(eph.get_orbit_f64("uraiOrb"), Some(15.0));
            assert_eq!(eph.get_orbit_f64("uraiClk"), Some(5.0));
            assert_eq!(eph.get_orbit_f64("t_tm"), Some(5.184000000000e+05));
        }
    }

    #[test]
    fn cnav_flags_field() {
        // RINEX 4.02 Table A10: optional flags after wn_op
        let content =
            "G04 2019 03 14 03 30 00 1.330042141490e-04 7.226219622680e-12 0.000000000000e+00
     2.001762390137e-03 6.914062500000e-01 4.625906973308e-09 1.887277537485e+00
     1.024454832077e-08 3.348654136062e-04 8.376315236092e-06 5.153800325291e+03
     2.412000000000e+05-4.656612873077e-09 5.171544951605e-01 2.328306436539e-08
     9.601927657114e-01 2.174140625000e+02-1.737767543851e+00-8.034028170143e-09
    -2.950122884460e-10-1.312310522376e-14-2.000000000000e+00 2.000000000000e+00
     0.000000000000e+00 7.000000000000e+00-8.789356797934e-09 5.000000000000e+00
    -5.820766091347e-10-6.606569513679e-09-1.178705133498e-08-1.178705133498e-08
     3.558540000000e+05 2.044000000000e+03";
        let (_, _, eph) =
            Ephemeris::parse_v4(NavMessageType::CNAV, content.lines(), TimeScale::GPST).unwrap();
        assert_eq!(eph.get_orbit_f64("wn_op"), Some(2044.0));
        assert_eq!(eph.get_orbit_f64("flags"), None);

        let content = format!("{} 3.000000000000e+00", content);
        let (_, _, eph) =
            Ephemeris::parse_v4(NavMessageType::CNAV, content.lines(), TimeScale::GPST).unwrap();
        assert_eq!(eph.get_orbit_f64("flags"), Some(3.0));
    }
}
