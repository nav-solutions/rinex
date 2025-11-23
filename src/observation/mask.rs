//! Observation RINEX masking ops

use crate::{
    observation::Record,
    observation::SNR,
    prelude::{Constellation, Observable},
};

use qc_traits::{FilterItem, MaskFilter, MaskOperand};

use std::str::FromStr;

/// Applies [MaskFilter] to [Record]
pub fn mask_mut(rec: &mut Record, mask: &MaskFilter) {
    match mask.operand {
        MaskOperand::Equals => match &mask.item {
            FilterItem::EpochItem(epoch) => rec.retain(|k, _| k.epoch == *epoch),
            FilterItem::ClockItem => rec.retain(|_, obs| obs.clock.is_some()),
            FilterItem::ConstellationItem(constells) => {
                let mut broad_sbas_filter = false;
                for c in constells {
                    broad_sbas_filter |= *c == Constellation::SBAS;
                }
                rec.retain(|_, obs| {
                    obs.signals.retain(|sig| {
                        if broad_sbas_filter {
                            sig.satellite.constellation.is_sbas()
                                || constells.contains(&sig.satellite.constellation)
                        } else {
                            constells.contains(&sig.satellite.constellation)
                        }
                    });
                    !obs.signals.is_empty()
                });
            },
            FilterItem::SvItem(items) => {
                rec.retain(|_, obs| {
                    obs.signals.retain(|sig| items.contains(&sig.satellite));
                    !obs.signals.is_empty()
                });
            },
            FilterItem::SNRItem(filter) => {
                let filter = SNR::from(*filter);
                rec.retain(|_, obs| {
                    obs.signals.retain(|sig| {
                        if let Some(signal_noise_ratio) = sig.signal_noise_ratio {
                            signal_noise_ratio == filter
                        } else {
                            false // no SNR: drop out
                        }
                    });
                    !obs.signals.is_empty()
                });
            },
            FilterItem::ComplexItem(filter) => {
                // try to interprate as [Observable]
                let observables = filter
                    .iter()
                    .filter_map(|f| {
                        if let Ok(ob) = Observable::from_str(f) {
                            Some(ob)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                if !observables.is_empty() {
                    rec.retain(|_, obs| {
                        obs.signals
                            .retain(|sig| observables.contains(&sig.observable));
                        !obs.signals.is_empty()
                    });
                }
            },
            _ => {},
        }, // MaskOperand::Equals

        MaskOperand::NotEquals => match &mask.item {
            FilterItem::EpochItem(epoch) => rec.retain(|k, _| k.epoch != *epoch),
            FilterItem::ClockItem => rec.retain(|_, obs| obs.clock.is_none()),
            FilterItem::ConstellationItem(constells) => {
                rec.retain(|_, obs| {
                    obs.signals
                        .retain(|sig| !constells.contains(&sig.satellite.constellation));
                    !obs.signals.is_empty()
                });
            },
            FilterItem::SvItem(items) => {
                rec.retain(|_, obs| {
                    obs.signals.retain(|sig| !items.contains(&sig.satellite));
                    !obs.signals.is_empty()
                });
            },
            FilterItem::ComplexItem(filter) => {
                // try to interprate as [Observable]
                let observables = filter
                    .iter()
                    .filter_map(|f| {
                        if let Ok(ob) = Observable::from_str(f) {
                            Some(ob)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                if !observables.is_empty() {
                    rec.retain(|_, obs| {
                        obs.signals
                            .retain(|sig| !observables.contains(&sig.observable));
                        !obs.signals.is_empty()
                    });
                }
            },
            _ => {},
        },
        MaskOperand::GreaterEquals => match &mask.item {
            FilterItem::EpochItem(epoch) => rec.retain(|k, _| k.epoch >= *epoch),
            FilterItem::SvItem(items) => {
                rec.retain(|_, obs| {
                    obs.signals.retain(|sig| {
                        let mut retained = true;
                        for item in items {
                            if item.constellation == sig.satellite.constellation {
                                retained &= sig.satellite.prn >= item.prn;
                            }
                        }
                        retained
                    });
                    !obs.signals.is_empty()
                });
            },
            FilterItem::SNRItem(filter) => {
                let filter = SNR::from(*filter);
                rec.retain(|_, obs| {
                    obs.signals.retain(|sig| {
                        if let Some(signal_noise_ratio) = sig.signal_noise_ratio {
                            signal_noise_ratio >= filter
                        } else {
                            false // no SNR: drop out
                        }
                    });
                    !obs.signals.is_empty()
                });
            },
            _ => {},
        },
        MaskOperand::GreaterThan => match &mask.item {
            FilterItem::EpochItem(epoch) => rec.retain(|k, _| k.epoch > *epoch),
            FilterItem::SvItem(items) => {
                rec.retain(|_, obs| {
                    obs.signals.retain(|sig| {
                        let mut retained = true;
                        for item in items {
                            if item.constellation == sig.satellite.constellation {
                                retained &= sig.satellite.prn > item.prn;
                            }
                        }
                        retained
                    });
                    !obs.signals.is_empty()
                });
            },
            FilterItem::SNRItem(filter) => {
                let filter = SNR::from(*filter);
                rec.retain(|_, obs| {
                    obs.signals.retain(|sig| {
                        if let Some(signal_noise_ratio) = sig.signal_noise_ratio {
                            signal_noise_ratio > filter
                        } else {
                            false // no SNR: drop out
                        }
                    });
                    !obs.signals.is_empty()
                });
            },
            _ => {},
        },
        MaskOperand::LowerEquals => match &mask.item {
            FilterItem::EpochItem(epoch) => rec.retain(|k, _| k.epoch <= *epoch),
            FilterItem::SvItem(items) => {
                rec.retain(|_, obs| {
                    obs.signals.retain(|sig| {
                        let mut retained = true;
                        for item in items {
                            if item.constellation == sig.satellite.constellation {
                                retained &= sig.satellite.prn <= item.prn;
                            }
                        }
                        retained
                    });
                    !obs.signals.is_empty()
                });
            },
            FilterItem::SNRItem(filter) => {
                let filter = SNR::from(*filter);
                rec.retain(|_, obs| {
                    obs.signals.retain(|sig| {
                        if let Some(signal_noise_ratio) = sig.signal_noise_ratio {
                            signal_noise_ratio <= filter
                        } else {
                            false // no SNR: drop out
                        }
                    });
                    !obs.signals.is_empty()
                });
            },
            _ => {},
        },
        MaskOperand::LowerThan => match &mask.item {
            FilterItem::EpochItem(epoch) => rec.retain(|k, _| k.epoch < *epoch),
            FilterItem::SvItem(items) => {
                rec.retain(|_, obs| {
                    obs.signals.retain(|sig| {
                        let mut retained = true;
                        for item in items {
                            if item.constellation == sig.satellite.constellation {
                                retained &= sig.satellite.prn < item.prn;
                            }
                        }
                        retained
                    });
                    !obs.signals.is_empty()
                });
            },
            FilterItem::SNRItem(filter) => {
                let filter = SNR::from(*filter);
                rec.retain(|_, obs| {
                    obs.signals.retain(|sig| {
                        if let Some(signal_noise_ratio) = sig.signal_noise_ratio {
                            signal_noise_ratio < filter
                        } else {
                            false // no SNR: drop out
                        }
                    });
                    !obs.signals.is_empty()
                });
            },
            _ => {},
        },
    }
}
