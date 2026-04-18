use crate::prelude::{MeteoKey, Rinex, RinexType};

#[cfg(feature = "meteo")]
#[cfg_attr(docsrs, doc(cfg(feature = "meteo")))]
mod feature; // feature dependent, high level methods

use std::collections::btree_map::{Iter, IterMut, Keys};

impl Rinex {
    /// Returns true if this [Rinex] format is [RinexType::Meteo].
    /// ```
    /// use rinex::prelude::Rinex;
    /// let rinex = Rinex::from_file("data/MET/V2/abvi0010.15m")
    ///     .unwrap();
    /// assert!(rinex.is_meteo_rinex());
    /// ```
    pub fn is_meteo_rinex(&self) -> bool {
        self.header.rinex_type == RinexType::Meteo
    }

    /// Returns [MeteoKey] Iterator.
    /// This only applies to Meteo RINEX and will panic otherwise (bad operation).
    pub fn meteo_observation_keys(&self) -> Keys<'_, MeteoKey, f64> {
        if let Some(rec) = self.record.as_meteo() {
            rec.keys()
        } else {
            panic!("bad rinex type")
        }
    }

    /// Returns Meteo Observations Iterator.
    /// This only applies to Meteo RINEX and will panic otherwise (bad operation).
    pub fn meteo_observations_iter(&self) -> Iter<'_, MeteoKey, f64> {
        if let Some(rec) = self.record.as_meteo() {
            rec.iter()
        } else {
            panic!("bad rinex type");
        }
    }

    /// Returns mutable Meteo Observations Iterator.
    /// This only applies to Meteo RINEX and will panic otherwise (bad operation).
    pub fn meteo_observations_iter_mut(&mut self) -> IterMut<'_, MeteoKey, f64> {
        if let Some(rec) = self.record.as_mut_meteo() {
            rec.iter_mut()
        } else {
            panic!("bad rinex type");
        }
    }
}
