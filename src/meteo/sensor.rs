//! Meteo sensor
use crate::prelude::{Observable, ParsingError};

#[cfg(feature = "nav")]
use anise::{
    math::Vector6,
    prelude::{Epoch, Frame, Orbit},
};

#[cfg(feature = "qc")]
use maud::{html, Markup, Render};

/// Meteo Observation Sensor
#[derive(Default, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Sensor {
    /// Physics measured by this sensor
    pub observable: Observable,
    /// Model of this sensor
    pub model: Option<String>,
    /// Type of sensor
    pub sensor_type: Option<String>,
    /// Sensor accuracy [°C,..]
    pub accuracy: Option<f32>,
    /// Posible sensor location (ECEF)
    pub position: Option<(f64, f64, f64)>,
    /// Possible sensor height eccentricity (m)
    pub height: Option<f64>,
}

#[cfg(feature = "qc")]
impl Render for Sensor {
    fn render(&self) -> Markup {
        html! {
            table class="table is-bordered" {
                tr {
                    th { "Observable" }
                    td { (self.observable.to_string()) }
                }
                @if let Some(model) = &self.model {
                    tr {
                        th { "Model" }
                        td { (model) }
                    }
                }
                @if let Some(sensor) = &self.sensor_type {
                    tr {
                        th { "Sensor Type" }
                        td { (sensor) }
                    }
                }
                @if let Some(accuracy) = self.accuracy {
                    tr {
                        th { "Sensor Accuracy" }
                        td { (format!("{:.3E}", accuracy)) }
                    }
                }
                @if cfg!(feature = "nav") {
                    @if let Some((x_ecef_m, y_ecef_m, z_ecef_m)) = self.position {
                        tr {
                            th { "Sensor position (ECEF m)" }
                            td {
                                (format!("{:.3E}m", x_ecef_m))
                            }
                            td {
                                (format!("{:.3E}m", y_ecef_m))
                            }
                            td {
                                (format!("{:.3E}m", z_ecef_m))
                            }
                        }
                    }
                    @if let Some(h) = self.height {
                        tr {
                            th { "Height" }
                            td { (format!("{:.3} m", h)) }
                        }
                    }
                }
            }
        }
    }
}

impl std::str::FromStr for Sensor {
    type Err = ParsingError;
    fn from_str(content: &str) -> Result<Self, Self::Err> {
        let (model, rem) = content.split_at(20);
        let (s_type, rem) = rem.split_at(20 + 6);
        let (accuracy, rem) = rem.split_at(7 + 4);
        let (observable, _) = rem.split_at(2);
        Ok(Self {
            model: {
                if !model.trim().is_empty() {
                    Some(model.trim().to_string())
                } else {
                    None
                }
            },
            sensor_type: {
                if !s_type.trim().is_empty() {
                    Some(s_type.trim().to_string())
                } else {
                    None
                }
            },
            accuracy: {
                if let Ok(f) = f32::from_str(accuracy.trim()) {
                    Some(f)
                } else {
                    None
                }
            },
            height: None,
            position: None,
            observable: Observable::from_str(observable.trim())?,
        })
    }
}

impl std::fmt::Display for Sensor {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(model) = &self.model {
            write!(f, "{:<width$}", model, width = 20)?
        } else {
            write!(f, "{:20}", "")?;
        }

        if let Some(stype) = &self.sensor_type {
            write!(f, "{:<width$}", stype, width = 26)?;
        } else {
            write!(f, "{:26}", "")?;
        }

        if let Some(accuracy) = self.accuracy {
            write!(f, "{:^11.1}", accuracy)?
        } else {
            write!(f, "{:11}", "")?
        }
        writeln!(f, "{} SENSOR MOD/TYPE/ACC", self.observable)?;

        if let Some((x, y, z)) = self.position {
            write!(f, "{:14.4}", x)?;
            write!(f, "{:14.4}", y)?;
            write!(f, "{:14.4}", z)?;

            let h = self.height.unwrap_or(0.0);

            write!(f, "{:14.4}", h)?;
            writeln!(f, " {} SENSOR POS XYZ/H", self.observable)?
        }
        Ok(())
    }
}

