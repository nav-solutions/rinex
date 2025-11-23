use crate::{
    epoch::parse_utc as parse_utc_epoch,
    navigation::{Ephemeris, NavFrame, NavFrameType, NavKey, NavMessageType},
    prelude::{Header, ParsingError, Version, SV},
};

mod v4;
use v4::parse as parse_v4_epoch;

/// ([NavKey], [NavFrame]) parsing attempt
pub fn parse_epoch(header: &Header, content: &str) -> Result<(NavKey, NavFrame), ParsingError> {
    if content.starts_with('>') {
        parse_v4_epoch(content)
    } else {
        // <V4: limited to LNAV Ephemeris frames.
        let version = header.version;

        let constellation = header
            .constellation
            .ok_or(ParsingError::UndefinedConstellation)?;

        let (epoch, sv, eph) = Ephemeris::parse_v2v3(version, constellation, content.lines())?;

        let key = NavKey {
            epoch,
            sv,
            msgtype: NavMessageType::LNAV,
            frmtype: NavFrameType::Ephemeris,
        };

        let frame = NavFrame::EPH(eph);

        Ok((key, frame))
    }
}

/// Returns true if given content matches the beginning of a
/// Navigation record epoch
pub fn is_new_epoch(line: &str, v: Version) -> bool {
    if v.major < 3 {
        // old RINEX
        if line.len() < 23 {
            return false; // not enough bytes
                          // to describe a PRN and an Epoch
        }

        let (prn, _) = line.split_at(2);
        if prn.trim().parse::<u8>().is_err() {
            return false;
        }

        let datestr = &line[3..22];
        parse_utc_epoch(datestr).is_ok()
    } else if v.major == 3 {
        // RINEX V3
        if line.len() < 24 {
            return false; // not enough bytes
                          // to describe an SV and an Epoch
        }

        // 1st entry matches a valid SV description
        let (sv, _) = line.split_at(4);

        if sv.parse::<SV>().is_err() {
            return false;
        }

        let datestr = &line[4..23];
        parse_utc_epoch(datestr).is_ok()
    } else {
        // Modern --> easy
        if let Some(c) = line.chars().next() {
            c == '>' // new epoch marker
        } else {
            false
        }
    }
}

#[cfg(test)]
mod test {
    use super::{is_new_epoch, parse_epoch};

    use crate::{
        navigation::{NavFrameType, NavMessageType},
        prelude::{Constellation, Epoch, Header, Version},
    };

    use std::str::FromStr;

    #[test]
    fn test_new_v3_epoch() {
        // NAV V<3
        let line =
            " 1 20 12 31 23 45  0.0 7.282570004460D-05 0.000000000000D+00 7.380000000000D+04";

        assert!(is_new_epoch(line, Version::new(1, 0)));
        assert!(is_new_epoch(line, Version::new(2, 0)));
        assert!(!is_new_epoch(line, Version::new(3, 0)));
        assert!(!is_new_epoch(line, Version::new(4, 0)));

        // NAV V<3
        let line =
            " 2 21  1  1 11 45  0.0 4.610531032090D-04 1.818989403550D-12 4.245000000000D+04";

        assert!(is_new_epoch(line, Version::new(1, 0)));
        assert!(is_new_epoch(line, Version::new(2, 0)));
        assert!(!is_new_epoch(line, Version::new(3, 0)));
        assert!(!is_new_epoch(line, Version::new(4, 0)));

        // GPS NAV V<3
        let line =
            " 3 17  1 13 23 59 44.0-1.057861372828D-04-9.094947017729D-13 0.000000000000D+00";

        assert!(is_new_epoch(line, Version::new(1, 0)));
        assert!(is_new_epoch(line, Version::new(2, 0)));
        assert!(!is_new_epoch(line, Version::new(3, 0)));
        assert!(!is_new_epoch(line, Version::new(4, 0)));

        // NAV V3
        let line =
            "C05 2021 01 01 00 00 00-4.263372393325e-04-7.525180478751e-11 0.000000000000e+00";

        assert!(!is_new_epoch(line, Version::new(1, 0)));
        assert!(!is_new_epoch(line, Version::new(2, 0)));
        assert!(is_new_epoch(line, Version::new(3, 0)));
        assert!(!is_new_epoch(line, Version::new(4, 0)));

        // NAV V3
        let line =
            "R21 2022 01 01 09 15 00-2.666609361768E-04-2.728484105319E-12 5.508000000000E+05";

        assert!(!is_new_epoch(line, Version::new(1, 0)));
        assert!(!is_new_epoch(line, Version::new(2, 0)));
        assert!(is_new_epoch(line, Version::new(3, 0)));
        assert!(!is_new_epoch(line, Version::new(4, 0)));
    }

