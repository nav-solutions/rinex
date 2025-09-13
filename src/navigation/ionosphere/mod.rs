use crate::prelude::ParsingError;

use std::str::FromStr;

mod bdgim;
mod klobuchar;
mod nequick_g;

#[cfg(feature = "ublox")]
#[cfg_attr(docsrs, doc(cfg(feature = "ublox")))]
pub mod ublox;

pub use bdgim::BdModel;
pub use klobuchar::{KbModel, KbRegionCode};
pub use nequick_g::{NgModel, NgRegionFlags};

/// [IonosphereModel] that may be described in modern NAV V4 RINEx
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum IonosphereModel {
    /// Klobuchar [KbModel] streamed by GPS
    Klobuchar(KbModel),

    /// Nequick-G [NgModel] streamed by GAL
    NequickG(NgModel),

    /// BDGIM [BdModel] streamed by BDS
    Bdgim(BdModel),
}

impl Default for IonosphereModel {
    fn default() -> Self {
        Self::Klobuchar(KbModel::default())
    }
}

impl IonosphereModel {
    /// Parses [IonosphereModel] from old (RINEX3) header Ionospheric terms.
    /// In this case, it will apply for the entire day course.
    /// Two models may exist: Klobuchar and NequickG.
    pub(crate) fn from_rinex3_header(header: &str) -> Result<Self, ParsingError> {
        let (system, model_params) = header.split_at(5);
        let rem = model_params.replace("D", "E");
        match system.trim() {
            /*
             * Models that only needs 3 fields
             */
            "GAL" => {
                let (a0, rem) = rem.split_at(12);
                let (a1, rem) = rem.split_at(12);
                let (a2, _) = rem.split_at(12);
                let a0 = f64::from_str(a0.trim()).map_err(|_| ParsingError::NequickGData)?;
                let a1 = f64::from_str(a1.trim()).map_err(|_| ParsingError::NequickGData)?;
                let a2 = f64::from_str(a2.trim()).map_err(|_| ParsingError::NequickGData)?;
                Ok(Self::NequickG(NgModel {
                    a: (a0, a1, a2),
                    // TODO: is this not the 4th field? double check
                    region: NgRegionFlags::default(),
                }))
            },
            system => {
                /*
                 * Model has 4 fields
                 */
                let (a0, rem) = rem.split_at(12);
                let (a1, rem) = rem.split_at(12);
                let (a2, rem) = rem.split_at(12);
                let (a3, _) = rem.split_at(12);
                // World or QZSS special orbital plan
                let region = match system.contains("QZS") {
                    true => KbRegionCode::Japan,
                    false => KbRegionCode::Worldwide,
                };
                /* determine which field we're dealing with */
                if system.ends_with('A') {
                    let a0 = f64::from_str(a0.trim()).map_err(|_| ParsingError::KlobucharData)?;
                    let a1 = f64::from_str(a1.trim()).map_err(|_| ParsingError::KlobucharData)?;
                    let a2 = f64::from_str(a2.trim()).map_err(|_| ParsingError::KlobucharData)?;
                    let a3 = f64::from_str(a3.trim()).map_err(|_| ParsingError::KlobucharData)?;

                    Ok(Self::Klobuchar(KbModel {
                        alpha: (a0, a1, a2, a3),
                        beta: (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64),
                        region,
                    }))
                } else {
                    let b0 = f64::from_str(a0.trim()).map_err(|_| ParsingError::KlobucharData)?;
                    let b1 = f64::from_str(a1.trim()).map_err(|_| ParsingError::KlobucharData)?;
                    let b2 = f64::from_str(a2.trim()).map_err(|_| ParsingError::KlobucharData)?;
                    let b3 = f64::from_str(a3.trim()).map_err(|_| ParsingError::KlobucharData)?;
                    Ok(Self::Klobuchar(KbModel {
                        alpha: (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64),
                        beta: (b0, b1, b2, b3),
                        region,
                    }))
                }
            },
        }
    }

