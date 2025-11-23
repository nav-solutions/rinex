use crate::{observation::Record, prelude::qc::MergeError};

pub fn merge_mut(rec: &mut Record, rhs: &Record) -> Result<(), MergeError> {
    for (k, rhs) in rhs.iter() {
        if let Some(lhs) = rec.get_mut(k) {
            // TODO: could merge clock field
            //  but only if receivers do match exactly !
            for rhs in rhs.signals.iter() {
                if let Some(lhs) = lhs
                    .signals
                    .iter_mut()
                    .find(|sig| sig.satellite == rhs.satellite && sig.observable == rhs.observable)
                {
                    if let Some(lli_flags) = rhs.lli_flags {
                        if lhs.lli_flags.is_none() {
                            lhs.lli_flags = Some(lli_flags);
                        }
                    }
                    if let Some(signal_noise_ratio) = rhs.signal_noise_ratio {
                        if lhs.signal_noise_ratio.is_none() {
                            lhs.signal_noise_ratio = Some(signal_noise_ratio);
                        }
                    }
                } else {
                    lhs.signals.push(rhs.clone());
                }
            }
        } else {
            rec.insert(*k, rhs.clone());
        }
    }
    Ok(())
}
