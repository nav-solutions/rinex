use crate::{errors::FormattingError, navigation::TimeOffset, prelude::TimeScale};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::io::{BufWriter, Write};

/// Navigation specific header fields
#[derive(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HeaderFields {
    /// [TimeOffset] definitions
    pub time_offsets: Vec<TimeOffset>,
}

impl HeaderFields {
    pub fn with_time_offset(&self, offset: TimeOffset) -> Self {
        let mut s = self.clone();
        s.time_offsets.push(offset);
        s
    }

    /// Add a [TimeOffset] definition
    pub fn add_time_offset(&mut self, offset: TimeOffset) {
        self.time_offsets.push(offset);
    }

    /// Format according to RINEX standards
    pub(crate) fn format<W: Write>(
        &self,
        w: &mut BufWriter<W>,
        major: u8,
    ) -> Result<(), FormattingError> {
        // time offsets
        match major {
            2 => {
                if let Some(t_gpst_utc) = self
                    .time_offsets
                    .iter()
                    .find(|k| k.lhs == TimeScale::GPST && k.rhs == TimeScale::UTC)
                {
                    t_gpst_utc.format_v2_delta_utc(w)?;
                }

                // TODO glonassT
                if let Some(t_glonasst_utc) = self
                    .time_offsets
                    .iter()
                    .find(|k| k.lhs == TimeScale::GPST && k.rhs == TimeScale::UTC)
                {
                    t_glonasst_utc.format_v2_corr_to_system_time(w)?;
                }
            },
            3 => {
                for time_offset in self.time_offsets.iter() {
                    time_offset.format_v3(w)?;
                }
            },
            _ => {}, // N/A
        }

        Ok(())
    }
}
