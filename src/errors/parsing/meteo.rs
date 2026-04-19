//! Errors specific to Observation RINEX files parsing
use thiserror::Error;

/// Errors that may arise during the parsing of Meteo
/// RINEX files, either in the Header section (mandatory prior
/// file processing) or during Record section.
#[derive(Debug, Error)]
pub enum MeteoRINEXParsingError {}
