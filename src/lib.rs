#![doc(
    html_logo_url = "https://raw.githubusercontent.com/nav-solutions/.github/master/logos/logo2.jpg"
)]
#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::type_complexity)]

/*
 * RINEX is part of the nav-solutions framework.
 * Authors: Guillaume W. Bres <guillaume.bressaix@gmail.com> et al.
 * (cf. https://github.com/nav-solutions/rinex/graphs/contributors)
 * This framework is shipped under Mozilla Public V2 license.
 *
 * Documentation: https://github.com/nav-solutions/rinex
 */

extern crate num_derive;

#[macro_use]
extern crate lazy_static;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

extern crate gnss_rs as gnss;
extern crate num;

#[cfg(feature = "qc")]
extern crate gnss_qc_traits as qc_traits;

pub mod antex;
pub mod carrier;
pub mod clock;
pub mod doris;
pub mod error;
pub mod hardware;
pub mod hatanaka;
pub mod header;
pub mod ionex;
pub mod marker;
pub mod meteo;
pub mod navigation;
pub mod observation;
pub mod production;
pub mod record;
pub mod types;
pub mod version;

mod bibliography;
mod constants;
mod epoch;
mod iterators;
mod leap;
mod linspace;
mod observable;
mod sampling;

#[cfg(feature = "qc")]
#[cfg_attr(docsrs, doc(cfg(feature = "qc")))]
mod qc;

#[cfg(feature = "processing")]
#[cfg_attr(docsrs, doc(cfg(feature = "processing")))]
mod processing;

#[cfg(feature = "binex")]
#[cfg_attr(docsrs, doc(cfg(feature = "binex")))]
mod binex;

#[cfg(feature = "rtcm")]
#[cfg_attr(docsrs, doc(cfg(feature = "rtcm")))]
mod rtcm;

#[cfg(test)]
mod tests;

use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
    str::FromStr,
};

use itertools::Itertools;

use antex::{Antenna, FrequencyDependentData};

#[cfg(feature = "antex")]
use antex::{AntennaMatcher, AntennaSpecific};

#[cfg(feature = "flate2")]
use flate2::{read::GzDecoder, write::GzEncoder, Compression as GzCompression};

#[cfg(feature = "clock")]
use std::collections::BTreeMap;

use crate::{
    epoch::epoch_decompose,
    hatanaka::CRINEX,
    observable::Observable,
    production::{DataSource, DetailedProductionAttributes, ProductionAttributes, FFU, PPU},
};

/// Package to include all basic structures
pub mod prelude {
    // export
    pub use crate::{
        carrier::Carrier,
        doris::Station,
        error::{Error, FormattingError, ParsingError},
        hatanaka::{
            Decompressor, DecompressorExpert, DecompressorExpertIO, DecompressorIO, CRINEX,
        },
        header::Header,
        leap::Leap,
        observable::Observable,
        types::Type as RinexType,
        version::Version,
        Rinex,
    };

    pub use crate::marker::{GeodeticMarker, MarkerType};

    pub use crate::meteo::MeteoKey;

    pub use crate::prod::ProductionAttributes;
    pub use crate::record::{Comments, Record};

    // pub re-export
    pub use gnss::prelude::{Constellation, DOMESTrackingPoint, COSPAR, DOMES, SV};
    pub use hifitime::{Duration, Epoch, Polynomial, TimeScale, TimeSeries};

    #[cfg(feature = "antex")]
    #[cfg_attr(docsrs, doc(cfg(feature = "antex")))]
    pub mod antex {
        pub use crate::antex::AntennaMatcher;
    }

    #[cfg(feature = "ionex")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ionex")))]
    pub use crate::ionex::{IonexKey, QuantizedCoordinates, TEC};

    #[cfg(feature = "obs")]
    #[cfg_attr(docsrs, doc(cfg(feature = "obs")))]
    pub mod obs {
        pub use crate::carrier::Carrier;

        pub use crate::observation::{
            ClockObservation, Combination, CombinationKey, EpochFlag, LliFlags, ObsKey,
            Observations, SignalObservation, SNR,
        };
    }

    #[cfg(feature = "binex")]
    #[cfg_attr(docsrs, doc(cfg(feature = "binex")))]
    pub mod binex {
        pub use crate::binex::{BIN2RNX, RNX2BIN};
        pub use binex::prelude::{Message, Meta};
    }

    #[cfg(feature = "clock")]
    #[cfg_attr(docsrs, doc(cfg(feature = "clock")))]
    pub mod clock {
        pub use crate::clock::{ClockKey, ClockProfile, ClockProfileType, ClockType, WorkClock};
    }

    #[cfg(feature = "nav")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nav")))]
    pub mod nav {
        pub use anise::{
            astro::AzElRange,
            errors::AlmanacResult,
            prelude::{Almanac, Frame, Orbit},
        };
    }

    #[cfg(feature = "ut1")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ut1")))]
    pub mod ut1 {
        pub use hifitime::ut1::{DeltaTaiUt1, Ut1Provider};
    }

    #[cfg(feature = "qc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "qc")))]
    pub mod qc {
        pub use qc_traits::{Merge, MergeError};
    }

    #[cfg(feature = "processing")]
    #[cfg_attr(docsrs, doc(cfg(feature = "processing")))]
    pub mod processing {
        pub use qc_traits::{
            Decimate, DecimationFilter, Filter, MaskFilter, Masking, Preprocessing, Split,
            TimeCorrection, TimeCorrectionError, TimeCorrectionsDB, Timeshift,
        };
    }

    #[cfg(feature = "binex")]
    #[cfg_attr(docsrs, doc(cfg(feature = "binex")))]
    pub use crate::binex::BIN2RNX;

    #[cfg(feature = "rtcm")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rtcm")))]
    pub use crate::rtcm::RTCM2RNX;
}

/// Package dedicated to file production.
pub mod prod {
    pub use crate::production::{
        DataSource, DetailedProductionAttributes, ProductionAttributes, FFU, PPU,
    };
}

use carrier::Carrier;
use prelude::*;

#[cfg(feature = "processing")]
use qc_traits::{MaskFilter, Masking};

#[cfg(feature = "processing")]
use crate::{
    clock::record::clock_mask_mut, doris::mask::mask_mut as doris_mask_mut,
    header::processing::header_mask_mut, ionex::mask_mut as ionex_mask_mut,
    meteo::mask::mask_mut as meteo_mask_mut, navigation::mask::mask_mut as navigation_mask_mut,
    observation::mask::mask_mut as observation_mask_mut,
};

