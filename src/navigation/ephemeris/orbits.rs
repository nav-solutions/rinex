//! NAV Orbits description, spanning all revisions and constellations
use std::str::FromStr;

use crate::{
    navigation::ephemeris::flags::{
        bds::{
            BdsB1cIntegrity, BdsB2aB1cIntegrity, BdsB2bIntegrity, BdsHealth, BdsSatH1,
            BdsSatelliteType,
        },
        gal::{GalDataSource, GalHealth},
        geo::GeoHealth,
        glonass::{GlonassHealth, GlonassHealth2, GlonassStatus},
        gps::{GpsQzssl1cHealth, GpsQzssl1l2l5Health},
        irnss::IrnssHealth,
    },
    prelude::ParsingError,
};

#[cfg(feature = "serde")]
use serde::Serialize;

include!(concat!(env!("OUT_DIR"), "/nav_orbits.rs"));

/// [OrbitItem] item is Navigation ephemeris entry.
/// It is a complex data wrapper, for high level
/// record description, across all revisions and constellations
#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum OrbitItem {
    /// Interpreted as unsigned byte
    U8(u8),

    /// Interpreted as signed byte
    I8(i8),

    /// Interpreted as unsigned 32 bit value
    U32(u32),

    /// Interpreted as double precision data
    F64(f64),

    /// L2P flag from GPS message is asserted when
    /// the quadrature L2 P(Y) does stream data
    /// otherwise it has been commanded off, most likely under A/S.
    Gpsl2pFlag(bool),

    /// GPS / QZSS [GpsQzssl1cHealth] flag
    GpsQzssl1cHealth(GpsQzssl1cHealth),

    /// GPS / QZSS [GpsQzssl1l2l5Health] flag
    GpsQzssl1l2l5Health(GpsQzssl1l2l5Health),

    /// [GeoHealth] SV indication
    GeoHealth(GeoHealth),

    /// [GalHealth] SV indication
    GalHealth(GalHealth),

    /// [GalDataSource] signal indication
    GalDataSource(GalDataSource),

    /// [IrnssHealth] SV indication
    IrnssHealth(IrnssHealth),

    /// [GlonassHealth] SV indication
    GlonassHealth(GlonassHealth),

    /// [GlonassHealth2] SV indication present in modern frames
    GlonassHealth2(GlonassHealth2),

    /// [GlonassStatus] NAV4 Orbit7 status flag
    GlonassStatus(GlonassStatus),

    /// [BdsSatH1] health flag in historical and D1/D2 frames
    BdsSatH1(BdsSatH1),

    /// [BdsHealth] flag in modern frames
    BdsHealth(BdsHealth),

    /// [BdsSatelliteType] indication
    BdsSatelliteType(BdsSatelliteType),

    /// [BdsB1cIntegrity] flag
    BdsB1cIntegrity(BdsB1cIntegrity),

    /// [BdsB2aB1cIntegrity] flag
    BdsB2aB1cIntegrity(BdsB2aB1cIntegrity),

    /// [BdsB2bIntegrity] flag
    BdsB2bIntegrity(BdsB2bIntegrity),
}

impl std::fmt::Display for OrbitItem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::U8(val) => write!(f, "{:02x}", val),
            Self::I8(val) => write!(f, "{:02x}", val),
            Self::U32(val) => write!(f, "{:08X}", val),
            Self::F64(val) => write!(f, "{}", val),
            Self::Gpsl2pFlag(val) => write!(f, "l2p={:?}", val),
            Self::GeoHealth(val) => write!(f, "{:?}", val),
            Self::GalHealth(val) => write!(f, "{:?}", val),
            Self::GalDataSource(val) => write!(f, "{:?}", val),
            Self::IrnssHealth(val) => write!(f, "{:?}", val),
            Self::GlonassHealth(val) => write!(f, "{:?}", val),
            Self::GlonassStatus(val) => write!(f, "{:?}", val),
            Self::BdsSatH1(val) => write!(f, "{:?}", val),
            Self::BdsHealth(val) => write!(f, "{:?}", val),
            Self::BdsSatelliteType(val) => write!(f, "{:?}", val),
            Self::GlonassHealth2(val) => write!(f, "{:?}", val),
            Self::GpsQzssl1cHealth(val) => write!(f, "{:?}", val),
            Self::GpsQzssl1l2l5Health(val) => write!(f, "{:?}", val),
            Self::BdsB1cIntegrity(val) => write!(f, "{:?}", val),
            Self::BdsB2aB1cIntegrity(val) => write!(f, "{:?}", val),
            Self::BdsB2bIntegrity(val) => write!(f, "{:?}", val),
        }
    }
}