    #[test]
    fn test_new_v4_epoch() {
        let line = "> EPH G02 LNAV";
        assert!(!is_new_epoch(line, Version::new(2, 0)));
        assert!(!is_new_epoch(line, Version::new(3, 0)));
        assert!(is_new_epoch(line, Version::new(4, 0)));
    }

    #[test]
    fn glonass_v2_parsing() {
        let version = Version::new(2, 0);

        let header = Header::basic_nav()
            .with_version(version)
            .with_constellation(Constellation::Glonass);

        let content =
            " 1 20 12 31 23 45  0.0 7.282570004460D-05 0.000000000000D+00 7.380000000000D+04
   -1.488799804690D+03-2.196182250980D+00 3.725290298460D-09 0.000000000000D+00
    1.292880712890D+04-2.049269676210D+00 0.000000000000D+00 1.000000000000D+00
    2.193169775390D+04 1.059645652770D+00-9.313225746150D-10 0.000000000000D+00";

        assert!(is_new_epoch(content, version));

        let entry = parse_epoch(&header, content);
        assert!(entry.is_ok(), "failed to parse epoch {:?}", entry.err());

        let (key, frame) = entry.unwrap();

        assert_eq!(
            key.epoch,
            Epoch::from_gregorian_utc(2020, 12, 31, 23, 45, 00, 00)
        );

        assert_eq!(key.sv.prn, 1);
        assert_eq!(key.sv.constellation, Constellation::Glonass);
        assert_eq!(key.frmtype, NavFrameType::Ephemeris);
        assert_eq!(key.msgtype, NavMessageType::LNAV);

        let ephemeris = frame.as_ephemeris().unwrap();

        assert_eq!(ephemeris.clock_bias, 7.282570004460E-05);
        assert_eq!(ephemeris.clock_drift, 0.0);
        assert_eq!(ephemeris.clock_drift_rate, 7.38E4);

        let orbits = &ephemeris.orbits;
        assert_eq!(orbits.len(), 10);

        for (k, v) in orbits.iter() {
            if k.eq("posX") {
                assert_eq!(v.as_f64(), -1.488799804690E+03);
            } else if k.eq("velX") {
                assert_eq!(v.as_f64(), -2.196182250980E+00);
            } else if k.eq("accelX") {
                assert_eq!(v.as_f64(), 3.725290298460E-09);
            } else if k.eq("health") {
                assert!(v.as_glonass_health_flag().is_some());
            } else if k.eq("posY") {
                assert_eq!(v.as_f64(), 1.292880712890E+04);
            } else if k.eq("velY") {
                assert_eq!(v.as_f64(), -2.049269676210E+00);
            } else if k.eq("accelY") {
                assert_eq!(v.as_f64(), 0.0);
            } else if k.eq("channel") {
                assert_eq!(v.as_i8(), 1);
            } else if k.eq("posZ") {
                assert_eq!(v.as_f64(), 2.193169775390E+04);
            } else if k.eq("velZ") {
                assert_eq!(v.as_f64(), 1.059645652770E+00);
            } else if k.eq("accelZ") {
                assert_eq!(v.as_f64(), -9.313225746150E-10);
            } else if k.eq("ageOp") {
                assert_eq!(v.as_f64(), 0.0);
            } else {
                panic!("Got unexpected key \"{}\" for GLOV2 record", k);
            }
        }
    }