#[cfg(docsrs)]
pub use bibliography::Bibliography;

/*
 * returns true if given line is a comment
 */
pub(crate) fn is_rinex_comment(content: &str) -> bool {
    content.len() > 60 && content.trim_end().ends_with("COMMENT")
}

/*
 * macro to format one header line or a comment
 */
pub(crate) fn fmt_rinex(content: &str, marker: &str) -> String {
    if content.len() < 60 {
        format!("{:<padding$}{}", content, marker, padding = 60)
    } else {
        let mut string = String::new();
        let nb_lines = num_integer::div_ceil(content.len(), 60);
        for i in 0..nb_lines {
            let start_off = i * 60;
            let end_off = std::cmp::min(start_off + 60, content.len());
            let chunk = &content[start_off..end_off];
            string.push_str(&format!("{:<padding$}{}", chunk, marker, padding = 60));
            if i < nb_lines - 1 {
                string.push('\n');
            }
        }
        string
    }
}

/*
 * macro to generate comments with standardized formatting
 */
pub(crate) fn fmt_comment(content: &str) -> String {
    fmt_rinex(content, "COMMENT")
}

#[derive(Clone, Debug)]
/// [Rinex] comprises a [Header] and a [Record] section.
/// ```
/// use rinex::prelude::*;
/// let rnx = Rinex::from_file("data/OBS/V2/delf0010.21o")
///     .unwrap();
/// // header contains high level information
/// // like file standard revision:
/// assert_eq!(rnx.header.version.major, 2);
/// assert_eq!(rnx.header.version.minor, 11);
///
/// let marker = rnx.header.geodetic_marker
///         .as_ref()
///         .unwrap();
///
/// assert_eq!(marker.number(), Some("13502M004".to_string()));
///
/// // Constellation describes which kind of vehicles
/// // are to be encountered in the record, or which
/// // GNSS constellation the data will be referred to.
/// // Mixed constellation, means a combination of vehicles or
/// // GNSS constellations is expected
/// assert_eq!(rnx.header.constellation, Some(Constellation::Mixed));
/// // Some information on the hardware being used might be stored
/// println!("{:#?}", rnx.header.rcvr);
/// // comments encountered in the Header section
/// println!("{:#?}", rnx.header.comments);
/// // sampling interval was set
/// assert_eq!(rnx.header.sampling_interval, Some(Duration::from_seconds(30.0))); // 30s sample rate
/// // record content is RINEX format dependent.
/// ```
pub struct Rinex {
    /// [Header] gives general information and describes following content.
    pub header: Header,
    /// [Comments] stored as they appeared in file body
    pub comments: Comments,
    /// [Record] is the actual file content and is heavily [RinexType] dependent
    pub record: Record,
    /// [ProductionAttributes] filled
    pub production: ProductionAttributes,
}

impl Rinex {
    /// Builds a new [Rinex] struct from given header & body sections.
    pub fn new(header: Header, record: record::Record) -> Rinex {
        Rinex {
            header,
            record,
            comments: Comments::new(),
            production: ProductionAttributes::default(),
        }
    }

    /// Builds a default Navigation [Rinex], useful in data production context.
    pub fn basic_nav() -> Self {
        Self {
            header: Header::basic_nav(),
            comments: Default::default(),
            production: ProductionAttributes::default(),
            record: Record::NavRecord(Default::default()),
        }
    }

    /// Builds a default Observation [Rinex], useful in data production context.
    pub fn basic_obs() -> Self {
        Self {
            header: Header::basic_obs(),
            comments: Default::default(),
            production: ProductionAttributes::default(),
            record: Record::ObsRecord(Default::default()),
        }
    }

    /// Builds a default Observation [CRINEX], useful in data production context.
    pub fn basic_crinex() -> Self {
        Self {
            comments: Default::default(),
            header: Header::basic_crinex(),
            production: ProductionAttributes::default(),
            record: Record::ObsRecord(Default::default()),
        }
    }

    /// Copy and return this [Rinex] with updated [Header].
    pub fn with_header(&self, header: Header) -> Self {
        Self {
            header,
            record: self.record.clone(),
            comments: self.comments.clone(),
            production: self.production.clone(),
        }
    }

    /// Replace [Header] with mutable access.
    pub fn replace_header(&mut self, header: Header) {
        self.header = header.clone();
    }

    /// Copy and return this [Rinex] with updated [Record]
    pub fn with_record(&self, record: Record) -> Self {
        Rinex {
            record,
            header: self.header.clone(),
            comments: self.comments.clone(),
            production: self.production.clone(),
        }
    }

    /// Replace [Record] with mutable access.
    pub fn replace_record(&mut self, record: Record) {
        self.record = record.clone();
    }

    /// Converts self to CRINEX (compressed RINEX) format.
    /// If current revision is < 3 then file gets converted to CRINEX1
    /// format, otherwise, modern Observations are converted to CRINEX3.
    /// This has no effect if self is not an Observation RINEX.
    ///
    /// ```
    /// use rinex::prelude::*;
    /// let rinex = Rinex::from_file("data/OBS/V3/DUTH0630.22O")
    ///     .unwrap();
    ///
    /// // convert to CRINEX
    /// let crinex = rinex.rnx2crnx();
    /// assert!(crinex.to_file("test.crx").is_ok());
    /// ```
    pub fn rnx2crnx(&self) -> Self {
        let mut s = self.clone();
        s.rnx2crnx_mut();
        s
    }

    /// Mutable [Self::rnx2crnx] implementation
    pub fn rnx2crnx_mut(&mut self) {
        if self.is_observation_rinex() {
            let mut crinex = CRINEX::default();
            crinex.version.major = match self.header.version.major {
                1 | 2 => 1,
                _ => 3,
            };
            crinex.date = epoch::now();
            crinex.prog = format!(
                "rs-rinex v{}",
                Header::format_pkg_version(env!("CARGO_PKG_VERSION"))
            );
            self.header = self.header.with_crinex(crinex);
        }
    }

    /// Copies and convert this supposedly Compact (compressed) [Rinex] into
    /// readable [Rinex]. This has no effect if this [Rinex] is not a compressed Observation RINEX.
    pub fn crnx2rnx(&self) -> Self {
        let mut s = self.clone();
        s.crnx2rnx_mut();
        s
    }