impl From<u32> for OrbitItem {
    fn from(value: u32) -> Self {
        Self::U32(value)
    }
}

impl From<f64> for OrbitItem {
    fn from(value: f64) -> Self {
        Self::F64(value)
    }
}

impl OrbitItem {
    /// Builds a [OrbitItem] from type descriptor and string content
    pub fn new(
        name_str: &str,
        type_str: &str,
        val_str: &str,
        msgtype: &NavMessageType,
        constellation: Constellation,
    ) -> Result<OrbitItem, ParsingError> {
        // make it "rust" compatible
        let float =
            f64::from_str(&val_str.replace('D', "e")).map_err(|_| ParsingError::NavNullOrbit)?;

        // do not tolerate zero values for native types
        match type_str {
            "u8" | "i8" | "u32" | "f64" => {
                if float == 0.0 {
                    return Err(ParsingError::NavNullOrbit);
                }
            },
            _ => {}, // non-native types
        }

        // uninterpreted data remains as native type and we exit.
        match type_str {
            "u8" => {
                let unsigned = float.round() as u8;
                return Ok(OrbitItem::U8(unsigned));
            },

            "i8" => {
                let signed = float.round() as i8;
                return Ok(OrbitItem::I8(signed));
            },

            "u32" => {
                let unsigned = float.round() as u32;
                return Ok(OrbitItem::U32(unsigned));
            },

            "f64" => {
                return Ok(OrbitItem::F64(float));
            },
            _ => {},
        };

        // handle complex types
        match type_str {
            "flag" => {
                // bit flags interpretation
                let unsigned = float.round() as u32;

                match name_str {
                    "health" => {
                        // complex health flag interpretation

                        // handle GEO case
                        if constellation.is_sbas() {
                            match msgtype {
                                NavMessageType::LNAV | NavMessageType::SBAS => {
                                    let flags = GeoHealth::from_bits(unsigned)
                                        .ok_or(ParsingError::NavFlagsMapping)?;

                                    return Ok(OrbitItem::GeoHealth(flags));
                                },
                                _ => return Err(ParsingError::NavHealthFlagDefinition),
                            }
                        }

                        // other cases
                        match (msgtype, constellation) {
                            (
                                NavMessageType::LNAV | NavMessageType::CNAV,
                                Constellation::GPS | Constellation::QZSS,
                            ) => {
                                let flags = GpsQzssl1l2l5Health::from(unsigned);

                                Ok(OrbitItem::GpsQzssl1l2l5Health(flags))
                            },
                            (NavMessageType::CNV2, Constellation::GPS | Constellation::QZSS) => {
                                let flags = GpsQzssl1cHealth::from_bits(unsigned)
                                    .ok_or(ParsingError::NavFlagsMapping)?;

                                Ok(OrbitItem::GpsQzssl1cHealth(flags))
                            },
                            (
                                NavMessageType::LNAV | NavMessageType::INAV | NavMessageType::FNAV,
                                Constellation::Galileo,
                            ) => {
                                let flags = GalHealth::from_bits(unsigned)
                                    .ok_or(ParsingError::NavFlagsMapping)?;

                                Ok(OrbitItem::GalHealth(flags))
                            },
                            (
                                NavMessageType::LNAV | NavMessageType::FDMA,
                                Constellation::Glonass,
                            ) => {
                                let flags = GlonassHealth::from_bits(unsigned)
                                    .ok_or(ParsingError::NavFlagsMapping)?;

                                Ok(OrbitItem::GlonassHealth(flags))
                            },
                            (
                                NavMessageType::LNAV | NavMessageType::D1 | NavMessageType::D2,
                                Constellation::BeiDou,
                            ) => {
                                // BDS H1 flag
                                let flags = BdsSatH1::from_bits(unsigned)
                                    .ok_or(ParsingError::NavFlagsMapping)?;

                                Ok(OrbitItem::BdsSatH1(flags))
                            },
                            (
                                NavMessageType::CNV1 | NavMessageType::CNV2 | NavMessageType::CNV3,
                                Constellation::BeiDou,
                            ) => {
                                // Modern BDS flag
                                let flags = BdsHealth::from(unsigned);

                                Ok(OrbitItem::BdsHealth(flags))
                            },
                            _ => Err(ParsingError::NavHealthFlagDefinition),
                        }
                    },
                    "health2" => {
                        // Subsidary health flags
                        match (msgtype, constellation) {
                            (NavMessageType::FDMA, Constellation::Glonass) => {
                                let flags = GlonassHealth2::from_bits(unsigned)
                                    .ok_or(ParsingError::NavFlagsMapping)?;

                                Ok(OrbitItem::GlonassHealth2(flags))
                            },
                            _ => Err(ParsingError::NavHealthFlagDefinition),
                        }
                    },
                    "source" => {
                        // complex signal source indication
                        match (msgtype, constellation) {
                            (
                                NavMessageType::LNAV | NavMessageType::INAV | NavMessageType::FNAV,
                                Constellation::Galileo,
                            ) => {
                                let flags = GalDataSource::from_bits(unsigned)
                                    .ok_or(ParsingError::NavFlagsMapping)?;

                                Ok(OrbitItem::GalDataSource(flags))
                            },
                            _ => Err(ParsingError::NavDataSourceDefinition),
                        }
                    },
                    "satType" => match (msgtype, constellation) {
                        (NavMessageType::CNV1 | NavMessageType::CNV2, Constellation::BeiDou) => {
                            let flags = BdsSatelliteType::from(unsigned);

                            Ok(OrbitItem::BdsSatelliteType(flags))
                        },
                        _ => Err(ParsingError::NavDataSourceDefinition),
                    },
                    "integrity" => match (msgtype, constellation) {
                        (NavMessageType::CNV1, Constellation::BeiDou) => {
                            let flags = BdsB1cIntegrity::from_bits(unsigned)
                                .ok_or(ParsingError::NavFlagsMapping)?;

                            Ok(OrbitItem::BdsB1cIntegrity(flags))
                        },
                        (NavMessageType::CNV2, Constellation::BeiDou) => {
                            let flags = BdsB2aB1cIntegrity::from_bits(unsigned)
                                .ok_or(ParsingError::NavFlagsMapping)?;

                            Ok(OrbitItem::BdsB2aB1cIntegrity(flags))
                        },
                        (NavMessageType::CNV3, Constellation::BeiDou) => {
                            let flags = BdsB2bIntegrity::from_bits(unsigned)
                                .ok_or(ParsingError::NavFlagsMapping)?;

                            Ok(OrbitItem::BdsB2bIntegrity(flags))
                        },
                        _ => Err(ParsingError::NavDataSourceDefinition),
                    },
                    "status" => {
                        // complex status indication
                        match (msgtype, constellation) {
                            (NavMessageType::FDMA, Constellation::Glonass) => {
                                let flags = GlonassStatus::from(unsigned);

                                Ok(OrbitItem::GlonassStatus(flags))
                            },
                            _ => Err(ParsingError::NavHealthFlagDefinition),
                        }
                    },
                    "l2p" => {
                        // l2p flag from GPS navigation message
                        Ok(OrbitItem::Gpsl2pFlag(unsigned > 0))
                    },
                    _ => Err(ParsingError::NavFlagsDefinition),
                }
            },
            _ => {
                // unknown complex type
                Err(ParsingError::NavUnknownComplexType)
            },
        }
    }

