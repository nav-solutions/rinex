//! Parse-time information about rejected NAV records.
use super::NavMessageType;
use crate::prelude::{Epoch, SV};

/// Why a complete NAV record was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavDiagnosticKind {
    /// A nonblank field in a known ephemeris layout could not be decoded.
    InvalidField,
    /// No supported layout, message type, or time pair was available.
    UnsupportedMessage,
}

/// One rejected record. Line numbers are one-based physical file lines.
#[derive(Clone, Debug)]
pub struct NavParseDiagnostic {
    pub kind: NavDiagnosticKind,
    pub record_line: usize,
    pub field_line: Option<usize>,
    pub slot: Option<usize>,
    pub field: Option<String>,
    pub raw: Option<String>,
    pub sv: Option<SV>,
    pub epoch: Option<Epoch>,
    pub message: Option<NavMessageType>,
    pub reason: String,
}

/// A parse-time report. It is not serialized into RINEX output.
#[derive(Clone, Debug, Default)]
pub struct NavParseReport {
    pub(crate) diagnostics: Vec<NavParseDiagnostic>,
    pub(crate) rejected_records: usize,
    pub(crate) unsupported_records: usize,
}

impl NavParseReport {
    pub fn diagnostics(&self) -> &[NavParseDiagnostic] {
        &self.diagnostics
    }

    pub fn rejected_records(&self) -> usize {
        self.rejected_records
    }

    pub fn unsupported_records(&self) -> usize {
        self.unsupported_records
    }
}