    /// [Rinex::crnx2rnx] mutable implementation
    pub fn crnx2rnx_mut(&mut self) {
        if self.is_observation_rinex() {
            let params = self.header.obs.as_ref().unwrap();
            self.header = self
                .header
                .with_observation_fields(observation::HeaderFields {
                    crinex: None,
                    codes: params.codes.clone(),
                    clock_offset_applied: params.clock_offset_applied,
                    scaling: params.scaling.clone(),
                    timeof_first_obs: params.timeof_first_obs,
                    timeof_last_obs: params.timeof_last_obs,
                });

            self.header.program = Some(format!(
                "rs-rinex v{}",
                Header::format_pkg_version(env!("CARGO_PKG_VERSION"))
            ));
        }
    }

    /// Returns a file name that would describe this [Rinex] according to standard naming conventions.
    /// For this information to be 100% complete, this [Rinex] must originate a file that
    /// followed standard naming conventions itself.
    ///
    /// Otherwise you must provide [ProductionAttributes] yourself with "custom" values
    /// to fullfil the remaining fields.
    ///
    /// In any case, this method is infaillible: we will always generate something,
    /// missing fields are blanked.
    ///
    /// NB: this method
    ///  - generates an upper case [String] as per standard conventions.
    ///  - prefers lengthy (V3) names as opposed to short (V2) file names,
    /// when applied to Observation, Navigation and Meteo formats.
    /// Use "short" to change that default behavior.
    ///  - you can use "suffix" to append a custom suffix to the standard name right away.
    /// ```
    /// use rinex::prelude::*;
    /// // Parse a File that follows standard naming conventions
    /// // and verify we generate something correct
    /// ```
    pub fn standard_filename(
        &self,
        short: bool,
        suffix: Option<&str>,
        custom: Option<ProductionAttributes>,
    ) -> String {
        let header = &self.header;
        let rinextype = header.rinex_type;
        let is_crinex = header.is_crinex();
        let constellation = header.constellation;

        let mut filename = match rinextype {
            RinexType::IonosphereMaps => {
                let name = match custom {
                    Some(ref custom) => {
                        custom.name[..std::cmp::min(3, custom.name.len())].to_string()
                    },
                    None => self.production.name.clone(),
                };
                let region = match &custom {
                    Some(ref custom) => custom.region.unwrap_or('G'),
                    None => self.production.region.unwrap_or('G'),
                };
                let ddd = match &custom {
                    Some(ref custom) => format!("{:03}", custom.doy),
                    None => {
                        if let Some(epoch) = self.first_epoch() {
                            let ddd = epoch.day_of_year().round() as u32;
                            format!("{:03}", ddd)
                        } else {
                            format!("{:03}", self.production.doy)
                        }
                    },
                };
                let yy = match &custom {
                    Some(ref custom) => format!("{:02}", custom.year - 2_000),
                    None => {
                        if let Some(epoch) = self.first_epoch() {
                            let yy = epoch_decompose(epoch).0;
                            format!("{:02}", yy - 2_000)
                        } else {
                            format!("{:02}", self.production.year - 2_000)
                        }
                    },
                };
                ProductionAttributes::ionex_format(&name, region, &ddd, &yy)
            },
            RinexType::ObservationData | RinexType::MeteoData | RinexType::NavigationData => {
                let name = match custom {
                    Some(ref custom) => custom.name.clone(),
                    None => self.production.name.clone(),
                };
                let ddd = match &custom {
                    Some(ref custom) => format!("{:03}", custom.doy),
                    None => {
                        if let Some(epoch) = self.first_epoch() {
                            let ddd = epoch.day_of_year().round() as u32;
                            format!("{:03}", ddd)
                        } else {
                            "DDD".to_string()
                        }
                    },
                };
                if short {
                    let yy = match &custom {
                        Some(ref custom) => format!("{:02}", custom.year - 2_000),
                        None => {
                            if let Some(epoch) = self.first_epoch() {
                                let yy = epoch_decompose(epoch).0;
                                format!("{:02}", yy - 2_000)
                            } else {
                                "YY".to_string()
                            }
                        },
                    };
                    let ext = match rinextype {
                        RinexType::ObservationData => {
                            if is_crinex {
                                'D'
                            } else {
                                'O'
                            }
                        },
                        RinexType::MeteoData => 'M',
                        RinexType::NavigationData => match constellation {
                            Some(Constellation::Glonass) => 'G',
                            _ => 'N',
                        },
                        _ => unreachable!("unreachable"),
                    };
                    ProductionAttributes::rinex_short_format(&name, &ddd, &yy, ext)
                } else {
                    /* long /V3 like format */
                    let batch = match &custom {
                        Some(ref custom) => {
                            if let Some(details) = &custom.v3_details {
                                details.batch
                            } else {
                                0
                            }
                        },
                        None => {
                            if let Some(details) = &self.production.v3_details {
                                details.batch
                            } else {
                                0
                            }
                        },
                    };

                    let country = match &custom {
                        Some(ref custom) => {
                            if let Some(details) = &custom.v3_details {
                                details.country.to_string()
                            } else {
                                "CCC".to_string()
                            }
                        },
                        None => {
                            if let Some(details) = &self.production.v3_details {
                                details.country.to_string()
                            } else {
                                "CCC".to_string()
                            }
                        },
                    };

                    let src = match &header.rcvr {
                        Some(_) => 'R', // means GNSS rcvr
                        None => {
                            if let Some(details) = &self.production.v3_details {
                                details.data_src.to_char()
                            } else {
                                'U' // means: unspecified
                            }
                        },
                    };

                    let yyyy = match &custom {
                        Some(ref custom) => format!("{:04}", custom.year),
                        None => {
                            if let Some(t0) = self.first_epoch() {
                                let yy = epoch_decompose(t0).0;
                                format!("{:04}", yy)
                            } else {
                                "YYYY".to_string()
                            }
                        },
                    };

                    let (hh, mm) = match &custom {
                        Some(ref custom) => {
                            if let Some(details) = &custom.v3_details {
                                (format!("{:02}", details.hh), format!("{:02}", details.mm))
                            } else {
                                ("HH".to_string(), "MM".to_string())
                            }
                        },
                        None => {
                            if let Some(epoch) = self.first_epoch() {
                                let (_, _, _, hh, mm, _, _) = epoch_decompose(epoch);
                                (format!("{:02}", hh), format!("{:02}", mm))
                            } else {
                                ("HH".to_string(), "MM".to_string())
                            }
                        },
                    };

                    // FFU sampling rate
                    let ffu = match self.dominant_sampling_interval() {
                        Some(duration) => FFU::from(duration).to_string(),
                        None => {
                            if let Some(ref custom) = custom {
                                if let Some(details) = &custom.v3_details {
                                    if let Some(ffu) = details.ffu {
                                        ffu.to_string()
                                    } else {
                                        "XXX".to_string()
                                    }
                                } else {
                                    "XXX".to_string()
                                }
                            } else {
                                "XXX".to_string()
                            }
                        },
                    };

                    // ffu only in OBS file names
                    let ffu = match rinextype {
                        RinexType::ObservationData => Some(ffu),
                        _ => None,
                    };

                    // PPU periodicity
                    let ppu = match custom {
                        Some(custom) => {
                            if let Some(details) = &custom.v3_details {
                                details.ppu
                            } else {
                                PPU::Unspecified
                            }
                        },
                        None => {
                            if let Some(details) = &self.production.v3_details {
                                details.ppu
                            } else {
                                PPU::Unspecified
                            }
                        },
                    };

                    let fmt = match rinextype {
                        RinexType::ObservationData => "MO".to_string(),
                        RinexType::MeteoData => "MM".to_string(),
                        RinexType::NavigationData => match constellation {
                            Some(Constellation::Mixed) | None => "MN".to_string(),
                            Some(constell) => format!("M{:x}", constell),
                        },
                        _ => unreachable!("unreachable fmt"),
                    };

                    let ext = if is_crinex { "crx" } else { "rnx" };

                    ProductionAttributes::rinex_long_format(
                        &name,
                        batch,
                        &country,
                        src,
                        &yyyy,
                        &ddd,
                        &hh,
                        &mm,
                        &ppu.to_string(),
                        ffu.as_deref(),
                        &fmt,
                        ext,
                    )
                }
            },
            rinex => unimplemented!("{} format", rinex),
        };
        if let Some(suffix) = suffix {
            filename.push_str(suffix);
        }
        filename
    }