    /// Unwraps [OrbitItem] as [f64] (always feasible)
    pub fn as_f64(&self) -> f64 {
        match self {
            OrbitItem::F64(f) => *f,
            OrbitItem::U8(val) => *val as f64,
            OrbitItem::I8(val) => *val as f64,
            OrbitItem::U32(val) => *val as f64,
            OrbitItem::Gpsl2pFlag(flag) => (*flag as u8) as f64,
            OrbitItem::GalHealth(flags) => flags.bits() as f64,
            OrbitItem::GpsQzssl1cHealth(flags) => flags.bits() as f64,
            OrbitItem::GpsQzssl1l2l5Health(flags) => flags.0 as f64,
            OrbitItem::GeoHealth(flags) => flags.bits() as f64,
            OrbitItem::IrnssHealth(flags) => flags.bits() as f64,
            OrbitItem::GlonassHealth(flags) => flags.bits() as f64,
            OrbitItem::GlonassStatus(status) => status.0 as f64,
            OrbitItem::GalDataSource(source) => source.bits() as f64,
            OrbitItem::GlonassHealth2(health) => health.bits() as f64,
            OrbitItem::BdsSatH1(sat_h1) => sat_h1.bits() as f64,
            OrbitItem::BdsHealth(health) => (*health as u32) as f64,
            OrbitItem::BdsSatelliteType(sat) => (*sat as u32) as f64,
            OrbitItem::BdsB1cIntegrity(integrity) => integrity.bits() as f64,
            OrbitItem::BdsB2aB1cIntegrity(integrity) => integrity.bits() as f64,
            OrbitItem::BdsB2bIntegrity(integrity) => integrity.bits() as f64,
        }
    }

