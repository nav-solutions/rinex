//! RINEX file Header

use crate::{
    antex::HeaderFields as AntexHeader,
    clock::HeaderFields as ClockHeader,
    hardware::{Antenna, Receiver, SvAntenna},
    hatanaka::CRINEX,
    leap::Leap,
    marker::GeodeticMarker,
    meteo::HeaderFields as MeteoHeader,
    navigation::HeaderFields as NavigationHeader,
    navigation::IonosphereModel,
    observation::HeaderFields as ObservationHeader,
    prelude::{Constellation, Duration, Epoch, COSPAR, SV},
    types::Type,
    version::Version,
};

use itertools::Itertools;
use std::collections::HashMap;

mod formatting;
mod parsing;

#[cfg(feature = "qc")]
mod qc;

#[cfg(feature = "processing")]
mod decim;

#[cfg(feature = "nav")]
use anise::{
    math::Vector6,
    prelude::{Frame, Orbit},
};

#[cfg(feature = "serde")]
use serde::Serialize;

#[cfg(feature = "processing")]
pub(crate) mod processing;

/// DCB compensation description
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DcbCompensation {
    /// Program used for DCBs evaluation and compensation
    pub program: String,
    /// Constellation to which this compensation applies to
    pub constellation: Constellation,
    /// URL: source of corrections
    pub url: String,
}

/// PCV compensation description
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PcvCompensation {
    /// Program used for PCVs evaluation and compensation
    pub program: String,
    /// Constellation to which this compensation applies to
    pub constellation: Constellation,
    /// URL: source of corrections
    pub url: String,
}

/// Describes `RINEX` file header
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Header {
    /// RINEX [Version]
    pub version: Version,

    /// RINEX [Type]
    pub rinex_type: Type,

    /// GNSS [Constellation] describing this entire file.
    pub constellation: Option<Constellation>,

    /// Comments from this section
    pub comments: Vec<String>,

    /// Possible software name (publishing software)
    pub program: Option<String>,

    /// Possible software operator
    pub run_by: Option<String>,

    /// Possible date of publication
    pub date: Option<String>,

    /// Possible station / agency URL
    pub station_url: Option<String>,

    /// Name of observer / operator
    pub observer: Option<String>,

    /// Production Agency
    pub agency: Option<String>,

    /// Possible [GeodeticMarker]
    pub geodetic_marker: Option<GeodeticMarker>,

    /// Glonass FDMA channels
    pub glo_channels: HashMap<SV, i8>,

    /// Possible COSPAR number (launch information)
    pub cospar: Option<COSPAR>,

    /// Possible [Leap] seconds counter
    pub leap: Option<Leap>,

    /// Approximate coordinates expressed in ECEF m
    pub rx_position: Option<(f64, f64, f64)>,

    /// Optionnal wavelength correction factors
    pub wavelengths: Option<(u32, u32)>,

    /// Possible sampling interval
    pub sampling_interval: Option<Duration>,

    /// Possible file license
    pub license: Option<String>,

    /// Possible Digital Object Identifier
    pub doi: Option<String>,

    /// Possible [Receiver] information
    #[cfg_attr(feature = "serde", serde(default))]
    pub rcvr: Option<Receiver>,

    /// Possible information about Receiver [Antenna]
    #[cfg_attr(feature = "serde", serde(default))]
    pub rcvr_antenna: Option<Antenna>,

    /// Possible information about satellite vehicle antenna.
    /// This only exists in ANTEX format.
    #[cfg_attr(feature = "serde", serde(default))]
    pub sv_antenna: Option<SvAntenna>,

    /// Possible Ionospheric Delay correction model, described in
    /// header section of old RINEX files (<V4).
    pub ionod_corrections: HashMap<Constellation, IonosphereModel>,

    /// Possible DCBs compensation information
    pub dcb_compensations: Vec<DcbCompensation>,

    /// Possible PCVs compensation information
    pub pcv_compensations: Vec<PcvCompensation>,

    /// Observation RINEX specific fields
    #[cfg_attr(feature = "serde", serde(default))]
    pub obs: Option<ObservationHeader>,

    /// NAV RINEX specific fields
    #[cfg_attr(feature = "serde", serde(default))]
    pub nav: Option<NavigationHeader>,

    /// Meteo RINEX specific fields
    #[cfg_attr(feature = "serde", serde(default))]
    pub meteo: Option<MeteoHeader>,

    /// High Precision Clock RINEX specific fields
    #[cfg_attr(feature = "serde", serde(default))]
    pub clock: Option<ClockHeader>,

    /// ANTEX specific fields
    #[cfg_attr(feature = "serde", serde(default))]
    pub antex: Option<AntexHeader>,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            version: Version::new(4, 0),
            rinex_type: Type::ObservationData,
            constellation: Some(Constellation::Mixed),
            program: Some(format!(
                "rs-rinex v{}",
                Self::format_pkg_version(env!("CARGO_PKG_VERSION"))
            )),
            obs: Some(Default::default()),
            date: Default::default(),
            dcb_compensations: Default::default(),
            license: Default::default(),
            comments: Default::default(),
            run_by: Default::default(),
            station_url: Default::default(),
            observer: Default::default(),
            agency: Default::default(),
            geodetic_marker: Default::default(),
            glo_channels: Default::default(),
            sampling_interval: None,
            leap: None,
            rx_position: None,
            wavelengths: None,
            cospar: None,
            doi: None,
            meteo: None,
            nav: None,
            clock: None,
            antex: None,
            rcvr: None,
            rcvr_antenna: None,
            sv_antenna: None,
            ionod_corrections: Default::default(),
            pcv_compensations: Default::default(),
        }
    }
}