    /// Parses [IonosphereModel] from old (RINEX3) header Ionospheric terms.
    /// In this case, it will apply for the entire day course.
    /// Only the Klobuchar model may exist.
    pub(crate) fn from_rinex2_header(header: &str, marker: &str) -> Result<Self, ParsingError> {
        let header = header.replace("d", "e").replace("D", "E");
        let (_, rem) = header.split_at(2);
        let (a0, rem) = rem.split_at(12);
        let (a1, rem) = rem.split_at(12);
        let (a2, rem) = rem.split_at(12);
        let (a3, _) = rem.split_at(12);

        if marker.contains("ALPHA") {
            let a0 = f64::from_str(a0.trim()).map_err(|_| ParsingError::KlobucharData)?;
            let a1 = f64::from_str(a1.trim()).map_err(|_| ParsingError::KlobucharData)?;
            let a2 = f64::from_str(a2.trim()).map_err(|_| ParsingError::KlobucharData)?;
            let a3 = f64::from_str(a3.trim()).map_err(|_| ParsingError::KlobucharData)?;

            Ok(Self::Klobuchar(KbModel {
                alpha: (a0, a1, a2, a3),
                beta: (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64),
                region: KbRegionCode::Worldwide,
            }))
        } else {
            // Assume marker.contains("BETA")
            let b0 = f64::from_str(a0.trim()).map_err(|_| ParsingError::KlobucharData)?;
            let b1 = f64::from_str(a1.trim()).map_err(|_| ParsingError::KlobucharData)?;
            let b2 = f64::from_str(a2.trim()).map_err(|_| ParsingError::KlobucharData)?;
            let b3 = f64::from_str(a3.trim()).map_err(|_| ParsingError::KlobucharData)?;

            Ok(Self::Klobuchar(KbModel {
                alpha: (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64),
                beta: (b0, b1, b2, b3),
                region: KbRegionCode::Worldwide,
            }))
        }
    }

    // /* converts self to meters of delay */
    // pub(crate) fn meters_delay(
    //     &self,
    //     t: Epoch,
    //     e: f64,
    //     a: f64,
    //     h_km: f64,
    //     user_lat_ddeg: f64,
    //     user_lon_ddeg: f64,
    //     carrier: Carrier,
    // ) -> Option<f64> {
    //     if let Some(kb) = self.as_klobuchar() {
    //         Some(kb.meters_delay(t, e, a, h_km, user_lat_ddeg, user_lon_ddeg, carrier))
    //     } else if let Some(ng) = self.as_nequick_g() {
    //         None
    //     } else if let Some(bd) = self.as_bdgim() {
    //         None
    //     } else {
    //         None
    //     }
    // }

    /// Returns reference to Klobuchar [KbModel]
    pub fn as_klobuchar(&self) -> Option<&KbModel> {
        match self {
            Self::Klobuchar(model) => Some(model),
            _ => None,
        }
    }

    /// Returns mutable reference to Klobuchar [KbModel]
    pub fn as_klobuchar_mut(&mut self) -> Option<&mut KbModel> {
        match self {
            Self::Klobuchar(model) => Some(model),
            _ => None,
        }
    }

    /// Returns reference to Nequick-G [NgModel]
    pub fn as_nequick_g(&self) -> Option<&NgModel> {
        match self {
            Self::NequickG(model) => Some(model),
            _ => None,
        }
    }