    /// Unwraps [OrbitItem] as [u32] (always feasible)
    pub fn as_u32(&self) -> u32 {
        match self {
            OrbitItem::U32(v) => *v,
            OrbitItem::U8(val) => *val as u32,
            OrbitItem::I8(val) => *val as u32,
            OrbitItem::F64(val) => val.round() as u32,
            OrbitItem::Gpsl2pFlag(flag) => *flag as u32,
            OrbitItem::GalHealth(health) => health.bits(),
            OrbitItem::GpsQzssl1cHealth(flags) => flags.bits(),
            OrbitItem::GpsQzssl1l2l5Health(flags) => flags.0,
            OrbitItem::GeoHealth(health) => health.bits(),
            OrbitItem::IrnssHealth(health) => health.bits(),
            OrbitItem::GlonassHealth(health) => health.bits(),
            OrbitItem::GlonassStatus(status) => status.0,
            OrbitItem::GalDataSource(source) => source.bits(),
            OrbitItem::GlonassHealth2(health) => health.bits(),
            OrbitItem::BdsSatH1(sat_h1) => sat_h1.bits(),
            OrbitItem::BdsHealth(health) => *health as u32,
            OrbitItem::BdsSatelliteType(sat) => *sat as u32,
            OrbitItem::BdsB1cIntegrity(integrity) => integrity.bits(),
            OrbitItem::BdsB2aB1cIntegrity(integrity) => integrity.bits(),
            OrbitItem::BdsB2bIntegrity(integrity) => integrity.bits(),
        }
    }

