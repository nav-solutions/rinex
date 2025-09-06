use crate::{
    clock::record::clock_decim_mut, meteo::decim::decim_mut as meteo_decim_mut,
    navigation::decim::decim_mut as navigation_decim_mut,
    observation::decim::decim_mut as observation_decim_mut, prelude::Rinex,
};

use qc_traits::{Decimate, DecimationFilter};

impl Decimate for Rinex {
    fn decimate(&self, f: &DecimationFilter) -> Self {
        let mut s = self.clone();
        s.decimate_mut(f);
        s
    }

    fn decimate_mut(&mut self, f: &DecimationFilter) {
        self.header.decimate_mut(f);

        if let Some(rec) = self.record.as_mut_obs() {
            observation_decim_mut(rec, f)
        } else if let Some(rec) = self.record.as_mut_nav() {
            navigation_decim_mut(rec, f)
        } else if let Some(rec) = self.record.as_mut_clock() {
            clock_decim_mut(rec, f)
        } else if let Some(rec) = self.record.as_mut_meteo() {
            meteo_decim_mut(rec, f)
        }
    }
}
