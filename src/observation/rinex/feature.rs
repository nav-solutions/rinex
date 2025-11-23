//! Feature dependent high level methods
use crate::{
    observation::{EpochFlag, LLIFlags, ObsKey, SignalObservation},
    prelude::{Carrier, Epoch, Observable, Rinex, SV},
};

use itertools::Itertools;

use std::collections::{BTreeMap, HashMap};

/// Supported signal [Combination]s
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Combination {
    /// Geometry Free (GF) combination (same physics)
    GeometryFree,

    /// Ionosphere Free (IF) combination (same physics)
    IonosphereFree,

    /// Wide Lane (Wl) combination (same physics)
    WideLane,

    /// Narrow Lane (Nl) combination (same physics)
    NarrowLane,

    /// Melbourne-Wübbena (MW) combination (cross-mixed physics)
    MelbourneWubbena,
}

impl std::fmt::Display for Combination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GeometryFree => write!(f, "Geometry Free"),
            Self::IonosphereFree => write!(f, "Ionosphere Free"),
            Self::WideLane => write!(f, "Wide Lane"),
            Self::NarrowLane => write!(f, "Narrow Lane"),
            Self::MelbourneWubbena => write!(f, "Melbourne-Wübbena"),
        }
    }
}

impl std::fmt::LowerHex for Combination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GeometryFree => write!(f, "gf"),
            Self::IonosphereFree => write!(f, "if"),
            Self::WideLane => write!(f, "wl"),
            Self::NarrowLane => write!(f, "nl"),
            Self::MelbourneWubbena => write!(f, "mw"),
        }
    }
}

/// [CombinationKey] is how we sort signal combinations
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct CombinationKey {
    /// [Epoch] of sampling
    pub epoch: Epoch,
    /// [EpochFlag]: sampling conditions
    pub flag: EpochFlag,
    /// [SV]: signal source
    pub sv: SV,
    /// Left Hand Side [Observable]
    pub lhs: Observable,
    /// Reference [Observable]
    pub reference: Observable,
}

/// [MultipathKey] is how we sort code multipath values
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct MultipathKey {
    /// [Epoch] of sampling
    pub epoch: Epoch,
    /// [SV]: signal source
    pub sv: SV,
    /// Phase reference [Observable]. Retrieve the
    /// [Carrier] signal for which the MP applies, with [Observable::carrier].
    pub signal: Observable,
    /// Comparison Phase [Observable].
    pub rhs: Observable,
}