    /// Unwraps [OrbitItem] as [u8] (always feasible)
    pub fn as_u8(&self) -> u8 {
        match self {
            OrbitItem::U32(v) => *v as u8,
            OrbitItem::U8(val) => *val,
            OrbitItem::I8(val) => *val as u8,
            OrbitItem::F64(val) => val.round() as u8,
            OrbitItem::Gpsl2pFlag(flag) => *flag as u8,
            OrbitItem::GalHealth(health) => health.bits() as u8,
            OrbitItem::GpsQzssl1cHealth(flags) => flags.bits() as u8,
            OrbitItem::GpsQzssl1l2l5Health(flags) => flags.0 as u8,
            OrbitItem::GeoHealth(health) => health.bits() as u8,
            OrbitItem::IrnssHealth(health) => health.bits() as u8,
            OrbitItem::GlonassHealth(health) => health.bits() as u8,
            OrbitItem::GlonassStatus(status) => status.0 as u8,
            OrbitItem::GlonassHealth2(health) => health.bits() as u8,
            OrbitItem::GalDataSource(source) => source.bits() as u8,
            OrbitItem::BdsSatH1(sat_h1) => sat_h1.bits() as u8,
            OrbitItem::BdsHealth(health) => (*health as u32) as u8,
            OrbitItem::BdsSatelliteType(sat) => (*sat as u32) as u8,
            OrbitItem::BdsB1cIntegrity(integrity) => integrity.bits() as u8,
            OrbitItem::BdsB2aB1cIntegrity(integrity) => integrity.bits() as u8,
            OrbitItem::BdsB2bIntegrity(integrity) => integrity.bits() as u8,
        }
    }

    /// Unwraps [OrbitItem] as [i8] (always feasible)
    pub fn as_i8(&self) -> i8 {
        match self {
            OrbitItem::U32(v) => *v as i8,
            OrbitItem::I8(val) => *val,
            OrbitItem::U8(val) => *val as i8,
            OrbitItem::F64(val) => val.round() as i8,
            OrbitItem::Gpsl2pFlag(flag) => *flag as i8,
            OrbitItem::GalHealth(health) => health.bits() as i8,
            OrbitItem::GpsQzssl1cHealth(flags) => flags.bits() as i8,
            OrbitItem::GpsQzssl1l2l5Health(flags) => flags.0 as i8,
            OrbitItem::GeoHealth(health) => health.bits() as i8,
            OrbitItem::IrnssHealth(health) => health.bits() as i8,
            OrbitItem::GlonassHealth(health) => health.bits() as i8,
            OrbitItem::GlonassStatus(status) => status.0 as i8,
            OrbitItem::GalDataSource(source) => source.bits() as i8,
            OrbitItem::GlonassHealth2(health) => health.bits() as i8,
            OrbitItem::BdsSatH1(sat_h1) => sat_h1.bits() as i8,
            OrbitItem::BdsHealth(health) => (*health as u32) as i8,
            OrbitItem::BdsSatelliteType(sat) => (*sat as u32) as i8,
            OrbitItem::BdsB1cIntegrity(integrity) => integrity.bits() as i8,
            OrbitItem::BdsB2aB1cIntegrity(integrity) => integrity.bits() as i8,
            OrbitItem::BdsB2bIntegrity(integrity) => integrity.bits() as i8,
        }
    }

    /// Unwraps Self as [Gpsl2pFlag] (if feasible)
    pub fn as_gps_l2p_flag(&self) -> Option<bool> {
        match self {
            OrbitItem::Gpsl2pFlag(flag) => Some(*flag),
            _ => None,
        }
    }

    /// Unwraps Self as [GpsQzssl1l2l5Health] flag (if feasible).
    pub fn as_gps_qzss_l1l2l5_health_flag(&self) -> Option<GpsQzssl1l2l5Health> {
        match self {
            OrbitItem::GpsQzssl1l2l5Health(h) => Some(h.clone()),
            _ => None,
        }
    }