    /// Guesses File [ProductionAttributes] from the actual Record content.
    /// This is particularly useful when working with datasets we are confident about,
    /// yet that do not follow standard naming conventions.
    /// Note that this method is infaillible, because we default to blank fields
    /// in case we cannot retrieve them.
    pub fn guess_production_attributes(&self) -> ProductionAttributes {
        // start from content identified from the filename
        let mut attributes = self.production.clone();

        let first_epoch = self.first_epoch();
        let last_epoch = self.last_epoch();
        let first_epoch_gregorian = first_epoch.map(|t0| t0.to_gregorian_utc());

        match first_epoch_gregorian {
            Some((y, _, _, _, _, _, _)) => attributes.year = y as u32,
            _ => {},
        }
        match first_epoch {
            Some(t0) => attributes.doy = t0.day_of_year().round() as u32,
            _ => {},
        }

        // notes on attribute."name"
        // - Non detailed OBS RINEX: this is usually the station name
        //   which can be named after a geodetic marker
        // - Non detailed NAV RINEX: station name
        // - CLK RINEX: name of the local clock
        // - IONEX: agency
        match self.header.rinex_type {
            RinexType::ClockData => match &self.header.clock {
                Some(clk) => match &clk.ref_clock {
                    Some(refclock) => attributes.name = refclock.to_string(),
                    _ => {
                        if let Some(site) = &clk.site {
                            attributes.name = site.to_string();
                        } else {
                            if let Some(agency) = &self.header.agency {
                                attributes.name = agency.to_string();
                            }
                        }
                    },
                },
                _ => {
                    if let Some(agency) = &self.header.agency {
                        attributes.name = agency.to_string();
                    }
                },
            },
            RinexType::IonosphereMaps => {
                if let Some(agency) = &self.header.agency {
                    attributes.name = agency.to_string();
                }
            },
            _ => match &self.header.geodetic_marker {
                Some(marker) => attributes.name = marker.name.to_string(),
                _ => {
                    if let Some(agency) = &self.header.agency {
                        attributes.name = agency.to_string();
                    }
                },
            },
        }

        if let Some(ref mut details) = attributes.v3_details {
            if let Some((_, _, _, hh, mm, _, _)) = first_epoch_gregorian {
                details.hh = hh;
                details.mm = mm;
            }
            if let Some(first_epoch) = first_epoch {
                if let Some(last_epoch) = last_epoch {
                    let total_dt = last_epoch - first_epoch;
                    details.ppu = PPU::from(total_dt);
                }
            }
        } else {
            attributes.v3_details = Some(DetailedProductionAttributes {
                batch: 0,                      // see notes down below
                country: "XXX".to_string(),    // see notes down below
                data_src: DataSource::Unknown, // see notes down below
                ppu: match (first_epoch, last_epoch) {
                    (Some(first), Some(last)) => {
                        let total_dt = last - first;
                        PPU::from(total_dt)
                    },
                    _ => PPU::Unspecified,
                },
                ffu: self.dominant_sampling_interval().map(FFU::from),
                hh: match first_epoch_gregorian {
                    Some((_, _, _, hh, _, _, _)) => hh,
                    _ => 0,
                },
                mm: match first_epoch_gregorian {
                    Some((_, _, _, _, mm, _, _)) => mm,
                    _ => 0,
                },
            });
        }
        /*
         * Several fields cannot be deduced from the actual
         * Record content. If provided filename did not describe them,
         * we have no means to recover them.
         * Example of such fields would be:
         *    + Country Code: would require a worldwide country database
         *    + Data source: is only defined in the filename
         */
        attributes
    }

    /// Parse [RINEX] content by consuming [BufReader] (efficient buffered reader).
    /// Attributes potentially described by a file name need to be provided either
    /// manually / externally, or guessed when parsing has been completed.
    pub fn parse<R: Read>(reader: &mut BufReader<R>) -> Result<Self, ParsingError> {
        // Parses Header section (=consumes header until this point)
        let mut header = Header::parse(reader)?;

        // Parse record (=consumes rest of this resource)
        // Comments are preserved and store "as is"
        let (record, comments) = Record::parse(&mut header, reader)?;

        Ok(Self {
            header,
            comments,
            record,
            production: Default::default(),
        })
    }

