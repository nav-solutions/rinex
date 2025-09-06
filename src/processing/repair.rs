use crate::{
    meteo::repair::repair_mut as meteo_repair_mut,
    navigation::repair::repair_mut as navigation_repair_mut,
    observation::repair::repair_mut as observation_repair_mut, prelude::Rinex,
};

use qc_traits::{Repair, RepairTrait};

impl RepairTrait for Rinex {
    fn repair(&self, r: Repair) -> Self {
        let mut s = self.clone();
        s.repair_mut(r);
        s
    }
    fn repair_mut(&mut self, r: Repair) {
        if let Some(rec) = self.record.as_mut_obs() {
            observation_repair_mut(rec, r);
        } else if let Some(rec) = self.record.as_mut_meteo() {
            meteo_repair_mut(rec, r);
        } else if let Some(rec) = self.record.as_mut_nav() {
            navigation_repair_mut(rec, r);
        }
    }
}