    /// Unwraps Self as [GpsQzssl1cHealth] flag (if feasible).
    pub fn as_gps_qzss_l1c_health_flag(&self) -> Option<GpsQzssl1cHealth> {
        match self {
            OrbitItem::GpsQzssl1cHealth(h) => Some(h.clone()),
            _ => None,
        }
    }

    /// Unwraps Self as [GeoHealth] flag (if feasible)
    pub fn as_geo_health_flag(&self) -> Option<GeoHealth> {
        match self {
            OrbitItem::GeoHealth(h) => Some(h.clone()),
            _ => None,
        }
    }

    /// Unwraps Self as [GlonassHealth] flag (if feasible)
    pub fn as_glonass_health_flag(&self) -> Option<GlonassHealth> {
        match self {
            OrbitItem::GlonassHealth(h) => Some(h.clone()),
            _ => None,
        }
    }

    /// Unwraps Self as [GlonassHealth2] flag (if feasible)
    pub fn as_glonass_health2_flag(&self) -> Option<GlonassHealth2> {
        match self {
            OrbitItem::GlonassHealth2(h) => Some(h.clone()),
            _ => None,
        }
    }

    /// Unwraps Self as [GlonassStatus] mask (if feasible)
    pub fn as_glonass_status_mask(&self) -> Option<GlonassStatus> {
        match self {
            OrbitItem::GlonassStatus(h) => Some(h.clone()),
            _ => None,
        }
    }

    /// Unwraps Self as [GalHealth] flag (if feasible)
    pub fn as_galileo_health_flag(&self) -> Option<GalHealth> {
        match self {
            OrbitItem::GalHealth(h) => Some(*h),
            _ => None,
        }
    }

    /// Unwraps Self as historical (and D1/D2) [BdsSatH1] flag (if feasible)
    pub fn as_bds_sat_h1_flag(&self) -> Option<BdsSatH1> {
        match self {
            OrbitItem::BdsSatH1(h) => Some(h.clone()),
            _ => None,
        }
    }

    /// Unwraps Self as modern [BdsHealth] flag (if feasible)
    pub fn as_bds_health_flag(&self) -> Option<BdsHealth> {
        match self {
            OrbitItem::BdsHealth(h) => Some(h.clone()),
            _ => None,
        }
    }

    /// Unwraps Self as modern [BdsSatelliteType] indication (if feasible)
    pub fn as_bds_satellite_type(&self) -> Option<BdsSatelliteType> {
        match self {
            OrbitItem::BdsSatelliteType(h) => Some(h.clone()),
            _ => None,
        }
    }

    /// Unwraps Self as [BdsB1cIntegrity] flag (if feasible)
    pub fn as_bds_b1c_integrity(&self) -> Option<BdsB1cIntegrity> {
        match self {
            OrbitItem::BdsB1cIntegrity(h) => Some(h.clone()),
            _ => None,
        }
    }

    /// Unwraps Self as [BdsB2aB1cIntegrity] flag (if feasible)
    pub fn as_bds_b2a_b1c_integrity(&self) -> Option<BdsB2aB1cIntegrity> {
        match self {
            OrbitItem::BdsB2aB1cIntegrity(h) => Some(h.clone()),
            _ => None,
        }
    }

    /// Unwraps Self as [BdsB2bIntegrity] flag (if feasible)
    pub fn as_bds_b2b_integrity(&self) -> Option<BdsB2bIntegrity> {
        match self {
            OrbitItem::BdsB2bIntegrity(h) => Some(h.clone()),
            _ => None,
        }
    }

    /// Unwraps Self as [IrnssHealth] flag (if feasible)
    pub fn as_irnss_health_flag(&self) -> Option<IrnssHealth> {
        match self {
            OrbitItem::IrnssHealth(h) => Some(h.clone()),
            _ => None,
        }
    }
}