    #[test]
    fn beidou_v3_parsing() {
        let header = Header::basic_nav().with_constellation(Constellation::Mixed);

        let content =
            "C05 2021 01 01 00 00 00 -.426337239332e-03 -.752518047875e-10  .000000000000e+00
      .100000000000e+01  .118906250000e+02  .105325815814e-08 -.255139531119e+01
      .169500708580e-06  .401772442274e-03  .292365439236e-04  .649346986580e+04
      .432000000000e+06  .105705112219e-06 -.277512444499e+01 -.211410224438e-06
      .607169709798e-01 -.897671875000e+03  .154887266488e+00 -.871464871438e-10
     -.940753471872e-09  .000000000000e+00  .782000000000e+03  .000000000000e+00
      .200000000000e+01  .000000000000e+00 -.599999994133e-09 -.900000000000e-08
      .432000000000e+06  .000000000000e+00 0.000000000000e+00 0.000000000000e+00";

        let (key, frame) = parse_epoch(&header, content).unwrap();

        assert_eq!(key.sv.prn, 5);
        assert_eq!(key.sv.constellation, Constellation::BeiDou);
        assert_eq!(
            key.epoch,
            Epoch::from_str("2021-01-01T00:00:00 BDT").unwrap()
        );
        assert_eq!(key.msgtype, NavMessageType::LNAV);
        assert_eq!(key.frmtype, NavFrameType::Ephemeris);

        let ephemeris = frame.as_ephemeris().unwrap();

        assert_eq!(ephemeris.clock_bias, -0.426337239332E-03);
        assert_eq!(ephemeris.clock_drift, -0.752518047875e-10);
        assert_eq!(ephemeris.clock_drift_rate, 0.0);

        let orbits = &ephemeris.orbits;
        assert_eq!(orbits.len(), 23);

        for (k, v) in orbits.iter() {
            if k.eq("aode") {
                assert_eq!(v.as_f64(), 0.100000000000e+01);
            } else if k.eq("crs") {
                assert_eq!(v.as_f64(), 0.118906250000e+02);
            } else if k.eq("deltaN") {
                assert_eq!(v.as_f64(), 0.105325815814e-08);
            } else if k.eq("m0") {
                assert_eq!(v.as_f64(), -0.255139531119e+01);
            } else if k.eq("cuc") {
                assert_eq!(v.as_f64(), 0.169500708580e-06);
            } else if k.eq("e") {
                assert_eq!(v.as_f64(), 0.401772442274e-03);
            } else if k.eq("cus") {
                assert_eq!(v.as_f64(), 0.292365439236e-04);
            } else if k.eq("sqrta") {
                assert_eq!(v.as_f64(), 0.649346986580e+04);
            } else if k.eq("toe") {
                assert_eq!(v.as_f64(), 0.432000000000e+06);
            } else if k.eq("cic") {
                assert_eq!(v.as_f64(), 0.105705112219e-06);
            } else if k.eq("omega0") {
                assert_eq!(v.as_f64(), -0.277512444499e+01);
            } else if k.eq("cis") {
                assert_eq!(v.as_f64(), -0.211410224438e-06);
            } else if k.eq("i0") {
                assert_eq!(v.as_f64(), 0.607169709798e-01);
            } else if k.eq("crc") {
                assert_eq!(v.as_f64(), -0.897671875000e+03);
            } else if k.eq("omega") {
                assert_eq!(v.as_f64(), 0.154887266488e+00);
            } else if k.eq("omegaDot") {
                assert_eq!(v.as_f64(), -0.871464871438e-10);
            } else if k.eq("idot") {
                assert_eq!(v.as_f64(), -0.940753471872e-09);
            // SPARE
            } else if k.eq("week") {
                assert_eq!(v.as_u32(), 782);
            //SPARE
            } else if k.eq("accuracy") {
                assert_eq!(v.as_f64(), 0.200000000000e+01);
            } else if k.eq("health") {
                assert_eq!(v.as_f64(), 0.000000000000e+00);
            } else if k.eq("tgd1b1b3") {
                assert_eq!(v.as_f64(), -0.599999994133e-09);
            } else if k.eq("tgd2b2b3") {
                assert_eq!(v.as_f64(), -0.900000000000e-08);
            } else if k.eq("t_tm") {
                assert_eq!(v.as_f64(), 0.432000000000e+06);
            } else if k.eq("aodc") {
                assert_eq!(v.as_f64(), 0.000000000000e+00);
            } else {
                panic!("Got unexpected key \"{}\" for BDSV3 record", k);
            }
        }
    }