impl Rinex {
    /// Returns [Carrier] signals Iterator
    pub fn carrier_iter(&self) -> Box<dyn Iterator<Item = Carrier> + '_> {
        Box::new(
            self.signal_observations_iter()
                .filter_map(|(_, sig)| {
                    if let Ok(carrier) =
                        Carrier::from_observable(sig.satellite.constellation, &sig.observable)
                    {
                        Some(carrier)
                    } else {
                        None
                    }
                })
                .unique(),
        )
    }

    /// Returns Signal Code Iterator (official RINEX denomination).
    /// For example, "1C" means L1 civilian, or "1P" means L1 precision code.
    pub fn signal_code_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
        Box::new(
            self.signal_observations_iter()
                .filter_map(|(_, sig)| {
                    let code = sig.observable.code()?;
                    Some(code)
                })
                .unique(),
        )
    }

    /// [SignalObservation]s Iterator for which sampling conditions were marked OK.
    /// For [Observable::PhaseRange] you should verify the tracking status as well, for completeness.
    /// You can use:
    /// - [Self::pseudo_range_sampling_ok_iter()] if you're only interested in decoded pseudo range
    /// - [Self::phase_range_sampling_ok_iter()] if you're only interested in estimated phase range
    pub fn signal_observations_sampling_ok_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (Epoch, &SignalObservation)> + '_> {
        Box::new(self.signal_observations_iter().filter_map(|(k, sig)| {
            if k.flag.is_ok() {
                Some((k.epoch, sig))
            } else {
                None
            }
        }))
    }

    /// [Self::signal_observations_sampling_ok_iter()] with [Observable::PseudoRange] mask applied.
    pub fn pseudo_range_sampling_ok_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (Epoch, &SignalObservation)> + '_> {
        Box::new(
            self.signal_observations_sampling_ok_iter()
                .filter_map(|(k, sig)| {
                    if sig.observable.is_pseudo_range_observable() {
                        Some((k, sig))
                    } else {
                        None
                    }
                }),
        )
    }

    /// Returns an Iterator over [Epoch]s where [Observable::PhaseRange] sampling took place in good conditions.
    /// For good tracking conditions, you need to take the attached [LLIFlags] into account.
    /// See [Rinex::signal_ok_iter()] for more information.
    /// Use [Rinex::phase_range_tracking_ok_iter()] for both good sampling and tracking conditions filtering.
    /// Use [Rinex::phase_tracking_issues_iter()] for tracking errors iteration.
    pub fn phase_range_sampling_ok_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (Epoch, &SignalObservation)> + '_> {
        Box::new(
            self.signal_observations_sampling_ok_iter()
                .filter_map(|(k, sig)| {
                    if sig.observable.is_phase_range_observable() {
                        Some((k, sig))
                    } else {
                        None
                    }
                }),
        )
    }

    /// Returns an Iterator over [Epoch]s where [Observable::PhaseRange] sampling and tracking took place in good conditions.
    /// Use [Rinex::phase_range_sampling_ok_iter()] to verify the sampling conditions only.
    /// NB: if [LLIFlags] is missing, we consider the tracking was GOOD. Because some receivers omit or do not encode
    /// the [LLIFlags] and would rather not stream out RINEX in such situation (and we would wind up with empty dataset).
    pub fn phase_range_tracking_ok_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (Epoch, &SignalObservation)> + '_> {
        Box::new(self.phase_range_sampling_ok_iter().filter_map(|(k, sig)| {
            if let Some(lli) = sig.lli_flags {
                if lli.intersects(LLIFlags::OK_OR_UNKNOWN) {
                    Some((k, sig))
                } else {
                    None
                }
            } else {
                Some((k, sig))
            }
        }))
    }

    /// Returns Iterator over Phase Cycle slips events.
    pub fn phase_cycle_slip_events(
        &self,
    ) -> Box<dyn Iterator<Item = (ObsKey, &SignalObservation)> + '_> {
        Box::new(self.signal_observations_iter().filter_map(|(k, sig)| {
            if sig.observable.is_phase_range_observable() {
                if let Some(lli) = sig.lli_flags {
                    if lli.intersects(LLIFlags::LOCK_LOSS) {
                        Some((k, sig))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }))
    }

    /// Returns Iterator over both Half and Full Phase Cycle slips events.
    /// Use provided [LLIFlags] to inquire.
    pub fn phase_half_full_cycle_slip_events(
        &self,
    ) -> Box<dyn Iterator<Item = (ObsKey, &SignalObservation)> + '_> {
        Box::new(self.phase_range_observations_iter().filter_map(|(k, sig)| {
            let lli = sig.lli_flags?;
            if lli.intersects(LLIFlags::LOCK_LOSS) || lli.intersects(LLIFlags::HALF_CYCLE_SLIP) {
                Some((k, sig))
            } else {
                None
            }
        }))
    }

    /// Copies and returns a new [Rinex] where [LLIFlags] mask (and mask) was applied.
    /// This only impacts Observation RINEX.
    pub fn observation_phase_tracking_lli_masking(&self, mask: LLIFlags) -> Self {
        let mut s = self.clone();
        s.observation_phase_tracking_lli_masking_mut(mask);
        s
    }

    /// Mutable [Self::observation_phase_tracking_lli_masking] implementation.
    /// This only impacts Observation RINEX.
    pub fn observation_phase_tracking_lli_masking_mut(&mut self, mask: LLIFlags) {
        if let Some(rec) = self.record.as_mut_obs() {
            rec.retain(|_, v| {
                v.signals.retain(|sig| {
                    if sig.observable.is_phase_range_observable() {
                        if let Some(lli) = sig.lli_flags {
                            lli.intersects(mask)
                        } else {
                            false
                        }
                    } else {
                        true
                    }
                });
                !v.signals.is_empty()
            })
        }
    }

    /// Returns an Iterator over [Epoch]s where abnormal sampling conditions were detected.
    /// Anomalies are described by the attached [EpochFlag] in each [ObsKey].
    pub fn epoch_anomalies(&self) -> Box<dyn Iterator<Item = &ObsKey> + '_> {
        Box::new(self.observation_keys().filter(|k| k.flag.is_ok()))
    }

    /// Form designed signal [Combination] from all observed signals.
    /// Unit is depend on [Observable]s being combined.
    /// But usually, [Observable::PhaseRange] and [Observable::PseudoRange] are intended here,
    /// meaning the return value is in meters of recombined frequency propagation.
    pub fn signals_combination(&self, combination: Combination) -> BTreeMap<CombinationKey, f64> {
        let mut ret = BTreeMap::new();

        if combination == Combination::MelbourneWubbena {
            for ((k_1, v_1), (_, v_2)) in self
                .signals_combination(Combination::WideLane)
                .iter()
                .filter(|(k, _)| k.reference.is_phase_range_observable())
                .zip(
                    self.signals_combination(Combination::NarrowLane)
                        .iter()
                        .filter(|(k, _)| k.reference.is_pseudo_range_observable()),
                )
                .filter_map(|((k_1, v_1), (k_2, v_2))| {
                    let lhs_1_code = k_1.lhs.code()?;
                    let lhs_2_code = k_2.lhs.code()?;
                    if lhs_1_code == lhs_2_code {
                        Some(((k_1, v_1), (k_2, v_2)))
                    } else {
                        None
                    }
                })
            {
                let key = CombinationKey {
                    epoch: k_1.epoch,
                    flag: k_1.flag,
                    sv: k_1.sv,
                    lhs: k_1.lhs.clone(),
                    reference: k_1.reference.clone(),
                };
                ret.insert(key, v_1 - v_2);
            }
        } else {
            let dominant_sampling = if let Some(dt) = self.header.sampling_interval {
                dt
            } else {
                if let Some(dt) = self.dominant_sampling_interval() {
                    dt
                } else {
                    // can't proceed without sampling interval guess.
                    return ret;
                }
            };

            let mut pr_ref_value = HashMap::<SV, (Epoch, Observable, f64, f64, f64)>::new();
            let mut phase_ref_value = HashMap::<SV, (Epoch, Observable, f64, f64, f64)>::new();

            for (k, v) in self.signal_observations_iter() {
                let is_phase_range = v.observable.is_phase_range_observable();
                let is_pseudo_range = v.observable.is_pseudo_range_observable();

                if !is_phase_range && !is_pseudo_range {
                    continue;
                }

                let is_l1_pivot = v.observable.is_l1_pivot(v.satellite.constellation);

                let carrier = v.observable.to_carrier(v.satellite.constellation);
                if carrier.is_err() {
                    continue;
                }

                let carrier = carrier.unwrap();
                let freq = carrier.frequency();
                let lambda = carrier.wavelength();

                if is_l1_pivot {
                    if is_phase_range {
                        phase_ref_value.insert(
                            v.satellite,
                            (k.epoch, v.observable.clone(), freq, lambda, v.value),
                        );
                    } else {
                        pr_ref_value.insert(
                            v.satellite,
                            (k.epoch, v.observable.clone(), freq, lambda, v.value),
                        );
                    };
                } else {
                    let reference = if is_phase_range {
                        &phase_ref_value
                    } else {
                        &pr_ref_value
                    };

                    if let Some((last_seen, reference, f_1, w_1, ref_value)) =
                        reference.get(&v.satellite)
                    {
                        if k.epoch - *last_seen > dominant_sampling {
                            // avoid moving forward on data gaps
                            // guards against substraction (a - b) from two different epochs
                            continue;
                        }

                        let alpha = match combination {
                            Combination::GeometryFree => 1.0,
                            Combination::IonosphereFree => f_1.powi(2),
                            Combination::WideLane => *f_1,
                            Combination::NarrowLane => *f_1,
                            Combination::MelbourneWubbena => {
                                unreachable!("mw combination");
                            },
                        };

                        let beta = match combination {
                            Combination::GeometryFree => -1.0,
                            Combination::IonosphereFree => -freq.powi(2),
                            Combination::WideLane => -freq,
                            Combination::NarrowLane => freq,
                            Combination::MelbourneWubbena => {
                                unreachable!("mw combination");
                            },
                        };

                        let gamma = match combination {
                            Combination::GeometryFree => 1.0,
                            Combination::IonosphereFree => f_1.powi(2) - freq.powi(2),
                            Combination::WideLane => *f_1 - freq,
                            Combination::NarrowLane => *f_1 + freq,
                            Combination::MelbourneWubbena => {
                                unreachable!("mw combination");
                            },
                        };

                        let (v_lhs, v_rhs) = match combination {
                            Combination::GeometryFree => {
                                if v.observable.is_phase_range_observable() {
                                    (*ref_value * w_1, v.value * lambda)
                                } else {
                                    (v.value, *ref_value)
                                }
                            },
                            _ => {
                                if v.observable.is_phase_range_observable() {
                                    (*ref_value * w_1, v.value * lambda)
                                } else {
                                    (*ref_value, v.value)
                                }
                            },
                        };

                        let combination = CombinationKey {
                            epoch: k.epoch,
                            flag: k.flag,
                            sv: v.satellite,
                            lhs: v.observable.clone(),
                            reference: reference.clone(),
                        };

                        ret.insert(combination, (alpha * v_lhs + beta * v_rhs) / gamma);
                    }
                }
            }
        }
        ret
    }

    /// Calculates the signal multipath bias (as meters of propagation delay)
    /// for all SV in sight and from dual frequency phase measurement.
    /// Note that this is not the absolute multipath bias because
    /// - it does not integrate the K_1_j "static" bias induced by differential code bias
    /// between SV and RX. You should apply it yourself on the provided results
    /// - the receiver own noise is contained in the result. Results should be averaged to mitigate.
    pub fn signals_multipath(&self) -> HashMap<MultipathKey, f64> {
        let mut ret = HashMap::new();

        let dominant_sampling = if let Some(dt) = self.header.sampling_interval {
            dt
        } else {
            if let Some(dt) = self.dominant_sampling_interval() {
                dt
            } else {
                // can't proceed without sampling interval guess.
                return ret;
            }
        };

        let mut t = Epoch::default();

        // stores all encountered pseudo range
        let mut rho = HashMap::<(SV, Observable), f64>::new();
        // stores all encountered phase range
        let mut phi = HashMap::<(SV, Observable), (f64, f64)>::new();

        for (k, v) in self.signal_observations_iter() {
            let is_ph = v.observable.is_phase_range_observable();
            let is_pr = v.observable.is_pseudo_range_observable();
            let carrier = v.observable.to_carrier(v.satellite.constellation);

            if !is_ph && !is_pr || carrier.is_err() {
                continue;
            }

            let carrier = carrier.unwrap();
            let freq = carrier.frequency().powi(2);
            let lambda = carrier.wavelength();

            if k.epoch - t > dominant_sampling {
                // data gap
                rho.clear();
                phi.clear();
            }

            if k.epoch - t == dominant_sampling {
                // new epoch
                // try to compute from previously stored data, then reset
                for ((phi_sv, phi_obs), (freq, phi_value)) in phi.iter() {
                    let phi_code = phi_obs.code();
                    if phi_code.is_none() {
                        continue;
                    }

                    let phi_code = phi_code.unwrap();

                    // locate associated pr
                    for ((rho_sv, rho_obs), rho) in rho.iter() {
                        let rho_code = rho_obs.code();
                        if rho_code.is_none() {
                            continue;
                        }

                        let rho_code = rho_code.unwrap();

                        // same modulation / signal as reference phase
                        if rho_code == phi_code && rho_sv == phi_sv {
                            // now compute for any different l_j phase
                            for ((phi_sv_j, phi_obs_j), (freq_j, phi_j)) in phi.iter() {
                                if phi_sv_j == phi_sv {
                                    if phi_obs_j != phi_obs {
                                        let key = MultipathKey {
                                            epoch: t,
                                            sv: *phi_sv,
                                            signal: phi_obs.clone(),
                                            rhs: phi_obs_j.clone(),
                                        };
                                        let value = rho
                                            - phi_value
                                            - 2.0 * freq_j / (freq - freq_j) * (phi_value - phi_j);
                                        ret.insert(key, value);
                                    }
                                }
                            }
                        }
                    }
                }
                //reset
                rho.clear();
                phi.clear();
            }

            // store value
            if is_pr {
                rho.insert((v.satellite, v.observable.clone()), v.value);
            } else {
                phi.insert(
                    (v.satellite, v.observable.clone()),
                    (freq, v.value * lambda),
                );
            }

            t = k.epoch;
        }
        ret
    }
}