/// Identifies closest (but older) revision contained in NAV database.
/// Closest content (in time) is used during record parsing to identify and sort data.
/// Returns None
/// - if no database entries were found for requested constellation.
///  - or only newer revision exist : we prefer matching on older revisions
pub(crate) fn closest_nav_standards(
    constellation: Constellation,
    revision: Version,
    msg: NavMessageType,
) -> Option<&'static NavHelper<'static>> {
    let database = &NAV_ORBITS;
    // start by trying to locate desired revision.
    // On each mismatch, we decrement and move on to next major/minor combination.
    let (mut major, mut minor): (u8, u8) = revision.into();

    loop {
        // Match constellation, message type & revision exactly
        let items: Vec<_> = database
            .iter()
            .filter(|item| {
                item.constellation == constellation
                    && item.msg == msg
                    && item.version == Version::new(major, minor)
            })
            .collect();

        if items.is_empty() {
            if minor == 0 {
                // we're done with this major
                // -> downgrade to previous major
                //    we start @ minor = 10, which is
                //    larger than most revisions we know
                if major == 0 {
                    // we're done
                    break;
                } else {
                    major -= 1;
                    minor = 10;
                }
            } else {
                minor -= 1;
            }
        } else {
            return Some(items[0]);
        }
    } // loop

    None
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::navigation::NavMessageType;

    #[test]
    fn orbit_database_sanity() {
        for frame in NAV_ORBITS.iter() {
            let nav_msg = frame.msg;
            let constellation = frame.constellation;

            for (name_str, type_str) in frame.items.iter() {
                let val_str = "1.2345";

                let e = OrbitItem::new(name_str, type_str, val_str, &nav_msg, constellation);
                assert!(
                    e.is_ok(),
                    "{}({}) {}:{} orbit item failed",
                    constellation,
                    nav_msg,
                    name_str,
                    type_str,
                );
            }
        }
    }

    #[test]
    fn nav_standards_finder() {
        // Constellation::Mixed is not contained in db!
        assert_eq!(
            closest_nav_standards(
                Constellation::Mixed,
                Version::default(),
                NavMessageType::LNAV
            ),
            None,
            "Mixed GNSS constellation is or should not exist in the DB"
        );

        // Test existing (exact match) entries
        for (constellation, rev, msg) in [
            (Constellation::GPS, Version::new(1, 0), NavMessageType::LNAV),
            (Constellation::GPS, Version::new(2, 0), NavMessageType::LNAV),
            (Constellation::GPS, Version::new(4, 0), NavMessageType::LNAV),
            (Constellation::GPS, Version::new(4, 0), NavMessageType::CNAV),
            (Constellation::GPS, Version::new(4, 0), NavMessageType::CNV2),
            (
                Constellation::Glonass,
                Version::new(2, 0),
                NavMessageType::LNAV,
            ),
            (
                Constellation::Glonass,
                Version::new(3, 0),
                NavMessageType::LNAV,
            ),
            (
                Constellation::Galileo,
                Version::new(3, 0),
                NavMessageType::LNAV,
            ),
            (
                Constellation::Galileo,
                Version::new(4, 0),
                NavMessageType::INAV,
            ),
            (
                Constellation::Galileo,
                Version::new(4, 0),
                NavMessageType::FNAV,
            ),
            (
                Constellation::QZSS,
                Version::new(3, 0),
                NavMessageType::LNAV,
            ),
            (
                Constellation::QZSS,
                Version::new(4, 0),
                NavMessageType::LNAV,
            ),
            (
                Constellation::QZSS,
                Version::new(4, 0),
                NavMessageType::CNAV,
            ),
            (
                Constellation::QZSS,
                Version::new(4, 0),
                NavMessageType::CNV2,
            ),
            (
                Constellation::BeiDou,
                Version::new(3, 0),
                NavMessageType::LNAV,
            ),
            // NAV V4 (exact)
            (
                Constellation::BeiDou,
                Version::new(4, 0),
                NavMessageType::D1,
            ),
            (
                Constellation::BeiDou,
                Version::new(4, 0),
                NavMessageType::D2,
            ),
            (
                Constellation::BeiDou,
                Version::new(4, 0),
                NavMessageType::CNV1,
            ),
            (
                Constellation::BeiDou,
                Version::new(4, 0),
                NavMessageType::CNV2,
            ),
            (
                Constellation::BeiDou,
                Version::new(4, 0),
                NavMessageType::CNV3,
            ),
            (
                Constellation::SBAS,
                Version::new(4, 0),
                NavMessageType::SBAS,
            ),
        ] {
            let found = closest_nav_standards(constellation, rev, msg);
            assert!(
                found.is_some(),
                "should have identified {}:V{} ({}) frame that actually exists in DB",
                constellation,
                rev,
                msg
            );

            let standards = found.unwrap();

            assert!(
                standards.constellation == constellation,
                "bad constellation identified \"{}\", expecting \"{}\"",
                constellation,
                standards.constellation
            );

            assert!(
                standards.version == rev,
                "should have matched {} V{} exactly, because it exists in DB",
                constellation,
                rev,
            );
        }

        // Test cases where the nearest revision is used, not that exact revision
        for (constellation, desired, expected, msg) in [
            (
                Constellation::GPS,
                Version::new(5, 0),
                Version::new(4, 0),
                NavMessageType::LNAV,
            ),
            (
                Constellation::GPS,
                Version::new(4, 1),
                Version::new(4, 0),
                NavMessageType::LNAV,
            ),
            (
                Constellation::Glonass,
                Version::new(3, 4),
                Version::new(3, 0),
                NavMessageType::LNAV,
            ),
            (
                Constellation::BeiDou,
                Version::new(4, 2),
                Version::new(4, 0),
                NavMessageType::CNV1,
            ),
            (
                Constellation::BeiDou,
                Version::new(4, 2),
                Version::new(4, 0),
                NavMessageType::CNV2,
            ),
            (
                Constellation::BeiDou,
                Version::new(4, 2),
                Version::new(4, 0),
                NavMessageType::CNV3,
            ),
            (
                Constellation::Galileo,
                Version::new(4, 2),
                Version::new(4, 0),
                NavMessageType::INAV,
            ),
            (
                Constellation::QZSS,
                Version::new(4, 2),
                Version::new(4, 0),
                NavMessageType::LNAV,
            ),
            (
                Constellation::QZSS,
                Version::new(4, 2),
                Version::new(4, 0),
                NavMessageType::CNAV,
            ),
        ] {
            let found = closest_nav_standards(constellation, desired, msg);

            assert!(
                found.is_some(),
                "should have converged for \"{}\" V\"{}\" (\"{}\") to nearest frame revision",
                constellation,
                desired,
                msg
            );

            let standards = found.unwrap();

            assert!(
                standards.constellation == constellation,
                "bad constellation identified \"{}\", expecting \"{}\"",
                constellation,
                standards.constellation
            );

            assert!(
                standards.version == expected,
                "closest_nav_standards() converged to wrong revision {}:{}({}) while \"{}\" was expected", 
                constellation,
                desired,
                msg,
                expected);
        }
    }

    #[test]
    fn test_db_item() {
        let e = OrbitItem::U8(10);
        assert_eq!(e.as_u8(), 10);

        let e = OrbitItem::F64(10.0);
        assert_eq!(e.as_u8(), 10);
        assert_eq!(e.as_u32(), 10);
        assert_eq!(e.as_f64(), 10.0);

        let e = OrbitItem::U32(1);
        assert_eq!(e.as_u32(), 1);
        assert_eq!(e.as_f64(), 1.0);
    }

    #[test]
    fn test_orbit_channel_5() {
        let lnav = NavMessageType::LNAV;

        let _ = OrbitItem::new(
            "channel",
            "i8",
            "5.000000000000D+00",
            &lnav,
            Constellation::Glonass,
        )
        .unwrap();
    }
}