    #[test]
    fn parse_galileo_v3() {
        let header = Header::basic_nav().with_constellation(Constellation::Galileo);

        let content =
            "E01 2021 01 01 10 10 00 -.101553811692e-02 -.804334376880e-11  .000000000000e+00
      .130000000000e+02  .435937500000e+02  .261510892978e-08 -.142304064404e+00
      .201165676117e-05  .226471573114e-03  .109840184450e-04  .544061822701e+04
      .468600000000e+06  .111758708954e-07 -.313008275208e+01  .409781932831e-07
      .980287270202e+00  .113593750000e+03 -.276495796017e+00 -.518200156545e-08
     -.595381942905e-09  .258000000000e+03  .213800000000e+04 0.000000000000e+00
      .312000000000e+01  .000000000000e+00  .232830643654e-09  .000000000000e+00
      .469330000000e+06 0.000000000000e+00 0.000000000000e+00 0.000000000000e+00";

        let (key, frame) = parse_epoch(&header, content).unwrap();

        assert_eq!(key.sv.prn, 1);
        assert_eq!(key.sv.constellation, Constellation::Galileo);
        assert_eq!(
            key.epoch,
            Epoch::from_str("2021-01-01T10:10:00 GST").unwrap(),
        );
        assert_eq!(key.frmtype, NavFrameType::Ephemeris);
        assert_eq!(key.msgtype, NavMessageType::LNAV);

        let ephemeris = frame.as_ephemeris().unwrap();

        assert_eq!(ephemeris.clock_bias, -0.101553811692e-02);
        assert_eq!(ephemeris.clock_drift, -0.804334376880e-11);
        assert_eq!(ephemeris.clock_drift_rate, 0.0);

        let orbits = &ephemeris.orbits;
        assert_eq!(orbits.len(), 23);

        for (k, v) in orbits.iter() {
            if k.eq("iodnav") {
                assert_eq!(v.as_f64(), 0.130000000000e+02);
            } else if k.eq("crs") {
                assert_eq!(v.as_f64(), 0.435937500000e+02);
            } else if k.eq("deltaN") {
                assert_eq!(v.as_f64(), 0.261510892978e-08);
            } else if k.eq("m0") {
                assert_eq!(v.as_f64(), -0.142304064404e+00);
            } else if k.eq("cuc") {
                assert_eq!(v.as_f64(), 0.201165676117e-05);
            } else if k.eq("e") {
                assert_eq!(v.as_f64(), 0.226471573114e-03);
            } else if k.eq("cus") {
                assert_eq!(v.as_f64(), 0.109840184450e-04);
            } else if k.eq("sqrta") {
                assert_eq!(v.as_f64(), 0.544061822701e+04);
            } else if k.eq("toe") {
                assert_eq!(v.as_f64(), 0.468600000000e+06);
            } else if k.eq("cic") {
                assert_eq!(v.as_f64(), 0.111758708954e-07);
            } else if k.eq("omega0") {
                assert_eq!(v.as_f64(), -0.313008275208e+01);
            } else if k.eq("cis") {
                assert_eq!(v.as_f64(), 0.409781932831e-07);
            } else if k.eq("i0") {
                assert_eq!(v.as_f64(), 0.980287270202e+00);
            } else if k.eq("crc") {
                assert_eq!(v.as_f64(), 0.113593750000e+03);
            } else if k.eq("omega") {
                assert_eq!(v.as_f64(), -0.276495796017e+00);
            } else if k.eq("omegaDot") {
                assert_eq!(v.as_f64(), -0.518200156545e-08);
            } else if k.eq("idot") {
                assert_eq!(v.as_f64(), -0.595381942905e-09);
            } else if k.eq("source") {
                assert_eq!(v.as_f64(), 0.258000000000e+03);
            } else if k.eq("week") {
                assert_eq!(v.as_u32(), 2138);
            //SPARE
            } else if k.eq("sisa") {
                assert_eq!(v.as_f64(), 0.312000000000e+01);
            } else if k.eq("health") {
                assert!(v.as_galileo_health_flag().is_some());
            } else if k.eq("bgdE5aE1") {
                assert_eq!(v.as_f64(), 0.232830643654e-09);
            } else if k.eq("bgdE5bE1") {
                assert_eq!(v.as_f64(), 0.000000000000e+00);
            } else if k.eq("t_tm") {
                assert_eq!(v.as_f64(), 0.469330000000e+06);
            } else {
                panic!("Got unexpected key \"{}\" for GALV3 record", k);
            }
        }
    }

