use crate::{
    hatanaka::Compressor,
    meteo::format as format_meteo_observations,
    navigation::format as format_navigation,
    prelude::{FormattingError, Header},
    record::Record,
};

use std::io::{BufWriter, Write};

impl Record {
    pub fn format<W: Write>(
        &self,
        w: &mut BufWriter<W>,
        header: &Header,
    ) -> Result<(), FormattingError> {
        let version_major = header.version.major;

        if let Some(rec) = self.as_obs() {
            let header = header
                .obs
                .as_ref()
                .ok_or(FormattingError::MissingObservableDefinition)?;

            // Compressed format (non readable yet still ASCII)
            // following the Hatanaka Compression algorithm.
            if header.crinex.is_some() {
                let mut compressor = Compressor::default();
                compressor.v3 = version_major > 2;
                compressor.format(w, &rec, header)?;
            } else {
                for (k, v) in rec.iter() {
                    v.format(version_major == 2, k, &header, w)?;
                }
            }

            Ok(())
        } else if let Some(rec) = self.as_meteo() {
            format_meteo_observations(w, rec, header)
        } else if let Some(rec) = self.as_nav() {
            format_navigation(w, rec, header)
        } else {
            Ok(())
        }
    }
}
