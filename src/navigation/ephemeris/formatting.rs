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

        // starts with (clock_bias, drift, rate)
        // epoch has already been buffered
        write!(
            w,
            "{}{}{}",
            NavFormatter::new(self.clock_bias),
            NavFormatter::new(self.clock_drift),
            NavFormatter::new(self.clock_drift_rate),
        )?;

        // following standard specs
        let data_fields = &standard_specs.items;
        for i in 0..data_fields.len() {
            if let Some(value) = self.get_orbit_f64(data_fields[i].0) {
                if i % 4 == 0 {
                    write!(w, "\n   {}", NavFormatter::new(value))?;
                } else {
                    write!(w, "{}", NavFormatter::new(value))?;
                }
            } else {
                if i % 4 == 0 {
                    write!(w, "\n   {}", NavFormatter::new(0.0))?;
                } else {
                    write!(w, "{}", NavFormatter::new(0.0))?;
                }
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
        let version = Version::from_str("2.0").unwrap();
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

        let utf8 = Utf8Buffer::new(1024);
        let mut writer = BufWriter::new(utf8);

        ephemeris
            .format(&mut writer, g01, version, msgtype)
            .unwrap();

        let inner = writer.into_inner().unwrap();

        let utf8 = inner.to_ascii_utf8();

        assert_eq!(
            utf8,
            "-1.000000000000E-04-2.000000000000E-11 0.000000000000E+00
    1.000000000000E+00 2.000000000000E+00 3.000000000000E+00 0.000000000000E+00
    5.000000000000E+00 0.000000000000E+00 0.000000000000E+00 0.000000000000E+00
    0.000000000000E+00 0.000000000000E+00 0.000000000000E+00 0.000000000000E+00
    0.000000000000E+00 0.000000000000E+00 0.000000000000E+00 0.000000000000E+00
    0.000000000000E+00 0.000000000000E+00 0.000000000000E+00 0.000000000000E+00
    0.000000000000E+00 0.000000000000E+00 0.000000000000E+00 0.000000000000E+00
    0.000000000000E+00 0.000000000000E+00\n"
        );
    }
}