    /// Format [RINEX] into writable I/O using efficient buffered writer
    /// and following standard specifications. The revision to be followed is defined
    /// in [Header] section. This is the mirror operation of [Self::parse].
    pub fn format<W: Write>(&self, writer: &mut BufWriter<W>) -> Result<(), FormattingError> {
        self.header.format(writer)?;
        self.record.format(writer, &self.header)?;
        writer.flush()?;
        Ok(())
    }

    /// Parses [Rinex] from local readable file.
    /// Will panic if provided file does not exist or is not readable.
    /// See [Self::from_gzip_file] for seamless Gzip support.
    ///
    /// If file name follows standard naming conventions, then internal definitions
    /// will truly be complete. Otherwise [ProductionAttributes] cannot be fully determined.
    /// If you want or need to you can either
    ///  1. define it yourself with further customization
    ///  2. use the smart guesser (after parsing): [Self::guess_production_attributes]
    ///
    /// This is typically needed in data production contexts.
    ///
    /// The parser automatically picks up the RINEX format and we support
    /// all of them, CRINEX (Compat RINEX) is natively supported.
    /// NB: the SINEX format is different and handled in a dedicated library.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Rinex, ParsingError> {
        let path = path.as_ref();

        // deduce all we can from file name
        let file_attributes = match path.file_name() {
            Some(filename) => {
                let filename = filename.to_string_lossy().to_string();
                if let Ok(prod) = ProductionAttributes::from_str(&filename) {
                    prod
                } else {
                    ProductionAttributes::default()
                }
            },
            _ => ProductionAttributes::default(),
        };

        let fd = File::open(path).expect("from_file: open error");

        let mut reader = BufReader::new(fd);
        let mut rinex = Self::parse(&mut reader)?;
        rinex.production = file_attributes;
        Ok(rinex)
    }

    /// Dumps [RINEX] into writable local file (as readable ASCII UTF-8)
    /// using efficient buffered formatting.
    /// This is the mirror operation of [Self::from_file].
    /// Returns total amount of bytes that was generated.
    /// ```
    /// // Read a RINEX and dump it without any modifications
    /// use rinex::prelude::*;
    /// let rnx = Rinex::from_file("data/OBS/V3/DUTH0630.22O")
    ///   .unwrap();
    /// assert!(rnx.to_file("test.rnx").is_ok());
    /// ```
    ///
    /// Other useful links are in data production contexts:
    ///   * [Self::standard_filename] to generate a standardized filename
    ///   * [Self::guess_production_attributes] helps generate standardized filenames for
    ///     files that do not follow naming conventions
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), FormattingError> {
        let fd = File::create(path)?;
        let mut writer = BufWriter::new(fd);
        self.format(&mut writer)?;
        Ok(())
    }

    /// Parses [Rinex] from local gzip compressed file.
    /// Will panic if provided file does not exist or is not readable.
    /// Refer to [Self::from_file] for more information.
    ///
    /// IONEX example:
    /// ```
    /// use rinex::prelude::Rinex;
    ///
    /// let rinex = Rinex::from_gzip_file("data/IONEX/V1/CKMG0020.22I.gz")
    ///     .unwrap();
    ///
    /// assert!(rinex.is_ionex());
    /// assert!(rinex.is_ionex_2d());
    ///
    /// let params = rinex.header.ionex
    ///     .as_ref()
    ///     .unwrap();
    ///
    /// // fixed altitude IONEX (=single isosurface)
    /// assert_eq!(params.grid.height.start, 350.0);
    /// assert_eq!(params.grid.height.end, 350.0);
    ///     
    /// // latitude grid
    /// assert_eq!(params.grid.latitude.start, 87.5);
    /// assert_eq!(params.grid.latitude.end, -87.5);
    /// assert_eq!(params.grid.latitude.spacing, -2.5);
    ///
    /// // longitude grid
    /// assert_eq!(params.grid.longitude.start, -180.0);
    /// assert_eq!(params.grid.longitude.end, 180.0);
    /// assert_eq!(params.grid.longitude.spacing, 5.0);

    /// assert_eq!(params.elevation_cutoff, 0.0);
    /// assert_eq!(params.mapping, None);
    /// ```
    #[cfg(feature = "flate2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "flate2")))]
    pub fn from_gzip_file<P: AsRef<Path>>(path: P) -> Result<Rinex, ParsingError> {
        let path = path.as_ref();

        // deduce all we can from file name
        let file_attributes = match path.file_name() {
            Some(filename) => {
                let filename = filename.to_string_lossy().to_string();
                if let Ok(prod) = ProductionAttributes::from_str(&filename) {
                    prod
                } else {
                    ProductionAttributes::default()
                }
            },
            _ => ProductionAttributes::default(),
        };

        let fd = File::open(path).expect("from_file: open error");

        let reader = GzDecoder::new(fd);
        let mut reader = BufReader::new(reader);
        let mut rinex = Self::parse(&mut reader)?;
        rinex.production = file_attributes;
        Ok(rinex)
    }

    /// Dumps and gzip encodes [RINEX] into writable local file,
    /// using efficient buffered formatting.
    /// This is the mirror operation of [Self::from_gzip_file].
    /// Returns total amount of bytes that was generated.
    /// ```
    /// // Read a RINEX and dump it without any modifications
    /// use rinex::prelude::*;
    /// let rnx = Rinex::from_file("data/OBS/V3/DUTH0630.22O")
    ///   .unwrap();
    /// assert!(rnx.to_file("test.rnx").is_ok());
    /// ```
    ///
    /// Other useful links are in data production contexts:
    ///   * [Self::standard_filename] to generate a standardized filename
    ///   * [Self::guess_production_attributes] helps generate standardized filenames for
    ///     files that do not follow naming conventions
    #[cfg(feature = "flate2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "flate2")))]
    pub fn to_gzip_file<P: AsRef<Path>>(&self, path: P) -> Result<(), FormattingError> {
        let fd = File::create(path)?;
        let compression = GzCompression::new(5);
        let mut writer = BufWriter::new(GzEncoder::new(fd, compression));
        self.format(&mut writer)?;
        Ok(())
    }

    /// Returns true if this is an ATX RINEX
    pub fn is_antex(&self) -> bool {
        self.header.rinex_type == types::Type::AntennaData
    }

    /// Returns true if this is a CLOCK RINEX
    pub fn is_clock_rinex(&self) -> bool {
        self.header.rinex_type == types::Type::ClockData
    }

    /// Returns true if Differential Code Biases (DCBs)
    /// are compensated for, in this file, for this GNSS constellation.
    /// DCBs are biases due to tiny frequency differences,
    /// in both the SV embedded code generator, and receiver PLL.
    /// If this is true, that means all code signals received in from
    /// all SV within that constellation, have intrinsinc DCB compensation.
    /// In very high precision and specific applications, you then do not have
    /// to deal with their compensation yourself.
    pub fn dcb_compensation(&self, constellation: Constellation) -> bool {
        self.header
            .dcb_compensations
            .iter()
            .filter(|dcb| dcb.constellation == constellation)
            .count()
            > 0
    }
    /// Returns true if Antenna Phase Center variations are compensated
    /// for in this file. Useful for high precision application.
    pub fn pcv_compensation(&self, constellation: Constellation) -> bool {
        self.header
            .pcv_compensations
            .iter()
            .filter(|pcv| pcv.constellation == constellation)
            .count()
            > 0
    }

    /// Determines whether [Rinex] is the result of a previous [Merge] operation.
    /// That is, the combination of two files merged together.  
    /// This is determined by the presence of custom yet somewhat standardized [Comments].
    pub fn is_merged(&self) -> bool {
        let special_comment = String::from("FILE MERGE");
        for comment in self.header.comments.iter() {
            if comment.contains(&special_comment) {
                return true;
            }
        }
        false
    }
}

