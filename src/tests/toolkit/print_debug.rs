//! debug print/panic for CI/CD advanced tests

use crate::prelude::Constellation;
use crate::prelude::Observable;
use crate::prelude::SV;

/// Print the diff in terms of SV (unique'd and sorted)
/// and eventually panics.
pub fn print_panic_sv_diffs(dut: &Vec<SV>, model: &Vec<SV>) {
    for dut_sat in dut.iter() {
        let mut found = false;
        for model_sat in model.iter() {
            if model_sat == dut_sat {
                found = true;
                break;
            }
        }
        if !found {
            panic!("found unexpected satellite {} in dataset", dut_sat);
        }
    }

    for model_sat in model.iter() {
        let mut found = false;
        for dut_sat in dut.iter() {
            if model_sat == dut_sat {
                found = true;
                break;
            }
        }
        if !found {
            panic!("satellite {} is missing from dataset", model_sat);
        }
    }
}

/// Print the diff in terms of GNSS (unique'd and sorted)
/// and eventually panics.
pub fn print_panic_gnss_diffs(dut: &Vec<Constellation>, model: &Vec<Constellation>) {
    for dut_gnss in dut.iter() {
        let mut found = false;
        for model_gnss in model.iter() {
            if model_gnss == dut_gnss {
                found = true;
                break;
            }
        }
        if !found {
            panic!("found unexpected constellation {} in dataset", dut_gnss);
        }
    }

    for model_gnss in model.iter() {
        let mut found = false;
        for dut_gnss in dut.iter() {
            if model_gnss == dut_gnss {
                found = true;
                break;
            }
        }
        if !found {
            panic!("constellation {} is missing from dataset", model_gnss);
        }
    }
}

/// Print the diff in terms of Observables (unique'd and sorted)
/// and eventually panics.
pub fn print_panic_observable_diffs(dut: &Vec<&Observable>, model: &Vec<&Observable>) {
    for dut_obs in dut.iter() {
        let mut found = false;
        for model_obs in model.iter() {
            if model_obs == dut_obs {
                found = true;
                break;
            }
        }
        if !found {
            panic!("found unexpected observable {} in dataset", dut_obs);
        }
    }

    for model_obs in model.iter() {
        let mut found = false;
        for dut_obs in dut.iter() {
            if model_obs == dut_obs {
                found = true;
                break;
            }
        }
        if !found {
            panic!("observable {} is missing from dataset", model_obs);
        }
    }
}
