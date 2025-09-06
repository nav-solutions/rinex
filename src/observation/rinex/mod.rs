//! Observation specific high level methods
use crate::{
    observation::{ClockObservation, ObsKey, Observations, SignalObservation},
    prelude::{Rinex, RinexType},
};

#[cfg(feature = "obs")]
#[cfg_attr(docsrs, doc(cfg(feature = "obs")))]
pub(crate) mod feature; // feature dependent, high level methods

use std::collections::btree_map::{Iter, IterMut, Keys};

impl Rinex {
    /// Returns true if [Rinex] format is [RinexType::ObservationData].
    /// ```
    /// use rinex::prelude::Rinex;
    /// let rinex = Rinex::from_file("data/OBS/V3/DUTH0630.22O")
    ///     .unwrap();
    /// assert!(rinex.is_observation_rinex());
    /// ```
    pub fn is_observation_rinex(&self) -> bool {
        self.header.rinex_type == RinexType::ObservationData
    }

    /// Returns [ObsKey] Iterator.
    /// This only applies to Observation RINEX and will panic otherwise (bad operation).
    pub fn observation_keys(&self) -> Keys<'_, ObsKey, Observations> {
        if let Some(rec) = self.record.as_obs() {
            rec.keys()
        } else {
            panic!("bad rinex type");
        }
    }

    /// Returns [Observations] Iterator.
    /// This only applies to Observation RINEX and will panic otherwise (bad operation).
    pub fn observations_iter(&self) -> Iter<'_, ObsKey, Observations> {
        if let Some(rec) = self.record.as_obs() {
            rec.iter()
        } else {
            panic!("bad rinex type");
        }
    }

    /// Mutable [Observations] Iterator.
    pub fn observations_iter_mut(&mut self) -> IterMut<'_, ObsKey, Observations> {
        if let Some(rec) = self.record.as_mut_obs() {
            rec.iter_mut()
        } else {
            panic!("bad rinex type");
        }
    }

    /// Returns [SignalObservation]s Iterator.
    pub fn signal_observations_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (ObsKey, &SignalObservation)> + '_> {
        Box::new(
            self.observations_iter()
                .flat_map(|(key, obs)| obs.signals.iter().map(|sig| (*key, sig))),
        )
    }

    /// Mutable [SignalObservation]s Iterator.
    pub fn signal_observations_iter_mut(
        &mut self,
    ) -> Box<dyn Iterator<Item = (ObsKey, &mut SignalObservation)> + '_> {
        Box::new(
            self.observations_iter_mut()
                .flat_map(|(key, obs)| obs.signals.iter_mut().map(move |sig| (*key, sig))),
        )
    }

    /// Returns [ClockObservation] Iterator.
    pub fn clock_observations_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (ObsKey, ClockObservation)> + '_> {
        Box::new(self.observations_iter().filter_map(|(k, v)| {
            let clock = v.clock?;
            Some((*k, clock))
        }))
    }

    /// Mutable [ClockObservation] Iterator
    pub fn clock_observations_iter_mut(
        &mut self,
    ) -> Box<dyn Iterator<Item = (ObsKey, &mut ClockObservation)> + '_> {
        Box::new(self.observations_iter_mut().filter_map(|(k, v)| {
            if let Some(ref mut clock) = v.clock {
                Some((*k, clock))
            } else {
                None
            }
        }))
    }

    /// Pseudo Range observation Iterator
    pub fn pseudo_range_observations_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (ObsKey, &SignalObservation)> + '_> {
        Box::new(self.signal_observations_iter().filter_map(|(k, v)| {
            if v.observable.is_pseudo_range_observable() {
                Some((k, v))
            } else {
                None
            }
        }))
    }

    /// Phase Range observation Iterator
    pub fn phase_range_observations_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (ObsKey, &SignalObservation)> + '_> {
        Box::new(self.signal_observations_iter().filter_map(|(k, v)| {
            if v.observable.is_phase_range_observable() {
                Some((k, v))
            } else {
                None
            }
        }))
    }

    /// Doppler observation Iterator
    pub fn doppler_observations_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (ObsKey, &SignalObservation)> + '_> {
        Box::new(self.signal_observations_iter().filter_map(|(k, v)| {
            if v.observable.is_doppler_observable() {
                Some((k, v))
            } else {
                None
            }
        }))
    }

    /// SSI observation Iterator
    pub fn ssi_observations_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (ObsKey, &SignalObservation)> + '_> {
        Box::new(self.signal_observations_iter().filter_map(|(k, v)| {
            if v.observable.is_ssi_observable() {
                Some((k, v))
            } else {
                None
            }
        }))
    }

    //  /// Applies given AND mask in place, to all observations.
    // /// This has no effect on non observation records.
    // /// This also drops observations that did not come with an LLI flag.
    // /// Only relevant on OBS RINEX.
    // pub fn lli_and_mask_mut(&mut self, mask: observation::LliFlags) {
    //     if !self.is_observation_rinex() {
    //         return; // nothing to browse
    //     }
    //     let record = self.record.as_mut_obs().unwrap();
    //     for (_e, (_clk, sv)) in record.iter_mut() {
    //         for (_sv, obs) in sv.iter_mut() {
    //             obs.retain(|_, data| {
    //                 if let Some(lli) = data.lli {
    //                     lli.intersects(mask)
    //                 } else {
    //                     false // drops data with no LLI attached
    //                 }
    //             })
    //         }
    //     }
    // }

    // /// Removes all observations where receiver phase lock was lost.
    // /// This is only relevant on OBS RINEX.
    // pub fn lock_loss_filter_mut(&mut self) {
    //     self.lli_and_mask_mut(observation::LliFlags::LOCK_LOSS)
    // }

    // /// [`Rinex::lli_and_mask`] immutable implementation.
    // /// Only relevant on OBS RINEX.
    // pub fn lli_and_mask(&self, mask: observation::LliFlags) -> Self {
    //     let mut c = self.clone();
    //     c.lli_and_mask_mut(mask);
    //     c
    // }

    // /// Aligns Phase observations at origin
    // pub fn observation_phase_align_origin_mut(&mut self) {
    //     let mut init_phases: HashMap<SV, HashMap<Observable, f64>> = HashMap::new();
    //     if let Some(r) = self.record.as_mut_obs() {
    //         for (_, (_, vehicles)) in r.iter_mut() {
    //             for (sv, observations) in vehicles.iter_mut() {
    //                 for (observable, data) in observations.iter_mut() {
    //                     if observable.is_phase_observable() {
    //                         if let Some(init_phase) = init_phases.get_mut(sv) {
    //                             if init_phase.get(observable).is_none() {
    //                                 init_phase.insert(observable.clone(), data.obs);
    //                             }
    //                         } else {
    //                             let mut map: HashMap<Observable, f64> = HashMap::new();
    //                             map.insert(observable.clone(), data.obs);
    //                             init_phases.insert(*sv, map);
    //                         }
    //                         data.obs -= init_phases.get(sv).unwrap().get(observable).unwrap();
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    // /// Aligns Phase observations at origin,
    // /// immutable implementation
    // pub fn observation_phase_align_origin(&self) -> Self {
    //     let mut s = self.clone();
    //     s.observation_phase_align_origin_mut();
    //     s
    // }

    // /// Converts all Phase Data to Carrier Cycles by multiplying all phase points
    // /// by the carrier signal wavelength.
    // pub fn observation_phase_carrier_cycles_mut(&mut self) {
    //     if let Some(r) = self.record.as_mut_obs() {
    //         for (_, (_, vehicles)) in r.iter_mut() {
    //             for (sv, observations) in vehicles.iter_mut() {
    //                 for (observable, data) in observations.iter_mut() {
    //                     if observable.is_phase_observable() {
    //                         if let Ok(carrier) = observable.carrier(sv.constellation) {
    //                             data.obs *= carrier.wavelength();
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    // /// Converts all Phase Data to Carrier Cycles by multiplying all phase points
    // /// by the carrier signal wavelength.
    // pub fn observation_phase_carrier_cycles(&self) -> Self {
    //     let mut s = self.clone();
    //     s.observation_phase_carrier_cycles_mut();
    //     s
    // }
}