    /// Returns reference to BDGIM [BdModel]
    pub fn as_bdgim(&self) -> Option<&BdModel> {
        match self {
            Self::Bdgim(model) => Some(model),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn rinex3_ng_header_parsing() {
        // v3 Nequick-G header
        let ng = IonosphereModel::from_rinex3_header(
            "GAL    6.6250e+01 -1.6406e-01 -2.4719e-03  0.0000e+00       ",
        );
        assert!(ng.is_ok(), "failed to parse GAL iono correction header");

        let ng = ng.unwrap();
        assert_eq!(
            ng,
            IonosphereModel::NequickG(NgModel {
                a: (6.6250e+01, -1.6406e-01, -2.4719e-03),
                region: NgRegionFlags::empty(),
            })
        );
    }

    #[test]
    fn rinex3_kb_header_parsing() {
        // v3 Kb header
        let kb = IonosphereModel::from_rinex3_header(
            "GPSA   7.4506e-09 -1.4901e-08 -5.9605e-08  1.1921e-07       ",
        );
        let kb = kb.unwrap();

        assert_eq!(
            kb,
            IonosphereModel::Klobuchar(KbModel {
                alpha: (7.4506E-9, -1.4901E-8, -5.9605E-8, 1.1921E-7),
                beta: (0.0, 0.0, 0.0, 0.0),
                region: KbRegionCode::Worldwide,
            })
        );

        let kb = IonosphereModel::from_rinex3_header(
            "GPSB   9.0112e+04 -6.5536e+04 -1.3107e+05  4.5875e+05       ",
        );
        assert!(kb.is_ok(), "failed to parse GPSB iono correction header");
        let kb = kb.unwrap();
        assert_eq!(
            kb,
            IonosphereModel::Klobuchar(KbModel {
                alpha: (0.0, 0.0, 0.0, 0.0),
                beta: (9.0112E4, -6.5536E4, -1.3107E5, 4.5875E5),
                region: KbRegionCode::Worldwide,
            })
        );

        // v3 Kb header w/ D notation
        let kb = IonosphereModel::from_rinex3_header(
            "GPSA   0.2794D-07  0.1490D-07 -0.1192D-06 -0.5960D-07       ",
        );
        let kb = kb.unwrap();

        assert_eq!(
            kb,
            IonosphereModel::Klobuchar(KbModel {
                alpha: (0.2794E-07, 0.1490E-07, -0.1192E-06, -0.5960E-07),
                beta: (0.0, 0.0, 0.0, 0.0),
                region: KbRegionCode::Worldwide,
            })
        );

        let kb = IonosphereModel::from_rinex3_header(
            "GPSB   0.1290D+06  0.3277D+05 -0.2621D+06  0.2621D+06       ",
        );
        assert!(kb.is_ok(), "failed to parse GPSB iono correction header");
        let kb = kb.unwrap();
        assert_eq!(
            kb,
            IonosphereModel::Klobuchar(KbModel {
                alpha: (0.0, 0.0, 0.0, 0.0),
                beta: (0.1290E+06, 0.3277E+05, -0.2621E+06, 0.2621E+06),
                region: KbRegionCode::Worldwide,
            })
        );

        let kb = IonosphereModel::from_rinex3_header(
            "BDSA   1.1176e-08  2.9802e-08 -4.1723e-07  6.5565e-07       ",
        );
        assert!(kb.is_ok(), "failed to parse BDSA iono correction header");
        let kb = kb.unwrap();
        assert_eq!(
            kb,
            IonosphereModel::Klobuchar(KbModel {
                alpha: (1.1176E-8, 2.9802E-8, -4.1723E-7, 6.5565E-7),
                beta: (0.0, 0.0, 0.0, 0.0),
                region: KbRegionCode::Worldwide,
            })
        );

        let kb = IonosphereModel::from_rinex3_header(
            "BDSB   1.4131e+05 -5.2429e+05  1.6384e+06 -4.5875e+05   3   ",
        );
        assert!(kb.is_ok(), "failed to parse BDSB iono correction header");
        let kb = kb.unwrap();
        assert_eq!(
            kb,
            IonosphereModel::Klobuchar(KbModel {
                alpha: (0.0, 0.0, 0.0, 0.0),
                beta: (1.4131E5, -5.2429E5, 1.6384E6, -4.5875E5),
                region: KbRegionCode::Worldwide,
            })
        );

        /*
         * Test japanese (QZSS) orbital plan
         */
        let kb = IonosphereModel::from_rinex3_header(
            "QZSA   7.4506e-09 -1.4901e-08 -5.9605e-08  1.1921e-07       ",
        );
        assert!(kb.is_ok(), "failed to parse QZSA iono correction header");
        let kb = kb.unwrap();
        let kb = kb.as_klobuchar().unwrap();
        assert_eq!(
            kb.region,
            KbRegionCode::Japan,
            "QZSA ionospheric corr badly interprated as worldwide correction"
        );

        let kb = IonosphereModel::from_rinex3_header(
            "QZSB   9.0112e+04 -6.5536e+04 -1.3107e+05  4.5875e+05       ",
        );
        assert!(kb.is_ok(), "failed to parse QZSB iono correction header");
        let kb = kb.unwrap();
        let kb = kb.as_klobuchar().unwrap();
        assert_eq!(
            kb.region,
            KbRegionCode::Japan,
            "QZSB ionospheric corr badly interprated as worldwide correction"
        );
    }

    #[test]
    fn rinex2_kb_header_parsing() {
        for (line, alpha) in [
            // RINEX v2.10 and v2.11 standards (same string in both)
            // https://files.igs.org/pub/data/format/rinex211.txt
            (
                "     .1676D-07   .2235D-07  -.1192D-06  -.1192D-06          ",
                (0.1676E-07, 0.2235E-07, -0.1192E-06, -0.1192E-06),
            ),
            // ftp://igs.ign.fr/pub/igs/data/2024/119/zimm1190.24n.gz
            (
                "     .2515D-07   .1490D-07  -.1192D-06  -.5960D-07          ",
                (0.2515E-07, 0.1490E-07, -0.1192E-06, -0.5960E-07),
            ),
            // https://github.com/osqzss/gps-sdr-sim/blob/654a9888c54218766909e15fe8139e4bc8e83ecc/brdc0010.22n
            (
                "    0.1211D-07 -0.7451D-08 -0.5960D-07  0.1192D-06          ",
                (0.1211E-07, -0.7451E-08, -0.5960E-07, 0.1192E-06),
            ),
        ] {
            let kb = IonosphereModel::from_rinex2_header(line, "ION ALPHA           ");
            assert!(kb.is_ok(), "failed to parse ION ALPHA header");
            let kb = kb.unwrap();
            assert_eq!(
                kb,
                IonosphereModel::Klobuchar(KbModel {
                    alpha,
                    beta: (0.0, 0.0, 0.0, 0.0),
                    region: KbRegionCode::Worldwide,
                })
            );
        }

        for (line, beta) in [
            // RINEX v2.10 and v2.11 standards (same string in both)
            // https://files.igs.org/pub/data/format/rinex211.txt
            (
                "     .1208D+06   .1310D+06  -.1310D+06  -.1966D+06          ",
                (0.1208E+06, 0.1310E+06, -0.1310E+06, -0.1966E+06),
            ),
            // ftp://igs.ign.fr/pub/igs/data/2024/119/zimm1190.24n.gz
            (
                "     .1290D+06   .4915D+05  -.2621D+06   .1966D+06          ",
                (0.1290E+06, 0.4915E+05, -0.2621E+06, 0.1966E+06),
            ),
            // https://github.com/osqzss/gps-sdr-sim/blob/654a9888c54218766909e15fe8139e4bc8e83ecc/brdc0010.22n
            (
                "    0.1167D+06 -0.2458D+06 -0.6554D+05  0.1114D+07          ",
                (0.1167E+06, -0.2458E+06, -0.6554E+05, 0.1114E+07),
            ),
        ] {
            let kb = IonosphereModel::from_rinex2_header(line, "ION BETA            ");
            assert!(kb.is_ok(), "failed to parse ION BETA header");
            let kb = kb.unwrap();
            assert_eq!(
                kb,
                IonosphereModel::Klobuchar(KbModel {
                    alpha: (0.0, 0.0, 0.0, 0.0),
                    beta,
                    region: KbRegionCode::Worldwide,
                })
            );
        }
    }
}