/*
 * Methods that return an Iterator exclusively.
 * These methods are used to browse data easily and efficiently.
 */
impl Rinex {
    /// Returns [Epoch] Iterator. This applies to all but ANTEX special format,
    /// for which we return null.
    pub fn epoch_iter(&self) -> Box<dyn Iterator<Item = Epoch> + '_> {
        if let Some(r) = self.record.as_obs() {
            Box::new(r.iter().map(|(k, _)| k.epoch))
        } else if let Some(r) = self.record.as_meteo() {
            Box::new(r.iter().map(|(k, _)| k.epoch).unique())
        } else if let Some(r) = self.record.as_doris() {
            Box::new(r.iter().map(|(k, _)| k.epoch))
        } else if let Some(r) = self.record.as_nav() {
            Box::new(r.iter().map(|(k, _)| k.epoch))
        } else if let Some(r) = self.record.as_clock() {
            Box::new(r.iter().map(|(k, _)| *k))
        } else if let Some(r) = self.record.as_ionex() {
            Box::new(r.iter().map(|(k, _)| k.epoch).unique())
        } else {
            Box::new([].into_iter())
        }
    }

    /// Returns a [SV] iterator, from all satellites encountered in this [Rinex].
    pub fn sv_iter(&self) -> Box<dyn Iterator<Item = SV> + '_> {
        if self.is_observation_rinex() {
            Box::new(
                self.signal_observations_iter()
                    .map(|(_, v)| v.sv)
                    .sorted()
                    .unique(),
            )
        } else if let Some(record) = self.record.as_nav() {
            Box::new(record.iter().map(|(k, _)| k.sv).sorted().unique())
        } else if let Some(record) = self.record.as_clock() {
            Box::new(
                // grab all embedded sv clocks
                record
                    .iter()
                    .flat_map(|(_, keys)| {
                        keys.iter()
                            .filter_map(|(key, _)| key.clock_type.as_sv())
                            .collect::<Vec<_>>()
                            .into_iter()
                    })
                    .unique(),
            )
        } else {
            Box::new([].into_iter())
        }
    }

    // /// List all [`SV`] per epoch of appearance.
    // /// ```
    // /// use rinex::prelude::*;
    // /// use std::str::FromStr;
    // /// let rnx = Rinex::from_file("data/OBS/V2/aopr0010.17o")
    // ///     .unwrap();
    // ///
    // /// let mut data = rnx.sv_epoch();
    // ///
    // /// if let Some((epoch, vehicles)) = data.nth(0) {
    // ///     assert_eq!(epoch, Epoch::from_str("2017-01-01T00:00:00 GPST").unwrap());
    // ///     let expected = vec![
    // ///         SV::new(Constellation::GPS, 03),
    // ///         SV::new(Constellation::GPS, 08),
    // ///         SV::new(Constellation::GPS, 14),
    // ///         SV::new(Constellation::GPS, 16),
    // ///         SV::new(Constellation::GPS, 22),
    // ///         SV::new(Constellation::GPS, 23),
    // ///         SV::new(Constellation::GPS, 26),
    // ///         SV::new(Constellation::GPS, 27),
    // ///         SV::new(Constellation::GPS, 31),
    // ///         SV::new(Constellation::GPS, 32),
    // ///     ];
    // ///     assert_eq!(*vehicles, expected);
    // /// }
    // /// ```
    // pub fn sv_epoch(&self) -> Box<dyn Iterator<Item = (Epoch, Vec<SV>)> + '_> {
    //     if let Some(record) = self.record.as_obs() {
    //         Box::new(
    //             record.iter().map(|((epoch, _), (_clk, entries))| {
    //                 (*epoch, entries.keys().unique().cloned().collect())
    //             }),
    //         )
    //     } else if let Some(record) = self.record.as_nav() {
    //         Box::new(
    //             // grab all vehicles through all epochs,
    //             // fold them into individual lists
    //             record.iter().map(|(epoch, frames)| {
    //                 (
    //                     *epoch,
    //                     frames
    //                         .iter()
    //                         .filter_map(|fr| {
    //                             if let Some((_, sv, _)) = fr.as_eph() {
    //                                 Some(sv)
    //                             } else if let Some((_, sv, _)) = fr.as_eop() {
    //                                 Some(sv)
    //                             } else if let Some((_, sv, _)) = fr.as_ion() {
    //                                 Some(sv)
    //                             } else if let Some((_, sv, _)) = fr.as_sto() {
    //                                 Some(sv)
    //                             } else {
    //                                 None
    //                             }
    //                         })
    //                         .fold(vec![], |mut list, sv| {
    //                             if !list.contains(&sv) {
    //                                 list.push(sv);
    //                             }
    //                             list
    //                         }),
    //                 )
    //             }),
    //         )
    //     } else {
    //         panic!(
    //             ".sv_epoch() is not feasible on \"{:?}\" RINEX",
    //             self.header.rinex_type
    //         );
    //     }
    // }

    /// Returns [Constellation]s Iterator.
    /// ```
    /// use rinex::prelude::*;
    /// use itertools::Itertools; // .sorted()
    /// let rnx = Rinex::from_file("data/OBS/V3/ACOR00ESP_R_20213550000_01D_30S_MO.rnx")
    ///     .unwrap();
    ///
    /// assert!(
    ///     rnx.constellations_iter().sorted().eq(
    ///         vec![
    ///             Constellation::GPS,
    ///             Constellation::Glonass,
    ///             Constellation::BeiDou,
    ///             Constellation::Galileo,
    ///         ]
    ///     ),
    ///     "parsed wrong GNSS context",
    /// );
    /// ```
    pub fn constellations_iter(&self) -> Box<dyn Iterator<Item = Constellation> + '_> {
        // Creates a unique list from .sv_iter()
        Box::new(self.sv_iter().map(|sv| sv.constellation).unique().sorted())
    }

    // /// Returns an Iterator over Unique Constellations, per Epoch
    // pub fn constellation_epoch(
    //     &self,
    // ) -> Box<dyn Iterator<Item = (Epoch, Vec<Constellation>)> + '_> {
    //     Box::new(self.sv_epoch().map(|(epoch, svnn)| {
    //         (
    //             epoch,
    //             svnn.iter().map(|sv| sv.constellation).unique().collect(),
    //         )
    //     }))
    // }

    /// Returns [Observable]s Iterator.
    /// Applies to Observation RINEX, Meteo RINEX and DORIS.
    /// Returns null for any other formats.  
    pub fn observables_iter(&self) -> Box<dyn Iterator<Item = &Observable> + '_> {
        if self.is_observation_rinex() {
            Box::new(
                self.signal_observations_iter()
                    .map(|(_, v)| &v.observable)
                    .unique()
                    .sorted(),
            )
        } else if self.is_meteo_rinex() {
            Box::new(
                self.meteo_observations_iter()
                    .map(|(k, _)| &k.observable)
                    .unique()
                    .sorted(),
            )
        // } else if self.record.as_doris().is_some() {
        //     Box::new(
        //         self.doris()
        //             .flat_map(|(_, stations)| {
        //                 stations
        //                     .iter()
        //                     .flat_map(|(_, observables)| observables.iter().map(|(k, _)| k))
        //             })
        //             .unique(),
        //     )
        } else {
            Box::new([].into_iter())
        }
    }

    /// ANTEX antennas specifications browsing
    pub fn antennas(
        &self,
    ) -> Box<dyn Iterator<Item = &(Antenna, HashMap<Carrier, FrequencyDependentData>)> + '_> {
        Box::new(
            self.record
                .as_antex()
                .into_iter()
                .flat_map(|record| record.iter()),
        )
    }

    /// Copies and returns new [Rinex] that is the result
    /// of observation differentiation. See [Self::observations_substract_mut] for more
    /// information.
    pub fn observations_substract(&self, rhs: &Self) -> Result<Self, Error> {
        let mut s = self.clone();
        s.observations_substract_mut(rhs)?;
        Ok(s)
    }

    /// Modifies [Rinex] in place with observation differentiation
    /// using the remote (RHS) counterpart, for each identical observation and signal source.
    ///
    /// This is currently limited to Observation RINEX. NB:
    /// - Only matched (differentiated) symbols will remain, any other observations are
    /// discarded.
    /// - Output symbols are not compliant with Observation RINEX, this is sort
    /// of like a "residual" RINEX. Use with care.
    ///
    /// This allows analyzing a local clock used as GNSS receiver reference clock
    /// spread to dual GNSS receiver, by means of phase differential analysis.
    pub fn observations_substract_mut(&mut self, rhs: &Self) -> Result<(), Error> {
        let lhs_dt = self
            .dominant_sampling_interval()
            .ok_or(Error::UndeterminedSamplingPeriod)?;

        let half_lhs_dt = lhs_dt / 2.0;

        if let Some(rhs) = rhs.record.as_obs() {
            if let Some(rec) = self.record.as_mut_obs() {
                rec.retain(|k, v| {
                    v.signals.retain_mut(|sig| {
                        let mut reference = 0.0;
                        let mut min_dt = Duration::MAX;

                        // temporal filter
                        let filtered_rhs_epochs = rhs.iter().filter(|(rhs, _)| {
                            let dt = (rhs.epoch - k.epoch).abs();
                            dt <= half_lhs_dt
                        });

                        for (rhs_epoch, rhs_values) in filtered_rhs_epochs {
                            for rhs_sig in rhs_values.signals.iter() {
                                if rhs_sig.sv == sig.sv && rhs_sig.observable == sig.observable {
                                    let dt = (rhs_epoch.epoch - k.epoch).abs();
                                    if dt <= min_dt {
                                        reference = rhs_sig.value;
                                        min_dt = dt;
                                    }
                                }
                            }
                        }

                        if min_dt < Duration::MAX {
                            sig.value -= reference;
                        }

                        min_dt < Duration::MAX
                    });

                    !v.signals.is_empty()
                });
            }
        }

        Ok(())
    }
}

