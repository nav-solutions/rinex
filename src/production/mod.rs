/*
 * File Production infrastructure.
 * File production information are specified in RINEX files that were named
 * according to standard specifications.
 *
 * Two use cases of this module:
 *   1. When a RINEX was parsed succesfully, we attach Self
 *      if we do regocnized a standard name.
 *      This helps regenerating a filename that still follows the standards.
 *      If ProductionAttributes are not recognized, it is not that big of a deal.
 *      It just means it will be difficult to easily regenerate a filename that
 *      strictly follows the standards, because we will then miss some information
 *      like the country code.
 *
 *   2. In our file production API, we can pass ProductionAttributes
 *      to customize the production of this context.
 */

use thiserror::Error;

mod ffu;
mod postponing;
mod ppu;
mod sequence;
mod snapshot;
mod source;

pub use ffu::FFU;
pub use postponing::Postponing;
pub use ppu::PPU;
pub use snapshot::SnapshotMode;
pub use source::DataSource;

#[derive(Error, Debug)]
/// File Production errors
pub enum Error {
    #[error("filename does not follow naming conventions")]
    NonStandardFileName,
    #[error("invalid file sequence")]
    InvalidFileSequence,
    #[error("invalid ffu format")]
    InvalidFFU,
}

/// File production attributes. Used when generating
/// RINEX data that follows standard naming conventions,
/// or attached to data parsed from such files.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ProductionAttributes {
    /// Name serves several roles which are type dependent.
    /// - Non detailed OBS RINEX: this is usually the station name
    ///   which can be named after a geodetic marker
    /// - Non detailed NAV RINEX: station name
    /// - CLK RINEX: name of the local clock
    /// - IONEX: agency
    pub name: String,
    /// Year of production
    pub year: u32,
    /// Production Day of Year (DOY)
    pub doy: u32,
    /// Detailed production attributes that only apply to
    /// modern Navigation or Observations (either Meteo or Observation) RINEx.
    pub v3_details: Option<DetailedProductionAttributes>,
    /// Optional Regional code present in IONEX file names.
    /// 'G' means Global (World wide) TEC map(s).
    pub region: Option<char>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DetailedProductionAttributes {
    /// Agency Country Code
    pub country: String,
    /// # in Batch if Self is part of a file serie
    pub batch: u8,
    /// Data source
    pub data_src: DataSource,
    /// PPU gives information on file production periodicity.
    pub ppu: PPU,
    /// FFU gives information on Observation sampling rate.
    pub ffu: Option<FFU>,
    /// Hour of first symbol (sampling, not publication)
    pub hh: u8,
    /// Minute of first symbol (sampling, not publication)
    pub mm: u8,
}

impl ProductionAttributes {
    /* filename generator */
    pub(crate) fn rinex_short_format(name: &str, ddd: &str, yy: &str, ext: char) -> String {
        format!("{}{}0.{}{}", &name, ddd, yy, ext,)
    }
    /* filename generator */
    pub(crate) fn rinex_long_format(
        name: &str,
        batch: u8,
        country: &str,
        src: char,
        yyyy: &str,
        ddd: &str,
        hh: &str,
        mm: &str,
        ppu: &str,
        ffu: Option<&str>,
        fmt: &str,
        ext: &str,
    ) -> String {
        if let Some(ffu) = ffu {
            format!(
                "{}{:02}{}_{}_{}{}{}{}_{}_{}_{}.{}",
                name,
                batch % 99,
                country,
                src,
                yyyy,
                ddd,
                hh,
                mm,
                ppu,
                ffu,
                fmt,
                ext,
            )
        } else {
            format!(
                "{}{:02}{}_{}_{}{}{}{}_{}_{}.{}",
                name,
                batch % 99,
                country,
                src,
                yyyy,
                ddd,
                hh,
                mm,
                ppu,
                fmt,
                ext,
            )
        }
    }
}

