//! Ephemeris message formatting
use crate::{
    navigation::{
        ephemeris::orbits::closest_nav_standards, formatting::NavFormatter, Ephemeris,
        NavMessageType,
    },
    prelude::{Constellation, SV},
    FormattingError, Version,
};

use std::io::{BufWriter, Write};

/// The BeiDou group delays are named differently by the RINEX 3 and
/// RINEX 4 orbit definitions: a record parsed in one revision is written
/// in the other under either name.
fn orbit_alias(key: &str) -> Option<&'static str> {
    match key {
        "tgdb1b3" => Some("tgd1b1b3"),
        "tgd1b1b3" => Some("tgdb1b3"),
        "tgdb2b3" => Some("tgd2b2b3"),
        "tgd2b2b3" => Some("tgdb2b3"),
        _ => None,
    }
}

impl Ephemeris {
    /// Formats [Ephemeris] according to RINEX standards
    pub fn format<W: Write>(
        &self,
        w: &mut BufWriter<W>,
        sv: SV,
        version: Version,
        msgtype: NavMessageType,
    ) -> Result<(), FormattingError> {
        let sv_constellation = if sv.constellation.is_sbas() {
            Constellation::SBAS
        } else {
            sv.constellation
        };

        // retrieve standard specs
        let standard_specs = match closest_nav_standards(sv_constellation, version, msgtype) {
            Some(specs) => specs,
            None => {
                return Err(FormattingError::MissingNavigationStandards);
            },
        };

        // starts with (clock_bias, drift, rate).
        // SBAS messages carry the transmission time (seconds of week)
        // in the drift rate slot, stored as the "week" orbit item.
        // epoch has already been buffered
        let third = if sv_constellation == Constellation::SBAS {
            self.get_orbit_f64("week").unwrap_or(self.clock_drift_rate)
        } else {
            self.clock_drift_rate
        };

        // RINEX 2 records use the 'D' exponent letter
        let formatter = if version.major < 3 {
            NavFormatter::new_v2
        } else {
            NavFormatter::new
        };

        write!(
            w,
            "{}{}{}",
            formatter(self.clock_bias),
            formatter(self.clock_drift),
            formatter(third),
        )?;

        // orbit lines: three leading blanks in RINEX 2, four from RINEX 3 on.
        // Fields absent from the record are left blank, as the parser
        // read them.
        let padding = if version.major < 3 { "   " } else { "    " };
        const BLANK: &str = "                   ";

        // following standard specs
        let data_fields = &standard_specs.items;
        for (i, (field, _)) in data_fields.iter().enumerate() {
            if i % 4 == 0 {
                write!(w, "\n{}", padding)?;
            }
            match self
                .get_orbit_f64(field)
                .or_else(|| orbit_alias(field).and_then(|alias| self.get_orbit_f64(alias)))
            {
                Some(value) => write!(w, "{}", formatter(value))?,
                None => write!(w, "{}", BLANK)?,
            }
        }

        write!(w, "\n")?;
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use crate::navigation::{ephemeris::OrbitItem, Ephemeris, NavMessageType};
    use crate::prelude::{Version, SV};

    use std::io::BufWriter;
    use std::str::FromStr;

    use crate::tests::formatting::Utf8Buffer;

    #[test]
    fn test_value_formatter() {}

    #[test]
    fn ephemeris_formatting() {
        let g01 = SV::from_str("G01").unwrap();
        let msgtype = NavMessageType::LNAV;

        let ephemeris = Ephemeris {
            clock_bias: -1.0e-4,
            clock_drift: -2.0e-11,
            clock_drift_rate: 0.0,
            orbits: [
                ("iode".to_string(), OrbitItem::F64(1.0)),
                ("crs".to_string(), OrbitItem::F64(2.0)),
                ("deltaN".to_string(), OrbitItem::F64(3.0)),
                ("4".to_string(), OrbitItem::F64(4.0)),
                ("cuc".to_string(), OrbitItem::F64(5.0)),
            ]
            .into_iter()
            .collect(),
        };

        // RINEX 2: orbit lines indented by three blanks, 'D' exponent,
        // fields absent from the record left blank
        let version = Version::from_str("2.0").unwrap();
        let utf8 = Utf8Buffer::new(1024);
        let mut writer = BufWriter::new(utf8);

        ephemeris
            .format(&mut writer, g01, version, msgtype)
            .unwrap();

        let inner = writer.into_inner().unwrap();
        let utf8 = inner.to_ascii_utf8();

        assert_eq!(
            utf8,
            "-1.000000000000D-04-2.000000000000D-11 0.000000000000D+00
    1.000000000000D+00 2.000000000000D+00 3.000000000000D+00                   
    5.000000000000D+00                                                         
                                                                               
                                                                               
                                                                               
                                                                               
                                         \n"
        );

        // RINEX 3: four blanks
        let version = Version::from_str("3.0").unwrap();
        let utf8 = Utf8Buffer::new(1024);
        let mut writer = BufWriter::new(utf8);

        ephemeris
            .format(&mut writer, g01, version, msgtype)
            .unwrap();

        let inner = writer.into_inner().unwrap();
        let utf8 = inner.to_ascii_utf8();

        assert!(utf8.starts_with(
            "-1.000000000000E-04-2.000000000000E-11 0.000000000000E+00
     1.000000000000E+00 2.000000000000E+00 3.000000000000E+00                   
     5.000000000000E+00"
        ));
    }

    #[test]
    fn sbas_ephemeris_formatting() {
        // SBAS: the transmission time (seconds of week) is written
        // in the clock drift rate slot
        let s36 = SV::from_str("S36").unwrap();
        let version = Version::from_str("3.0").unwrap();

        let ephemeris = Ephemeris {
            clock_bias: 0.0,
            clock_drift: 0.0,
            clock_drift_rate: 0.0,
            orbits: [
                ("week".to_string(), OrbitItem::U32(437280)),
                ("satPosX".to_string(), OrbitItem::F64(42003.688)),
            ]
            .into_iter()
            .collect(),
        };

        let utf8 = Utf8Buffer::new(1024);
        let mut writer = BufWriter::new(utf8);

        ephemeris
            .format(&mut writer, s36, version, NavMessageType::LNAV)
            .unwrap();

        let inner = writer.into_inner().unwrap();
        let utf8 = inner.to_ascii_utf8();

        assert!(utf8.starts_with(
            " 0.000000000000E+00 0.000000000000E+00 4.372800000000E+05
     4.200368800000E+04"
        ));
    }
}