impl Sensor {
    /// Define a new [Observable] sensor
    pub fn new(observable: Observable) -> Self {
        Self::default().with_observable(observable)
    }

    /// Copies and defines [Sensor] model
    pub fn with_model(&self, model: &str) -> Self {
        let mut s = self.clone();
        s.model = Some(model.to_string());
        s
    }

    /// Copies and defines [Sensor] type
    pub fn with_type(&self, stype: &str) -> Self {
        let mut s = self.clone();
        s.sensor_type = Some(stype.to_string());
        s
    }

    /// Copies and defines [Sensor] [Observable]
    pub fn with_observable(&self, observable: Observable) -> Self {
        let mut s = self.clone();
        s.observable = observable;
        s
    }

    /// Copies and define new sensor position
    pub fn with_position(&self, position: (f64, f64, f64)) -> Self {
        let mut s = self.clone();
        s.position = Some(position);
        s
    }

    /// Copies and define new sensor height eccentricity
    pub fn with_height(&self, h: f64) -> Self {
        let mut s = self.clone();
        s.height = Some(h);
        s
    }

    /// Copies and defines sensor accuracy
    pub fn with_accuracy(&self, accuracy: f32) -> Self {
        let mut s = self.clone();
        s.accuracy = Some(accuracy);
        s
    }

    #[cfg(feature = "nav")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nav")))]
    /// Expresses [Sensor] position as an [Orbit]
    pub fn rx_orbit(&self, t: Epoch, frame: Frame) -> Option<Orbit> {
        let (x_ecef_m, y_ecef_m, z_ecef_m) = self.position?;

        let pos_vel = Vector6::new(
            x_ecef_m / 1000.0,
            y_ecef_m / 1000.0,
            z_ecef_m / 1000.0,
            0.0,
            0.0,
            0.0,
        );

        Some(Orbit::from_cartesian_pos_vel(pos_vel, t, frame))
    }
}

#[cfg(test)]
// #[cfg(feature = "nav")]
mod test {
    use super::*;
    // use anise::prelude::{Orbit, Frame};
    use std::str::FromStr;

    #[test]
    fn test_formatting() {
        let s = Sensor::new(Observable::Temperature);
        assert_eq!(
            s.to_string(),
            "                                                         TD SENSOR MOD/TYPE/ACC\n"
        );
        let s = s.with_model("PAROSCIENTIFIC");
        assert_eq!(
            s.to_string(),
            "PAROSCIENTIFIC                                           TD SENSOR MOD/TYPE/ACC\n"
        );
        let s = s.with_observable(Observable::Pressure);
        let s = s.with_type("740-16B");
        assert_eq!(
            s.to_string(),
            "PAROSCIENTIFIC      740-16B                              PR SENSOR MOD/TYPE/ACC\n"
        );
        let s = s.with_accuracy(0.2);
        assert_eq!(
            s.to_string(),
            "PAROSCIENTIFIC      740-16B                       0.2    PR SENSOR MOD/TYPE/ACC\n"
        );

        // let rx_orbit = Orbit::from_position(
        //     0.0,
        //     0.0,
        //     0.0,
        //     Default::default(),
        //     Frame::from_name("EARTH", ""),
        // );

        // let mut s = s.with_position(rx_orbit);

        // s.height = Some(1234.5678);

        // assert_eq!(
        //     s.to_string(),
        //     "PAROSCIENTIFIC      740-16B                       0.2    PR SENSOR MOD/TYPE/ACC
        // 0.0000        0.0000        0.0000     1234.5678 PR SENSOR POS XYZ/H\n"
        // );
    }

    #[test]
    fn from_str() {
        let s = Sensor::from_str("                                                  0.0    PR ");
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s.model, None);
        assert_eq!(s.sensor_type, None);
        assert_eq!(s.accuracy, Some(0.0));
        assert_eq!(s.observable, Observable::Pressure);

        let s = Sensor::from_str(
            "PAROSCIENTIFIC      740-16B                       0.2    PR SENSOR MOD/TYPE/ACC",
        );
        assert!(s.is_ok());

        let s = Sensor::from_str(
            "                                                  0.0    PR SENSOR MOD/TYPE/ACC",
        );

        assert!(s.is_ok());
    }
}
