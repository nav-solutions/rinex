//! General helpers
use crate::error::ParsingError;

/// Validate text before slicing fixed-width RINEX byte columns.
/// `first_line` is the one-based position of the first line in this context.
pub(crate) fn validate_ascii(content: &str, first_line: usize) -> Result<(), ParsingError> {
    if !content.is_ascii() {
        for (index, line) in content.lines().enumerate() {
            if let Some(column) = line.bytes().position(|byte| !byte.is_ascii()) {
                return Err(ParsingError::NonAscii {
                    line: first_line + index,
                    byte_column: column + 1,
                });
            }
        }
    }
    Ok(())
}