impl Header {
    /// Returns true if this [Header] containts the special [CRINEX] marker
    pub fn is_crinex(&self) -> bool {
        if let Some(obs) = &self.obs {
            obs.crinex.is_some()
        } else {
            false
        }
    }

    /// Builds a basic [Header] to describe a Multi-GNSS Navigation RINEX
    pub fn basic_nav() -> Self {
        Self::default()
            .with_type(Type::NavigationData)
            .with_constellation(Constellation::Mixed)
    }

    /// Builds a basic [Header] to describe a Multi-GNSS Observation RINEX
    pub fn basic_obs() -> Self {
        Self::default()
            .with_type(Type::ObservationData)
            .with_constellation(Constellation::Mixed)
    }

    /// Builds a basic [Header] to describe a Multi-GNSS Compressed Observation RINEX
    pub fn basic_crinex() -> Self {
        Self::default()
            .with_type(Type::ObservationData)
            .with_constellation(Constellation::Mixed)
            .with_observation_fields(ObservationHeader::default().with_crinex(CRINEX::default()))
    }

    /// Formats the package version (possibly shortenned, in case of lengthy release)
    /// to fit within a formatted COMMENT
    pub(crate) fn format_pkg_version(version: &str) -> String {
        version
            .split('.')
            .enumerate()
            .filter_map(|(nth, v)| {
                if nth < 2 {
                    Some(v.to_string())
                } else if nth == 2 {
                    Some(
                        v.split('-')
                            .filter_map(|v| {
                                if v == "rc" {
                                    Some("rc".to_string())
                                } else {
                                    let mut s = String::new();
                                    s.push_str(&v[0..1]);
                                    Some(s)
                                }
                            })
                            .join(""),
                    )
                } else {
                    None
                }
            })
            .join(".")
    }

    /// Generates the special "FILE MERGE" comment
    pub(crate) fn merge_comment(pkg_version: &str, timestamp: Epoch) -> String {
        let formatted_version = Self::format_pkg_version(pkg_version);

        let (y, m, d, hh, mm, ss, _) = timestamp.to_gregorian_utc();
        format!(
            "rs-rinex v{} {:>width$}          {}{:02}{:02} {:02}{:02}{:02} {:x}",
            formatted_version,
            "FILE MERGE",
            y,
            m,
            d,
            hh,
            mm,
            ss,
            timestamp.time_scale,
            width = 19 - formatted_version.len(),
        )
    }

    /// Copies and returns [Header] with specific RINEX [Version].
    /// If this is part of a CRINEX (Compact RINEX) file, the specificities are automatically
    /// adapted.
    pub fn with_version(mut self, version: Version) -> Self {
        self.version = version;

        // automatically generates a correct CRINEX header
        if let Some(obs_specifics) = &mut self.obs {
            if let Some(crinex) = &mut obs_specifics.crinex {
                if version.major > 2 {
                    crinex.version.major = 3;
                } else {
                    crinex.version.major = 1;
                }
            }
        }

        self
    }

    /// Copies and returns [Header] with specific RINEX [Type]
    pub fn with_type(&self, t: Type) -> Self {
        let mut s = self.clone();
        s.rinex_type = t;
        s
    }

    /// Copies and returns [Header] with generate information
    pub fn with_general_information(&self, program: &str, run_by: &str, agency: &str) -> Self {
        let mut s = self.clone();
        s.program = Some(program.to_string());
        s.run_by = Some(run_by.to_string());
        s.agency = Some(agency.to_string());
        s
    }