impl std::str::FromStr for ProductionAttributes {
    type Err = Error;
    fn from_str(fname: &str) -> Result<Self, Self::Err> {
        let fname = fname.to_uppercase();
        if fname.len() < 13 {
            let offset = fname.find('.').unwrap_or(0);
            if offset != 8 {
                return Err(Error::NonStandardFileName);
            };

            // determine type of RINEX first
            // because it determines how to parse the "name" field
            let year = fname[offset + 1..offset + 3]
                .parse::<u32>()
                .map_err(|_| Error::NonStandardFileName)?;

            let rtype = &fname[offset + 3..offset + 4];
            let name_offset = match rtype {
                "I" => 3usize, // only 3 digits on IONEX
                _ => 4usize,
            };

            Ok(Self {
                year: year + 2_000, // year uses 2 digit in old format
                name: fname[..name_offset].to_string(),
                doy: {
                    fname[4..7]
                        .parse::<u32>()
                        .map_err(|_| Error::NonStandardFileName)?
                },
                region: match rtype {
                    "I" => fname.chars().nth(3),
                    _ => None,
                },
                v3_details: None,
            })
        } else {
            let offset = fname.find('.').unwrap_or(0);
            if offset < 30 {
                return Err(Error::NonStandardFileName);
            };

            let year = fname[12..16]
                .parse::<u32>()
                .map_err(|_| Error::NonStandardFileName)?;

            let batch = fname[5..6]
                .parse::<u8>()
                .map_err(|_| Error::NonStandardFileName)?;

            // determine type of RINEX first
            // because it determines how to parse the "name" field
            let rtype = &fname[offset + 3..offset + 4];
            let name_offset = match rtype {
                "I" => 3usize, // only 3 digits on IONEX
                _ => 4usize,
            };

            Ok(Self {
                year,
                name: fname[..name_offset].to_string(),
                doy: {
                    fname[16..19]
                        .parse::<u32>()
                        .map_err(|_| Error::NonStandardFileName)?
                },
                region: None, // IONEX files only use a short format
                v3_details: Some(DetailedProductionAttributes {
                    batch,
                    country: fname[6..9].to_string(),
                    ppu: PPU::from_str(&fname[24..27])?,
                    data_src: DataSource::from_str(&fname[10..11])?,
                    hh: {
                        fname[19..21]
                            .parse::<u8>()
                            .map_err(|_| Error::NonStandardFileName)?
                    },
                    mm: {
                        fname[21..23]
                            .parse::<u8>()
                            .map_err(|_| Error::NonStandardFileName)?
                    },
                    ffu: match offset {
                        34 => Some(FFU::from_str(&fname[28..32])?),
                        _ => None, // NAV FILE case
                    },
                }),
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::DetailedProductionAttributes;
    use super::ProductionAttributes;
    use super::{DataSource, FFU, PPU};

    use hifitime::Unit;
    use std::str::FromStr;
    #[test]
    fn short_rinex_filenames() {
        for (filename, name, year, doy) in [
            ("AJAC3550.21O", "AJAC", 2021, 355),
            ("AJAC3550.21D", "AJAC", 2021, 355),
            ("KOSG0010.15O", "KOSG", 2015, 1),
            ("rovn0010.21o", "ROVN", 2021, 1),
            ("barq071q.19o", "BARQ", 2019, 71),
            ("VLNS0010.22D", "VLNS", 2022, 1),
        ] {
            println!("Testing RINEX filename \"{}\"", filename);
            let attrs = ProductionAttributes::from_str(filename).unwrap();
            assert_eq!(attrs.name, name);
            assert_eq!(attrs.year, year);
            assert_eq!(attrs.doy, doy);
        }
    }
    #[test]
    fn long_rinex_filenames() {
        for (filename, name, year, doy, detail) in [
            (
                "ACOR00ESP_R_20213550000_01D_30S_MO.crx",
                "ACOR",
                2021,
                355,
                DetailedProductionAttributes {
                    country: "ESP".to_string(),
                    batch: 0,
                    data_src: DataSource::Receiver,
                    ppu: PPU::Daily,
                    hh: 0,
                    mm: 0,
                    ffu: Some(FFU {
                        val: 30,
                        unit: Unit::Second,
                    }),
                },
            ),
            (
                "KMS300DNK_R_20221591000_01H_30S_MO.crx",
                "KMS3",
                2022,
                159,
                DetailedProductionAttributes {
                    country: "DNK".to_string(),
                    batch: 0,
                    data_src: DataSource::Receiver,
                    ppu: PPU::Hourly,
                    hh: 10,
                    mm: 0,
                    ffu: Some(FFU {
                        val: 30,
                        unit: Unit::Second,
                    }),
                },
            ),
            (
                "AMEL00NLD_R_20210010000_01D_MN.rnx",
                "AMEL",
                2021,
                1,
                DetailedProductionAttributes {
                    country: "NLD".to_string(),
                    batch: 0,
                    data_src: DataSource::Receiver,
                    hh: 0,
                    mm: 0,
                    ppu: PPU::Daily,
                    ffu: None,
                },
            ),
            (
                "ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz",
                "ESBC",
                2020,
                177,
                DetailedProductionAttributes {
                    country: "DNK".to_string(),
                    batch: 0,
                    data_src: DataSource::Receiver,
                    ppu: PPU::Daily,
                    hh: 0,
                    mm: 0,
                    ffu: Some(FFU {
                        val: 30,
                        unit: Unit::Second,
                    }),
                },
            ),
            (
                "MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz",
                "MOJN",
                2020,
                177,
                DetailedProductionAttributes {
                    country: "DNK".to_string(),
                    batch: 0,
                    data_src: DataSource::Receiver,
                    ppu: PPU::Daily,
                    hh: 0,
                    mm: 0,
                    ffu: Some(FFU {
                        val: 30,
                        unit: Unit::Second,
                    }),
                },
            ),
            (
                "ESBC00DNK_R_20201772223_01D_30S_MO.crx.gz",
                "ESBC",
                2020,
                177,
                DetailedProductionAttributes {
                    country: "DNK".to_string(),
                    batch: 0,
                    data_src: DataSource::Receiver,
                    ppu: PPU::Daily,
                    hh: 22,
                    mm: 23,
                    ffu: Some(FFU {
                        val: 30,
                        unit: Unit::Second,
                    }),
                },
            ),
            (
                "ESBC01DNK_R_20201772223_01D_30S_MO.crx.gz",
                "ESBC",
                2020,
                177,
                DetailedProductionAttributes {
                    country: "DNK".to_string(),
                    batch: 1,
                    data_src: DataSource::Receiver,
                    ppu: PPU::Daily,
                    hh: 22,
                    mm: 23,
                    ffu: Some(FFU {
                        val: 30,
                        unit: Unit::Second,
                    }),
                },
            ),
            (
                "ESBC04DNK_R_20201772223_01D_30S_MO.crx.gz",
                "ESBC",
                2020,
                177,
                DetailedProductionAttributes {
                    country: "DNK".to_string(),
                    batch: 4,
                    data_src: DataSource::Receiver,
                    ppu: PPU::Daily,
                    hh: 22,
                    mm: 23,
                    ffu: Some(FFU {
                        val: 30,
                        unit: Unit::Second,
                    }),
                },
            ),
        ] {
            println!("Testing RINEX filename \"{}\"", filename);
            let attrs = ProductionAttributes::from_str(filename).unwrap();
            assert_eq!(attrs.name, name);
            assert_eq!(attrs.year, year);
            assert_eq!(attrs.doy, doy);
            assert_eq!(attrs.v3_details, Some(detail));
        }
    }
}
