//! Geodetic marker

use crate::{
    fmt_rinex,
    prelude::{FormattingError, ParsingError},
};

use std::io::{BufWriter, Write};

#[cfg(feature = "serde")]
use serde::Serialize;

#[derive(Default, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GeodeticMarker {
    /// Marker name
    pub name: String,
    /// Marker type
    pub marker_type: Option<MarkerType>,
    /// Marker/monument ID
    identification: Option<u32>,
    /// Marker/monument number.
    /// Probably if agency has more than one of them.
    monument: Option<u16>,
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MarkerType {
    /// Earth fixed & high precision
    #[default]
    Geodetic,
    /// Earth fixed & low precision
    NonGeodetic,
    /// Generated from network
    NonPhysical,
    /// Orbiting space vehicle
    Spaceborne,
    /// Aircraft, balloon..
    Airborne,
    /// Mobile water craft
    Watercraft,
    /// Mobile terrestrial vehicle
    Groundcraft,
    /// Fixed on water surface
    FixedBuoy,
    /// Floating on water surface
    FloatingBuoy,
    /// Floating on ice
    FloatingIce,
    /// Fixed on glacier
    Glacier,
    /// Rockets, shells, etc..
    Ballistic,
    /// Animal carrying a receiver
    Animal,
    /// Human being carrying a receiver
    Human,
}

impl std::str::FromStr for MarkerType {
    type Err = ParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().replace('_', " ").as_str() {
            "geodetic" => Ok(Self::Geodetic),
            "non geodetic" => Ok(Self::NonGeodetic),
            "ground craft" => Ok(Self::Groundcraft),
            "water craft" => Ok(Self::Watercraft),
            "airborne" => Ok(Self::Airborne),
            "non physical" => Ok(Self::NonPhysical),
            "spaceborne" => Ok(Self::Spaceborne),
            "fixed buoy" => Ok(Self::FixedBuoy),
            "floating ice" => Ok(Self::FloatingIce),
            "floating buoy" => Ok(Self::FloatingBuoy),
            "glacier" => Ok(Self::Glacier),
            "ballistic" => Ok(Self::Ballistic),
            "animal" => Ok(Self::Animal),
            "human" => Ok(Self::Human),
            _ => Err(ParsingError::MarkerType),
        }
    }
}

impl std::fmt::Display for MarkerType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let keyword = match self {
            Self::Geodetic => "GEODETIC",
            Self::NonGeodetic => "NON_GEODETIC",
            Self::NonPhysical => "NON_PHYSICAL",
            Self::Spaceborne => "SPACEBORNE",
            Self::Airborne => "AIRBORNE",
            Self::Watercraft => "WATER_CRAFT",
            Self::Groundcraft => "GROUND_CRAFT",
            Self::FixedBuoy => "FIXED_BUOY",
            Self::FloatingBuoy => "FLOATING_BUOY",
            Self::FloatingIce => "FLOATING_ICE",
            Self::Glacier => "GLACIER",
            Self::Ballistic => "BALLISTIC",
            Self::Animal => "ANIMAL",
            Self::Human => "HUMAN",
        };
        f.write_str(keyword)
    }
}

impl GeodeticMarker {
    /// Formats [GeodeticMarker] into [BufWriter]
    pub(crate) fn format<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        writeln!(w, "{}", fmt_rinex(&self.name, "MARKER NAME"))?;
        if let Some(number) = self.number() {
            writeln!(w, "{}", fmt_rinex(&number, "MARKER NUMBER"))?;
        }
        if let Some(marker_type) = &self.marker_type {
            writeln!(w, "{}", fmt_rinex(&marker_type.to_string(), "MARKER TYPE"))?;
        }
        Ok(())
    }

    /// Returns a GeodeticMarker with given "name".
    pub fn with_name(&self, name: &str) -> Self {
        let mut s = self.clone();
        s.name = name.to_string();
        s
    }
    /// Returns marker "number" in standardized format
    pub fn number(&self) -> Option<String> {
        let id = self.identification?;
        let monument = self.monument?;
        Some(format!("{:05}M{:03}", id, monument))
    }
    /// Returns a copy with a DOMES number (`DDDDDMDDD`). Other formats leave
    /// the existing number unchanged; arbitrary RINEX A20 identifiers are not represented.
    pub fn with_number(&self, content: &str) -> Self {
        let mut s = self.clone();
        if content.len() == 9 && content.chars().nth(5) == Some('M') {
            if let Ok(id) = u32::from_str_radix(&content[..5], 10) {
                if let Ok(monument) = u16::from_str_radix(&content[6..], 10) {
                    s.identification = Some(id);
                    s.monument = Some(monument);
                }
            }
        }
        s
    }
}

#[cfg(test)]
mod test {
    use super::{GeodeticMarker, MarkerType};

    #[test]
    fn standard_marker_keywords_parse_and_format() {
        // RINEX 4.02 Table 8: expected strings are specification keywords.
        for (keyword, marker_type) in [
            ("GEODETIC", MarkerType::Geodetic),
            ("NON_GEODETIC", MarkerType::NonGeodetic),
            ("NON_PHYSICAL", MarkerType::NonPhysical),
            ("SPACEBORNE", MarkerType::Spaceborne),
            ("AIRBORNE", MarkerType::Airborne),
            ("WATER_CRAFT", MarkerType::Watercraft),
            ("GROUND_CRAFT", MarkerType::Groundcraft),
            ("FIXED_BUOY", MarkerType::FixedBuoy),
            ("FLOATING_BUOY", MarkerType::FloatingBuoy),
            ("FLOATING_ICE", MarkerType::FloatingIce),
            ("GLACIER", MarkerType::Glacier),
            ("BALLISTIC", MarkerType::Ballistic),
            ("ANIMAL", MarkerType::Animal),
            ("HUMAN", MarkerType::Human),
        ] {
            assert_eq!(keyword.parse::<MarkerType>().unwrap(), marker_type);
            assert_eq!(marker_type.to_string(), keyword);
        }
        assert_eq!(
            "ground craft".parse::<MarkerType>().unwrap(),
            MarkerType::Groundcraft
        );
    }

    #[test]
    fn marker_number() {
        let marker = GeodeticMarker::default();
        let marker = marker.with_number("10118M001");
        assert_eq!(marker.number(), Some("10118M001".to_string()));
        let marker = marker.with_number("10118M123");
        assert_eq!(marker.number(), Some("10118M123".to_string()));
    }
}