#[cfg(feature = "processing")]
#[cfg_attr(docsrs, doc(cfg(feature = "processing")))]
impl Masking for Rinex {
    fn mask(&self, f: &MaskFilter) -> Self {
        let mut s = self.clone();
        s.mask_mut(f);
        s
    }
    fn mask_mut(&mut self, f: &MaskFilter) {
        header_mask_mut(&mut self.header, f);
        if let Some(rec) = self.record.as_mut_obs() {
            observation_mask_mut(rec, f);
        } else if let Some(rec) = self.record.as_mut_nav() {
            navigation_mask_mut(rec, f);
        } else if let Some(rec) = self.record.as_mut_clock() {
            clock_mask_mut(rec, f);
        } else if let Some(rec) = self.record.as_mut_meteo() {
            meteo_mask_mut(rec, f);
        } else if let Some(rec) = self.record.as_mut_doris() {
            doris_mask_mut(rec, f);
        } else if let Some(rec) = self.record.as_mut_ionex() {
            ionex_mask_mut(rec, f);
        }
    }
}

#[cfg(feature = "clock")]
use crate::clock::{ClockKey, ClockProfile, ClockProfileType};

/*
 * Clock RINEX specific feature
 */
#[cfg(feature = "clock")]
#[cfg_attr(docsrs, doc(cfg(feature = "clock")))]
impl Rinex {
    /// Returns Iterator over Clock RINEX content.
    pub fn precise_clock(
        &self,
    ) -> Box<dyn Iterator<Item = (&Epoch, &BTreeMap<ClockKey, ClockProfile>)> + '_> {
        Box::new(
            self.record
                .as_clock()
                .into_iter()
                .flat_map(|record| record.iter()),
        )
    }
    /// Returns Iterator over Clock RINEX content for Space Vehicles only (not ground stations).
    pub fn precise_sv_clock(
        &self,
    ) -> Box<dyn Iterator<Item = (Epoch, SV, ClockProfileType, ClockProfile)> + '_> {
        Box::new(self.precise_clock().flat_map(|(epoch, rec)| {
            rec.iter().filter_map(|(key, profile)| {
                key.clock_type
                    .as_sv()
                    .map(|sv| (*epoch, sv, key.profile_type.clone(), profile.clone()))
            })
        }))
    }
    /// Returns Iterator over Clock RINEX content for Ground Station clocks only (not onboard clocks)
    pub fn precise_station_clock(
        &self,
    ) -> Box<dyn Iterator<Item = (Epoch, String, ClockProfileType, ClockProfile)> + '_> {
        Box::new(self.precise_clock().flat_map(|(epoch, rec)| {
            rec.iter().filter_map(|(key, profile)| {
                key.clock_type.as_station().map(|clk_name| {
                    (
                        *epoch,
                        clk_name.clone(),
                        key.profile_type.clone(),
                        profile.clone(),
                    )
                })
            })
        }))
    }
}

