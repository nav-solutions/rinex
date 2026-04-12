use crate::{
    antex::{FrequencyDependentData, Record},
    prelude::{qc::MergeError, Carrier},
};

use std::collections::HashMap;

pub fn merge_mut(rec: &mut Record, rhs: &Record) -> Result<(), MergeError> {
    for (antenna, subset) in rhs.iter() {
        for (carrier, freqdata) in subset.iter() {
            /*
             * determine whether self contains this antenna & signal or not
             */
            let mut has_ant = false;
            let mut has_signal = false;
            for (lhs_ant, subset) in rec.iter_mut() {
                if lhs_ant == antenna {
                    has_ant |= true;
                    for (lhs_carrier, _) in subset.iter_mut() {
                        if lhs_carrier == carrier {
                            has_signal |= true;
                            break;
                        }
                    }
                    if !has_signal {
                        subset.insert(*carrier, freqdata.clone());
                    }
                }
            }
            if !has_ant {
                let mut inner = HashMap::<Carrier, FrequencyDependentData>::new();
                inner.insert(*carrier, freqdata.clone());
                rec.push((antenna.clone(), inner));
            }
        }
    }
    Ok(())
}
