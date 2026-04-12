//! Receiver and antenna
use crate::{
    fmt_rinex,
    prelude::{FormattingError, COSPAR, SV},
};

use std::{
    io::{BufWriter, Write},
    str::FromStr,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "qc")]
use qc_traits::{html, Markup, QcHtmlReporting};

/// GNSS receiver description
#[derive(Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Receiver {
    /// Receiver (hardware) model
    pub model: String,
    /// Receiver (hardware) identification info
    pub sn: String, // serial #
    /// Receiver embedded software info
    pub firmware: String, // firmware #
}

impl Receiver {
    /// Formats [Receiver] into [BufWriter]
    pub(crate) fn format<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        writeln!(
            w,
            "{}",
            fmt_rinex(
                &format!("{:<20}{:<20}{}", self.sn, self.model, self.firmware),
                "REC # / TYPE / VERS"
            )
        )?;
        Ok(())
    }

    pub fn with_model(&self, model: &str) -> Self {
        let mut s = self.clone();
        s.model = model.to_string();
        s
    }

    pub fn with_serial_number(&self, sn: &str) -> Self {
        let mut s = self.clone();
        s.sn = sn.to_string();
        s
    }

    pub fn with_firmware(&self, firmware: &str) -> Self {
        let mut s = self.clone();
        s.firmware = firmware.to_string();
        s
    }
}

impl FromStr for Receiver {
    type Err = std::io::Error;
    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let (id, rem) = line.split_at(20);
        let (make, rem) = rem.split_at(20);
        let (version, _) = rem.split_at(20);
        Ok(Receiver {
            sn: id.trim().to_string(),
            model: make.trim().to_string(),
            firmware: version.trim().to_string(),
        })
    }
}

/// Antenna description
#[derive(Default, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Antenna {
    /// Hardware model / make descriptor
    pub model: String,
    /// Serial number / identification number
    pub sn: String,
    /// Base / reference point coordinates
    pub coords: Option<(f64, f64, f64)>,
    /// Optionnal `h` eccentricity (height component),
    /// referenced to base/reference point, in meter
    pub height: Option<f64>,
    /// Optionnal `eastern` eccentricity (eastern component),
    /// referenced to base/reference point, in meter
    pub eastern: Option<f64>,
    /// Optionnal `northern` eccentricity (northern component),
    /// referenced to base/reference point, in meter
    pub northern: Option<f64>,
}

impl Antenna {
    /// Formats [Antenna] into [BufWriter]
    pub(crate) fn format<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        writeln!(
            w,
            "{}",
            fmt_rinex(&format!("{:<20}{}", self.sn, self.model), "ANT # / TYPE")
        )?;
        if let Some(coords) = &self.coords {
            writeln!(
                w,
                "{}",
                fmt_rinex(
                    &format!("{:14.4}{:14.4}{:14.4}", coords.0, coords.1, coords.2),
                    "APPROX POSITION XYZ"
                )
            )?;
        }
        writeln!(
            w,
            "{}",
            fmt_rinex(
                &format!(
                    "{:14.4}{:14.4}{:14.4}",
                    self.height.unwrap_or(0.0),
                    self.eastern.unwrap_or(0.0),
                    self.northern.unwrap_or(0.0)
                ),
                "ANTENNA: DELTA H/E/N"
            )
        )?;
        Ok(())
    }

    /// Sets desired model
    pub fn with_model(&self, m: &str) -> Self {
        let mut s = self.clone();
        s.model = m.to_string();
        s
    }

    /// Sets desired Serial Number
    pub fn with_serial_number(&self, sn: &str) -> Self {
        let mut s = self.clone();
        s.sn = sn.to_string();
        s
    }

    /// Sets reference/base coordinates (3D)
    pub fn with_base_coordinates(&self, coords: (f64, f64, f64)) -> Self {
        let mut s = self.clone();
        s.coords = Some(coords);
        s
    }

    /// Sets antenna `h` eccentricity component
    pub fn with_height(&self, h: f64) -> Self {
        let mut s = self.clone();
        s.height = Some(h);
        s
    }

    /// Sets antenna `eastern` coordinates component
    pub fn with_eastern_component(&self, e: f64) -> Self {
        let mut s = self.clone();
        s.eastern = Some(e);
        s
    }

    /// Sets antenna `northern` coordiantes component
    pub fn with_northern_component(&self, n: f64) -> Self {
        let mut s = self.clone();
        s.northern = Some(n);
        s
    }
}

#[cfg(feature = "qc")]
impl QcHtmlReporting for Antenna {
    fn render(&self) -> Markup {
        html! {
            table class="table is-bordered" {
                tbody {
                    tr {
                        th {
                            "Model"
                        }
                        td {
                            (self.model.clone())
                        }
                    }
                    tr {
                        th {
                            "SN#"
                        }
                        td {
                            (self.sn.clone())
                        }
                    }
                    tr {
                        th {
                            "Base Coordinates"
                        }
                        td {
                            @if let Some(coords) = self.coords {
                                (format!("({}m, {}m, {}m) (ECEF)",
                                    coords.0, coords.1, coords.2))
                            } @else {
                                "Unknown"
                            }
                        }
                    }
                    th {
                        "Height"
                    }
                    td {
                        @if let Some(h) = self.height {
                            (format!("{} m", h))
                        } @else {
                            "Unknown"
                        }
                    }
                    th {
                        "Eccentricity"
                    }
                    td {
                        @if let Some(north) = self.northern {
                            @if let Some(east) = self.eastern {
                                (format!("{}m N, {}m E", north, east))
                            } @else {
                                "Unknown"
                            }
                        } @else {
                            "Unknown"
                        }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "qc")]
impl QcHtmlReporting for Receiver {
    fn render(&self) -> Markup {
        html! {
            div class="table-container" {
                table class="table is-bordered" {
                    tbody {
                        tr {
                            th {
                                "Model"
                            }
                            td {
                                (self.model.clone())
                            }
                        }
                        tr {
                            th {
                                "SN#"
                            }
                            td {
                                (self.sn.clone())
                            }
                        }
                        tr {
                            th {
                                "Firmware"
                            }
                            td {
                                (self.firmware.clone())
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Space vehicle antenna information,
/// only exists in ANTEX records
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SvAntenna {
    /// vehicle this antenna is attached to
    pub sv: SV,
    /// Antenna model description
    pub model: String,
    /// COSPAR launch ID code
    pub cospar: Option<COSPAR>,
}

impl SvAntenna {
    pub fn with_sv(&self, sv: SV) -> Self {
        let mut s = self.clone();
        s.sv = sv;
        s
    }
    pub fn with_model(&self, m: &str) -> Self {
        let mut s = self.clone();
        s.model = m.to_string();
        s
    }
    pub fn with_cospar(&self, c: &str) -> Self {
        let mut s = self.clone();
        if let Ok(cospar) = COSPAR::from_str(c) {
            s.cospar = Some(cospar);
        }
        s
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;
    #[test]
    fn rcvr_parser() {
        let content = "2090088             LEICA GR50          4.51                ";
        let rcvr = Receiver::from_str(content);
        assert!(rcvr.is_ok());
        let rcvr = rcvr.unwrap();
        assert_eq!(rcvr.model, "LEICA GR50");
        assert_eq!(rcvr.sn, "2090088");
        assert_eq!(rcvr.firmware, "4.51");
    }
}
