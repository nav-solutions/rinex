use crate::{
    epoch::parse_in_timescale as parse_epoch_in_timescale,
    navigation::{
        ephemeris::orbits::{closest_nav_standards, OrbitItem},
        Ephemeris, NavMessageType,
    },
    prelude::{Constellation, Epoch, ParsingError, TimeScale, Version, SV},
};

use std::{collections::HashMap, str::Lines};

/// Parses all orbital elements.
/// Descriptor is retrieved from database db/NAV/orbits.
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
        let mut line: &str = match version.major < 3 {
            true => &line[3..],
            false => &line[4..],
        };

        let mut nb_missing = 4 - (line.len() / word_size);
        // println!("LINE \"{}\" | NB MISSING {}", line, nb_missing); //DEBUG

        loop {
            if line.is_empty() {
                key_index += nb_missing;
                break;
            }

            let (val_str, rem) = line.split_at(std::cmp::min(word_size, line.len()));
            let val_str = val_str.trim();
            // println!("CONTENT \"{}\"", content); // DEBUG

            // handle omitted fields
            if val_str.is_empty() {
                // omitted field
                key_index += 1;
                nb_missing = nb_missing.saturating_sub(1);
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
                match OrbitItem::new(name_str, type_str, val_str, &msgtype, constell) {
                    Ok(item) => {
                        // println!("found key=\"{}\" (type={}) value=\"{}\"", key, token, content); // DEBUG
                        map.insert(name_str.to_string(), item);
                    },
                    Err(_) => {},
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

        let ts = sv
            .constellation
            .timescale()
            .ok_or(ParsingError::NoTimescaleDefinition)?;

        let epoch = parse_epoch_in_timescale(date.trim(), ts)?;

        let clock_bias = clk_bias
            .replace('D', "E")
            .trim()
            .parse::<f64>()
            .map_err(|_| ParsingError::ClockParsing)?;

        let clock_drift = clk_dr
            .replace('D', "E")
            .trim()
            .parse::<f64>()
            .map_err(|_| ParsingError::ClockParsing)?;

        let mut clock_drift_rate = clk_drr
            .replace('D', "E")
            .trim()
            .parse::<f64>()
            .map_err(|_| ParsingError::ClockParsing)?;

        // parse orbits :
        //  only Legacy Frames in V2 and V3 (old) RINEX
        let mut orbits = parse_orbits(version, NavMessageType::LNAV, sv.constellation, lines)?;

        if sv.constellation.is_sbas() {
            // SBAS frames specificity:
            // clock drift rate does not exist and is actually the week counter
            orbits.insert(
                "week".to_string(),
                OrbitItem::U32(clock_drift_rate.round() as u32),
            );

            clock_drift_rate = 0.0_f64; // drift rate null: non existing
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

        let clock_bias = clk_bias
            .replace('D', "E")
            .trim()
            .parse::<f64>()
            .map_err(|_| ParsingError::ClockParsing)?;

        let clock_drift = clk_dr
            .replace('D', "E")
            .trim()
            .parse::<f64>()
            .map_err(|_| ParsingError::ClockParsing)?;

        let mut clock_drift_rate = clk_drr
            .replace('D', "E")
            .trim()
            .parse::<f64>()
            .map_err(|_| ParsingError::ClockParsing)?;

        let mut orbits =
            parse_orbits(Version { major: 4, minor: 0 }, msg, sv.constellation, lines)?;

        if sv.constellation.is_sbas() {
            // SBAS frames specificity:
            // clock drift rate does not exist and is actually the week counter
            orbits.insert(
                "week".to_string(),
                OrbitItem::U32(clock_drift_rate.round() as u32),
            );
            clock_drift_rate = 0.0_f64; // drift rate null: non existing
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
        navigation::{Ephemeris, NavMessageType},
        prelude::{Constellation, Version},
    };

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
        assert_eq!(
            ephemeris.get_orbit_field_f64("iodnav"),
            Some(7.500000000000e+01)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("crs"),
            Some(1.478125000000e+01)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("deltaN"),
            Some(2.945479833915e-09)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("m0"),
            Some(-3.955466341850e-01)
        );

        assert_eq!(
            ephemeris.get_orbit_field_f64("cuc"),
            Some(8.065253496170e-07)
        );
        assert_eq!(ephemeris.get_orbit_field_f64("e"), Some(3.683507675305e-04));
        assert_eq!(
            ephemeris.get_orbit_field_f64("cus"),
            Some(-3.911554813385e-07)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("sqrta"),
            Some(5.440603218079e+03)
        );

        assert_eq!(
            ephemeris.get_orbit_field_f64("toe"),
            Some(3.522000000000e+05)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("cic"),
            Some(-6.519258022308e-08)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("omega0"),
            Some(2.295381450845e+00)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("cis"),
            Some(7.450580596924e-09)
        );

        assert_eq!(
            ephemeris.get_orbit_field_f64("i0"),
            Some(9.883726443393e-01)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("crc"),
            Some(3.616875000000e+02)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("omega"),
            Some(2.551413130998e-01)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("omegaDot"),
            Some(-5.907746081337e-09)
        );

        assert_eq!(
            ephemeris.get_orbit_field_f64("idot"),
            Some(1.839362331110e-10)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("source"),
            Some(2.580000000000e+02)
        );
        assert_eq!(ephemeris.get_week(), Some(2111));

        assert_eq!(
            ephemeris.get_orbit_field_f64("sisa"),
            Some(3.120000000000e+00)
        );
        //assert_eq!(ephemeris.get_orbit_field_f64("health"), Some(0.000000000000e+00));
        assert_eq!(
            ephemeris.get_orbit_field_f64("bgdE5aE1"),
            Some(-1.303851604462e-08)
        );
        assert!(ephemeris.get_orbit_field_f64("bgdE5bE1").is_none());

        assert_eq!(
            ephemeris.get_orbit_field_f64("t_tm"),
            Some(3.555400000000e+05)
        );
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

        assert_eq!(ephemeris.get_orbit_field_f64("aode"), Some(1.0));
        assert_eq!(
            ephemeris.get_orbit_field_f64("crs"),
            Some(1.18906250000e+01)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("deltaN"),
            Some(0.105325815814e-08)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("m0"),
            Some(-0.255139531119e+01)
        );

        assert_eq!(
            ephemeris.get_orbit_field_f64("cuc"),
            Some(0.169500708580e-06)
        );
        assert_eq!(ephemeris.get_orbit_field_f64("e"), Some(0.401772442274e-03));
        assert_eq!(
            ephemeris.get_orbit_field_f64("cus"),
            Some(0.292365439236e-04)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("sqrta"),
            Some(0.649346986580e+04)
        );

        assert_eq!(
            ephemeris.get_orbit_field_f64("toe"),
            Some(0.432000000000e+06)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("cic"),
            Some(0.105705112219e-06)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("omega0"),
            Some(-0.277512444499e+01)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("cis"),
            Some(-0.211410224438e-06)
        );

        assert_eq!(
            ephemeris.get_orbit_field_f64("i0"),
            Some(0.607169709798e-01)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("crc"),
            Some(-0.897671875000e+03)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("omega"),
            Some(0.154887266488e+00)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("omegaDot"),
            Some(-0.871464871438e-10)
        );

        assert_eq!(
            ephemeris.get_orbit_field_f64("idot"),
            Some(-0.940753471872e-09)
        );
        assert_eq!(ephemeris.get_week(), Some(782));
        assert_eq!(
            ephemeris.get_orbit_field_f64("accuracy"),
            Some(0.200000000000e+01)
        );
        assert!(ephemeris.get_orbit_field_f64("health").is_some());

        assert_eq!(
            ephemeris.get_orbit_field_f64("tgd1b1b3"),
            Some(-0.599999994133e-09)
        );

        assert_eq!(
            ephemeris.get_orbit_field_f64("tgd2b2b3"),
            Some(-0.900000000000e-08)
        );

        assert!(ephemeris.get_orbit_field_f64("aodc").is_none());
        assert_eq!(
            ephemeris.get_orbit_field_f64("t_tm"),
            Some(0.432000000000e+06)
        );
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
        assert_eq!(
            ephemeris.get_orbit_field_f64("posX"),
            Some(-1.488799804690E3)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("posY"),
            Some(1.292880712890E4)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("posZ"),
            Some(2.193169775390E4)
        );

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

        assert_eq!(ephemeris.get_orbit_field_f64("channel"), Some(5.0));
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
        assert_eq!(
            ephemeris.get_orbit_field_f64("posX"),
            Some(0.783916601562E4)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("posY"),
            Some(-0.216949155273E5)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("posZ"),
            Some(0.109021518555E5)
        );
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
        assert_eq!(
            ephemeris.get_orbit_field_f64("posX"),
            Some(-1.488799804690E3)
        );
        assert_eq!(ephemeris.get_orbit_field_f64("velX"), None);
        assert_eq!(
            ephemeris.get_orbit_field_f64("posY"),
            Some(1.292880712890E4)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("posZ"),
            Some(2.193169775390E4)
        );
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
        assert_eq!(
            ephemeris.get_orbit_field_f64("posX"),
            Some(0.783916601562E4)
        );
        assert_eq!(ephemeris.get_orbit_field_f64("velX"), None);
        assert_eq!(
            ephemeris.get_orbit_field_f64("posY"),
            Some(-0.216949155273E5)
        );
        assert_eq!(
            ephemeris.get_orbit_field_f64("posZ"),
            Some(0.109021518555E5)
        );
    }
}