/*
 * ANTEX specific feature
 */
#[cfg(feature = "antex")]
#[cfg_attr(docsrs, doc(cfg(feature = "antex")))]
impl Rinex {
    /// Iterates over antenna specifications that are still valid
    pub fn antex_valid_calibrations(
        &self,
        now: Epoch,
    ) -> Box<dyn Iterator<Item = (&Antenna, &HashMap<Carrier, FrequencyDependentData>)> + '_> {
        Box::new(self.antennas().filter_map(move |(ant, data)| {
            if ant.is_valid(now) {
                Some((ant, data))
            } else {
                None
            }
        }))
    }
    /// Returns APC offset for given spacecraft, expressed in NEU coordinates [mm] for given
    /// frequency. "now" is used to determine calibration validity (in time).
    pub fn sv_antenna_apc_offset(
        &self,
        now: Epoch,
        sv: SV,
        freq: Carrier,
    ) -> Option<(f64, f64, f64)> {
        self.antex_valid_calibrations(now)
            .filter_map(|(ant, freqdata)| match &ant.specific {
                AntennaSpecific::SvAntenna(sv_ant) => {
                    if sv_ant.sv == sv {
                        freqdata
                            .get(&freq)
                            .map(|freqdata| freqdata.apc_eccentricity)
                    } else {
                        None
                    }
                },
                _ => None,
            })
            .reduce(|k, _| k) // we're expecting a single match here
    }
    /// Returns APC offset for given RX Antenna model (ground station model).
    /// Model name is the IGS code, which has to match exactly but we're case insensitive.
    /// The APC offset is expressed in NEU coordinates
    /// [mm]. "now" is used to determine calibration validity (in time).
    pub fn rx_antenna_apc_offset(
        &self,
        now: Epoch,
        matcher: AntennaMatcher,
        freq: Carrier,
    ) -> Option<(f64, f64, f64)> {
        let to_match = matcher.to_lowercase();
        self.antex_valid_calibrations(now)
            .filter_map(|(ant, freqdata)| match &ant.specific {
                AntennaSpecific::RxAntenna(rx_ant) => match &to_match {
                    AntennaMatcher::IGSCode(code) => {
                        if rx_ant.igs_type.to_lowercase().eq(code) {
                            freqdata
                                .get(&freq)
                                .map(|freqdata| freqdata.apc_eccentricity)
                        } else {
                            None
                        }
                    },
                    AntennaMatcher::SerialNumber(sn) => {
                        if rx_ant.igs_type.to_lowercase().eq(sn) {
                            freqdata
                                .get(&freq)
                                .map(|freqdata| freqdata.apc_eccentricity)
                        } else {
                            None
                        }
                    },
                },
                _ => None,
            })
            .reduce(|k, _| k) // we're expecting a single match here
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{fmt_comment, is_rinex_comment};
    #[test]
    fn fmt_comments_singleline() {
        for desc in [
            "test",
            "just a basic comment",
            "just another lengthy comment blahblabblah",
        ] {
            let comment = fmt_comment(desc);
            assert!(
                comment.len() >= 60,
                "comments should be at least 60 byte long"
            );
            assert_eq!(
                comment.find("COMMENT"),
                Some(60),
                "comment marker should located @ 60"
            );
            assert!(is_rinex_comment(&comment), "should be valid comment");
        }
    }
    #[test]
    fn fmt_wrapped_comments() {
        for desc in ["just trying to form a very lengthy comment that will overflow since it does not fit in a single line",
            "just trying to form a very very lengthy comment that will overflow since it does fit on three very meaningful lines. Imazdmazdpoakzdpoakzpdokpokddddddddddddddddddaaaaaaaaaaaaaaaaaaaaaaa"] {
            let nb_lines = num_integer::div_ceil(desc.len(), 60);
            let comments = fmt_comment(desc);
            assert_eq!(comments.lines().count(), nb_lines);
            for line in comments.lines() {
                assert!(line.len() >= 60, "comment line should be at least 60 byte long");
                assert_eq!(line.find("COMMENT"), Some(60), "comment marker should located @ 60");
                assert!(is_rinex_comment(line), "should be valid comment");
            }
        }
    }
    #[test]
    fn fmt_observables_v3() {
        for (desc, expected) in [
("R    9 C1C L1C S1C C2C C2P L2C L2P S2C S2P",
"R    9 C1C L1C S1C C2C C2P L2C L2P S2C S2P                  SYS / # / OBS TYPES"),
("G   18 C1C L1C S1C C2P C2W C2S C2L C2X L2P L2W L2S L2L L2X         S2P S2W S2S S2L S2X",
"G   18 C1C L1C S1C C2P C2W C2S C2L C2X L2P L2W L2S L2L L2X  SYS / # / OBS TYPES
       S2P S2W S2S S2L S2X                                  SYS / # / OBS TYPES"),
        ] {
            assert_eq!(fmt_rinex(desc, "SYS / # / OBS TYPES"), expected);
        }
    }
}
