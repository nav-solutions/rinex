use std::{collections::HashMap, str::FromStr};

use crate::{
    antex::{Antenna, AntennaSpecific, Calibration, CalibrationMethod, RxAntenna, SvAntenna},
    linspace::Linspace,
    parse_f64,
    prelude::{Carrier, Epoch, ParsingError, COSPAR, SV},
};