    #[test]
    fn parse_glonass_v3() {
        let header = Header::basic_nav().with_constellation(Constellation::Glonass);

        let content =
            "R07 2021 01 01 09 45 00 -.420100986958e-04  .000000000000e+00  .342000000000e+05
      .124900639648e+05  .912527084351e+00  .000000000000e+00  .000000000000e+00
      .595546582031e+04  .278496932983e+01  .000000000000e+00  .500000000000e+01
      .214479208984e+05 -.131077289581e+01 -.279396772385e-08  .000000000000e+00";

        let (key, frame) = parse_epoch(&header, content).unwrap();

        assert_eq!(
            key.epoch,
            Epoch::from_gregorian_utc(2021, 01, 01, 09, 45, 00, 00)
        );

        assert_eq!(key.sv.prn, 7);
        assert_eq!(key.sv.constellation, Constellation::Glonass);
        assert_eq!(key.msgtype, NavMessageType::LNAV);
        assert_eq!(key.frmtype, NavFrameType::Ephemeris);

        let ephemeris = frame.as_ephemeris().unwrap();

        assert_eq!(ephemeris.clock_bias, -0.420100986958e-04);
        assert_eq!(ephemeris.clock_drift, 0.000000000000e+00);
        assert_eq!(ephemeris.clock_drift_rate, 0.342000000000e+05);

        let orbits = &ephemeris.orbits;
        assert_eq!(orbits.len(), 9);

        for (k, v) in orbits.iter() {
            if k.eq("posX") {
                assert_eq!(v.as_f64(), 0.124900639648e+05);
            } else if k.eq("velX") {
                assert_eq!(v.as_f64(), 0.912527084351e+00);
            } else if k.eq("accelX") {
                assert_eq!(v.as_f64(), 0.000000000000e+00);
            } else if k.eq("health") {
                assert!(v.as_glonass_health_flag().is_some());
            } else if k.eq("posY") {
                assert_eq!(v.as_f64(), 0.595546582031e+04);
            } else if k.eq("velY") {
                assert_eq!(v.as_f64(), 0.278496932983e+01);
            } else if k.eq("accelY") {
                assert_eq!(v.as_f64(), 0.000000000000e+00);
            } else if k.eq("channel") {
                assert_eq!(v.as_i8(), 5);
            } else if k.eq("posZ") {
                assert_eq!(v.as_f64(), 0.214479208984e+05);
            } else if k.eq("velZ") {
                assert_eq!(v.as_f64(), -0.131077289581e+01);
            } else if k.eq("accelZ") {
                assert_eq!(v.as_f64(), -0.279396772385e-08);
            } else if k.eq("ageOp") {
                assert_eq!(v.as_f64(), 0.000000000000e+00);
            } else {
                panic!("Got unexpected key \"{}\" for GLOV3 record", k);
            }
        }
    }
}