    /// Copies and returns [Header] with special [CRINEX] fields
    pub(crate) fn with_crinex(&self, c: CRINEX) -> Self {
        let mut s = self.clone();
        if let Some(ref mut obs) = s.obs {
            obs.crinex = Some(c)
        } else {
            s.obs = Some(ObservationHeader::default().with_crinex(c));
        }
        s
    }

    /// Copies and returns [Header] with specific [Receiver] information
    pub fn with_receiver(&self, r: Receiver) -> Self {
        let mut s = self.clone();
        s.rcvr = Some(r);
        s
    }

    /// Copies and returns [Header] with specific [Antenna] information
    pub fn with_receiver_antenna(&self, a: Antenna) -> Self {
        let mut s = self.clone();
        s.rcvr_antenna = Some(a);
        s
    }

    /// Copies and returns [Header] modified to [Constellation]
    pub fn with_constellation(&self, c: Constellation) -> Self {
        let mut s = self.clone();
        s.constellation = Some(c);
        s
    }

    /// Copies and returns [Header] with a new comment
    pub fn with_comment(&self, c: &str) -> Self {
        let mut s = self.clone();
        s.comments.push(c.to_string());
        s
    }

    /// Copies and returns [Header] with following comments
    pub fn with_comments(&self, c: Vec<String>) -> Self {
        let mut s = self.clone();
        s.comments = c.clone();
        s
    }

    /// Copies and returns [Header] with specific [ObservationHeader]
    pub fn with_observation_fields(&self, fields: ObservationHeader) -> Self {
        let mut s = self.clone();
        s.obs = Some(fields);
        s
    }

    /// Converts approximate coordinates to an [Orbit]
    /// at given point in spacetime and using [Frame] model
    #[cfg(feature = "nav")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nav")))]
    pub fn rx_orbit(&self, t: Epoch, fr: Frame) -> Option<Orbit> {
        let (x_ecef_m, y_ecef_m, z_ecef_m) = self.rx_position?;

        let pos_vel = Vector6::new(
            x_ecef_m / 1000.0,
            y_ecef_m / 1000.0,
            z_ecef_m / 1000.0,
            0.0,
            0.0,
            0.0,
        );

        Some(Orbit::from_cartesian_pos_vel(pos_vel, t, fr))
    }

    /// Copies and define new rx position, from an [Orbit]
    #[cfg(feature = "nav")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nav")))]
    pub fn with_rx_orbit(&self, orbit: Orbit) -> Self {
        let mut s = self.clone();
        let state = orbit.to_cartesian_pos_vel();
        let (x_ecef_km, y_ecef_km, z_ecef_km) = (state[0], state[1], state[2]);
        s.rx_position = Some((x_ecef_km * 1.0E3, y_ecef_km * 1.0E3, z_ecef_km * 1.0E3));
        s
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::{Epoch, Header};
    use std::str::FromStr;

    #[test]
    fn test_merge_comment() {
        let j2000 = Epoch::from_str("2000-01-01T00:00:00 UTC").unwrap();

        for (pkg_version, formatted, comment) in [
            (
                "1.0.0",
                "1.0.0",
                "rs-rinex v1.0.0     FILE MERGE          20000101 000000 UTC",
            ),
            (
                "10.0.0",
                "10.0.0",
                "rs-rinex v10.0.0    FILE MERGE          20000101 000000 UTC",
            ),
            (
                "0.17.0",
                "0.17.0",
                "rs-rinex v0.17.0    FILE MERGE          20000101 000000 UTC",
            ),
            (
                "0.17.1",
                "0.17.1",
                "rs-rinex v0.17.1    FILE MERGE          20000101 000000 UTC",
            ),
            (
                "0.17.1-alpha",
                "0.17.1a",
                "rs-rinex v0.17.1a   FILE MERGE          20000101 000000 UTC",
            ),
            (
                "0.17.1-rc",
                "0.17.1rc",
                "rs-rinex v0.17.1rc  FILE MERGE          20000101 000000 UTC",
            ),
            (
                "0.17.1-rc-1",
                "0.17.1rc1",
                "rs-rinex v0.17.1rc1 FILE MERGE          20000101 000000 UTC",
            ),
        ] {
            assert_eq!(Header::format_pkg_version(pkg_version), formatted);

            let generated = Header::merge_comment(pkg_version, j2000);
            assert_eq!(generated, comment,);
        }
    }
}