#[cfg(test)]
mod test {

    use super::Combination;
    use crate::prelude::{Carrier, Rinex};

    #[test]
    fn gf_signal_combination() {
        let fullpath = format!(
            "{}/data/OBS/V3/ACOR00ESP_R_20213550000_01D_30S_MO.rnx",
            env!("CARGO_MANIFEST_DIR")
        );
        let rinex = Rinex::from_file(&fullpath).unwrap();

        let mut tests_passed = 0;

        let gps_l1 = Carrier::L1.wavelength();
        let gps_l2 = Carrier::L2.wavelength();
        let gps_l5 = Carrier::L5.wavelength();

        for (k, value) in rinex.signals_combination(Combination::GeometryFree) {
            assert!(k.flag.is_ok(), "sampling flag has been deformed!");
            let epoch = k.epoch.to_string();
            let sv = k.sv.to_string();
            let rhs = k.reference.to_string();
            let lhs = k.lhs.to_string();

            match epoch.as_str() {
                "2021-12-21T00:00:00 GPST" => match sv.as_str() {
                    "G01" => match rhs.as_str() {
                        "L1C" => match lhs.as_str() {
                            "L2S" => {
                                assert_eq!(value, 129274705.784 * gps_l1 - 100733552.500 * gps_l2);
                                tests_passed += 1;
                            },
                            "L2W" => {
                                assert_eq!(value, 129274705.784 * gps_l1 - 100733552.498 * gps_l2);
                                tests_passed += 1;
                            },
                            "L5Q" => {
                                assert_eq!(value, 129274705.784 * gps_l1 - 96536320.758 * gps_l5);
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        "C1C" => match lhs.as_str() {
                            "C2S" => {
                                assert_eq!(value, 24600162.420 - 24600158.420);
                                tests_passed += 1;
                            },
                            "C2W" => {
                                assert_eq!(value, 24600162.100 - 24600158.420);
                                tests_passed += 1;
                            },
                            "C5Q" => {
                                assert_eq!(value, 24600160.900 - 24600158.420);
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        obs => panic!("invalid RHS (ref) observable: {}", obs),
                    },
                    "G07" => match rhs.as_str() {
                        "L1C" => match lhs.as_str() {
                            "L2S" => {
                                assert_eq!(value, 125167854.812 * gps_l1 - 97533394.668 * gps_l2);
                                tests_passed += 1;
                            },
                            "L2W" => {
                                assert_eq!(value, 125167854.812 * gps_l1 - 97533382.650 * gps_l2);
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        "C1C" => match lhs.as_str() {
                            "C2S" => {
                                assert_eq!(value, 23818653.000 - 23818653.240);
                                tests_passed += 1;
                            },
                            "C2W" => {
                                assert_eq!(value, 23818652.720 - 23818653.240);
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        obs => panic!("invalid RHS (ref) observable: {}", obs),
                    },
                    _ => {},
                },
                _ => {},
            }
        }
        assert_eq!(tests_passed, 10);
    }

    #[test]
    fn if_signal_combination() {
        let fullpath = format!(
            "{}/data/OBS/V3/ACOR00ESP_R_20213550000_01D_30S_MO.rnx",
            env!("CARGO_MANIFEST_DIR")
        );
        let rinex = Rinex::from_file(&fullpath).unwrap();

        let mut tests_passed = 0;

        let gps_l1 = Carrier::L1.frequency().powi(2);
        let gps_l2 = Carrier::L2.frequency().powi(2);
        let gps_l5 = Carrier::L5.frequency().powi(2);

        let gps_w1 = Carrier::L1.wavelength();
        let gps_w2 = Carrier::L2.wavelength();
        let gps_w5 = Carrier::L5.wavelength();

        for (k, value) in rinex.signals_combination(Combination::IonosphereFree) {
            assert!(k.flag.is_ok(), "sampling flag has been deformed!");
            let epoch = k.epoch.to_string();
            let sv = k.sv.to_string();
            let rhs = k.reference.to_string();
            let lhs = k.lhs.to_string();

            match epoch.as_str() {
                "2021-12-21T00:00:00 GPST" => match sv.as_str() {
                    "G01" => match rhs.as_str() {
                        "L1C" => match lhs.as_str() {
                            "L2S" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 129274705.784
                                        - gps_l2 * gps_w2 * 100733552.500)
                                        / (gps_l1 - gps_l2))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            "L2W" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 129274705.784
                                        - gps_l2 * gps_w2 * 100733552.498)
                                        / (gps_l1 - gps_l2))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            "L5Q" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 129274705.784
                                        - gps_l5 * gps_w5 * 96536320.758)
                                        / (gps_l1 - gps_l5))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        "C1C" => match lhs.as_str() {
                            "C2S" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 24600158.420 - gps_l2 * 24600162.420)
                                        / (gps_l1 - gps_l2)
                                );
                                tests_passed += 1;
                            },
                            "C2W" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 24600158.420 - gps_l2 * 24600162.100)
                                        / (gps_l1 - gps_l2)
                                );
                                tests_passed += 1;
                            },
                            "C5Q" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 24600158.420 - gps_l5 * 24600160.900)
                                        / (gps_l1 - gps_l5)
                                );
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        obs => panic!("invalid RHS (ref) observable: {}", obs),
                    },
                    "G07" => match rhs.as_str() {
                        "L1C" => match lhs.as_str() {
                            "L2S" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 125167854.812
                                        - gps_l2 * gps_w2 * 97533394.668)
                                        / (gps_l1 - gps_l2))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            "L2W" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 125167854.812
                                        - gps_l2 * gps_w2 * 97533382.650)
                                        / (gps_l1 - gps_l2))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        "C1C" => match lhs.as_str() {
                            "C2S" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 23818653.240 - gps_l2 * 23818653.000)
                                        / (gps_l1 - gps_l2)
                                );
                                tests_passed += 1;
                            },
                            "C2W" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 23818653.240 - gps_l2 * 23818652.720)
                                        / (gps_l1 - gps_l2)
                                );
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        obs => panic!("invalid RHS (ref) observable: {}", obs),
                    },
                    _ => {},
                },
                _ => {},
            }
        }
        assert_eq!(tests_passed, 10);
    }

    #[test]
    fn wl_signal_combination() {
        let fullpath = format!(
            "{}/data/OBS/V3/ACOR00ESP_R_20213550000_01D_30S_MO.rnx",
            env!("CARGO_MANIFEST_DIR")
        );
        let rinex = Rinex::from_file(&fullpath).unwrap();

        let mut tests_passed = 0;

        let gps_l1 = Carrier::L1.frequency();
        let gps_l2 = Carrier::L2.frequency();
        let gps_l5 = Carrier::L5.frequency();

        let gps_w1 = Carrier::L1.wavelength();
        let gps_w2 = Carrier::L2.wavelength();
        let gps_w5 = Carrier::L5.wavelength();

        for (k, value) in rinex.signals_combination(Combination::WideLane) {
            assert!(k.flag.is_ok(), "sampling flag has been deformed!");
            let epoch = k.epoch.to_string();
            let sv = k.sv.to_string();
            let rhs = k.reference.to_string();
            let lhs = k.lhs.to_string();

            match epoch.as_str() {
                "2021-12-21T00:00:00 GPST" => match sv.as_str() {
                    "G01" => match rhs.as_str() {
                        "L1C" => match lhs.as_str() {
                            "L2S" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 129274705.784
                                        - gps_l2 * gps_w2 * 100733552.500)
                                        / (gps_l1 - gps_l2))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            "L2W" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 129274705.784
                                        - gps_l2 * gps_w2 * 100733552.498)
                                        / (gps_l1 - gps_l2))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            "L5Q" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 129274705.784
                                        - gps_l5 * gps_w5 * 96536320.758)
                                        / (gps_l1 - gps_l5))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        "C1C" => match lhs.as_str() {
                            "C2S" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 24600158.420 - gps_l2 * 24600162.420)
                                        / (gps_l1 - gps_l2)
                                );
                                tests_passed += 1;
                            },
                            "C2W" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 24600158.420 - gps_l2 * 24600162.100)
                                        / (gps_l1 - gps_l2)
                                );
                                tests_passed += 1;
                            },
                            "C5Q" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 24600158.420 - gps_l5 * 24600160.900)
                                        / (gps_l1 - gps_l5)
                                );
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        obs => panic!("invalid RHS (ref) observable: {}", obs),
                    },
                    "G07" => match rhs.as_str() {
                        "L1C" => match lhs.as_str() {
                            "L2S" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 125167854.812
                                        - gps_l2 * gps_w2 * 97533394.668)
                                        / (gps_l1 - gps_l2))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            "L2W" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 125167854.812
                                        - gps_l2 * gps_w2 * 97533382.650)
                                        / (gps_l1 - gps_l2))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        "C1C" => match lhs.as_str() {
                            "C2S" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 23818653.240 - gps_l2 * 23818653.000)
                                        / (gps_l1 - gps_l2)
                                );
                                tests_passed += 1;
                            },
                            "C2W" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 23818653.240 - gps_l2 * 23818652.720)
                                        / (gps_l1 - gps_l2)
                                );
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        obs => panic!("invalid RHS (ref) observable: {}", obs),
                    },
                    _ => {},
                },
                _ => {},
            }
        }
        assert_eq!(tests_passed, 10);
    }

    #[test]
    fn nl_signal_combination() {
        let fullpath = format!(
            "{}/data/OBS/V3/ACOR00ESP_R_20213550000_01D_30S_MO.rnx",
            env!("CARGO_MANIFEST_DIR")
        );
        let rinex = Rinex::from_file(&fullpath).unwrap();

        let mut tests_passed = 0;

        let gps_l1 = Carrier::L1.frequency();
        let gps_l2 = Carrier::L2.frequency();
        let gps_l5 = Carrier::L5.frequency();

        let gps_w1 = Carrier::L1.wavelength();
        let gps_w2 = Carrier::L2.wavelength();
        let gps_w5 = Carrier::L5.wavelength();

        for (k, value) in rinex.signals_combination(Combination::NarrowLane) {
            assert!(k.flag.is_ok(), "sampling flag has been deformed!");
            let epoch = k.epoch.to_string();
            let sv = k.sv.to_string();
            let rhs = k.reference.to_string();
            let lhs = k.lhs.to_string();

            match epoch.as_str() {
                "2021-12-21T00:00:00 GPST" => match sv.as_str() {
                    "G01" => match rhs.as_str() {
                        "L1C" => match lhs.as_str() {
                            "L2S" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 129274705.784
                                        + gps_l2 * gps_w2 * 100733552.500)
                                        / (gps_l1 + gps_l2))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            "L2W" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 129274705.784
                                        + gps_l2 * gps_w2 * 100733552.498)
                                        / (gps_l1 + gps_l2))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            "L5Q" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 129274705.784
                                        + gps_l5 * gps_w5 * 96536320.758)
                                        / (gps_l1 + gps_l5))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        "C1C" => match lhs.as_str() {
                            "C2S" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 24600158.420 + gps_l2 * 24600162.420)
                                        / (gps_l1 + gps_l2)
                                );
                                tests_passed += 1;
                            },
                            "C2W" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 24600158.420 + gps_l2 * 24600162.100)
                                        / (gps_l1 + gps_l2)
                                );
                                tests_passed += 1;
                            },
                            "C5Q" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 24600158.420 + gps_l5 * 24600160.900)
                                        / (gps_l1 + gps_l5)
                                );
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        obs => panic!("invalid RHS (ref) observable: {}", obs),
                    },
                    "G07" => match rhs.as_str() {
                        "L1C" => match lhs.as_str() {
                            "L2S" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 125167854.812
                                        + gps_l2 * gps_w2 * 97533394.668)
                                        / (gps_l1 + gps_l2))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            "L2W" => {
                                let err = (value
                                    - (gps_l1 * gps_w1 * 125167854.812
                                        + gps_l2 * gps_w2 * 97533382.650)
                                        / (gps_l1 + gps_l2))
                                    .abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        "C1C" => match lhs.as_str() {
                            "C2S" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 23818653.240 + gps_l2 * 23818653.000)
                                        / (gps_l1 + gps_l2)
                                );
                                tests_passed += 1;
                            },
                            "C2W" => {
                                assert_eq!(
                                    value,
                                    (gps_l1 * 23818653.240 + gps_l2 * 23818652.720)
                                        / (gps_l1 + gps_l2)
                                );
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        obs => panic!("invalid RHS (ref) observable: {}", obs),
                    },
                    _ => {},
                },
                _ => {},
            }
        }
        assert_eq!(tests_passed, 10);
    }

    #[test]
    fn mw_signal_combination() {
        let fullpath = format!(
            "{}/data/OBS/V3/ACOR00ESP_R_20213550000_01D_30S_MO.rnx",
            env!("CARGO_MANIFEST_DIR")
        );
        let rinex = Rinex::from_file(&fullpath).unwrap();

        let mut tests_passed = 0;

        let gps_l1 = Carrier::L1.frequency();
        let gps_w1 = Carrier::L1.wavelength();

        let gps_l2 = Carrier::L2.frequency();
        let gps_w2 = Carrier::L2.wavelength();

        let gps_l5 = Carrier::L5.frequency();
        let gps_w5 = Carrier::L5.wavelength();

        for (k, value) in rinex.signals_combination(Combination::MelbourneWubbena) {
            assert!(k.flag.is_ok(), "sampling flag has been deformed!");
            let epoch = k.epoch.to_string();
            let sv = k.sv.to_string();
            let rhs = k.reference.to_string();
            let lhs = k.lhs.to_string();

            match epoch.as_str() {
                "2021-12-21T00:00:00 GPST" => match sv.as_str() {
                    "G01" => match rhs.as_str() {
                        "L1C" => match lhs.as_str() {
                            "L2S" => {
                                let phase = (gps_l1 * gps_w1 * 129274705.784
                                    - gps_l2 * gps_w2 * 100733552.500)
                                    / (gps_l1 - gps_l2);

                                let code = (gps_l1 * 24600158.420 + gps_l2 * 24600162.420)
                                    / (gps_l1 + gps_l2);

                                let err = (value - (phase - code)).abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            "L2W" => {
                                let phase = (gps_l1 * gps_w1 * 129274705.784
                                    - gps_l2 * gps_w2 * 100733552.498)
                                    / (gps_l1 - gps_l2);

                                let code = (gps_l1 * 24600158.420 + gps_l2 * 24600162.100)
                                    / (gps_l1 + gps_l2);

                                let err = (value - (phase - code)).abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            "L5Q" => {
                                let phase = (gps_l1 * gps_w1 * 129274705.784
                                    - gps_l5 * gps_w5 * 96536320.758)
                                    / (gps_l1 - gps_l5);

                                let code = (gps_l1 * 24600158.420 + gps_l5 * 24600160.900)
                                    / (gps_l1 + gps_l5);

                                let err = (value - (phase - code)).abs();
                                assert!(err < 1.0E-6);

                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        obs => panic!("invalid RHS (ref) observable: {}", obs),
                    },
                    "G07" => match rhs.as_str() {
                        "L1C" => match lhs.as_str() {
                            "L2S" => {
                                let phase = (gps_l1 * gps_w1 * 125167854.812
                                    - gps_l2 * gps_w2 * 97533394.668)
                                    / (gps_l1 - gps_l2);

                                let code = (gps_l1 * 23818653.240 + gps_l2 * 23818653.000)
                                    / (gps_l1 + gps_l2);

                                let err = (value - (phase - code)).abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            "L2W" => {
                                let phase = (gps_l1 * gps_w1 * 125167854.812
                                    - gps_l2 * gps_w2 * 97533382.650)
                                    / (gps_l1 - gps_l2);

                                let code = (gps_l1 * 23818653.240 + gps_l2 * 23818652.720)
                                    / (gps_l1 + gps_l2);

                                let err = (value - (phase - code)).abs();
                                assert!(err < 1.0E-6);
                                tests_passed += 1;
                            },
                            obs => panic!("invalid LHS observable: {}", obs),
                        },
                        obs => panic!("invalid RHS (ref) observable: {}", obs),
                    },
                    _ => {},
                },
                _ => {},
            }
        }
        assert_eq!(tests_passed, 5);
    }

    #[test]
    #[ignore]
    fn code_multipath() {
        let fullpath = format!(
            "{}/data/OBS/V3/ACOR00ESP_R_20213550000_01D_30S_MO.rnx",
            env!("CARGO_MANIFEST_DIR")
        );

        let rinex = Rinex::from_file(&fullpath).unwrap();

        let mut tests_passed = 0;

        let gps_l1 = Carrier::L1.frequency();
        let gps_l2 = Carrier::L2.frequency();
        let gps_l5 = Carrier::L5.frequency();

        let gps_w1 = Carrier::L1.wavelength();
        let gps_w2 = Carrier::L1.wavelength();
        let gps_w5 = Carrier::L1.wavelength();

        let mut tests_passed = 0;

        for (k, value) in rinex.signals_multipath() {
            let epoch = k.epoch.to_string();
            let signal = k.signal.to_string();
            let sv = k.sv.to_string();
            let rhs = k.rhs.to_string();

            match epoch.as_str() {
                "2021-12-21T00:00:00 GPST" => match sv.as_str() {
                    "G01" => match signal.as_str() {
                        "L1C" => match rhs.as_str() {
                            "L2S" => {
                                let err = (value
                                    - 24600158.420
                                    - 129274705.784 * gps_w1
                                    - 2.0 * gps_l2 / (gps_l1 - gps_l2)
                                        * (129274705.784 * gps_w1 - 100733552.500 * gps_w2))
                                    .abs();
                                assert!(err < 1.0E-6, "error too large: {}", value);
                                tests_passed += 1;
                            },
                            _ => {},
                        },
                        _ => {},
                    },
                    _ => {}, // sv
                },
                _ => {}, // epoch
            }
        }
        assert_eq!(tests_passed, 1);
    }
}
