//! integrated tests
pub mod toolkit;

mod antex;
mod compression;
mod crinex;
mod filename;
pub mod formatting;
mod parsing;

#[cfg(all(feature = "flate2", feature = "qc"))]
mod sbas;

#[cfg(feature = "flate2")]
mod production;

#[cfg(feature = "qc")]
mod merge;

#[cfg(feature = "clock")]
mod clock;

#[cfg(feature = "binex")]
mod binex;

#[cfg(feature = "processing")]
mod processing;

#[cfg(feature = "meteo")]
mod meteo;

#[cfg(feature = "nav")]
mod nav;

#[cfg(feature = "rtcm")]
mod rtcm;

#[cfg(feature = "ublox")]
mod ublox;

#[cfg(all(feature = "flate2", feature = "nav"))]
mod kepler;

mod obs;

#[cfg(feature = "log")]
use log::LevelFilter;

#[cfg(feature = "log")]
use std::sync::Once;

#[cfg(feature = "log")]
static INIT: Once = Once::new();

#[cfg(feature = "log")]
pub fn init_logger() {
    INIT.call_once(|| {
        env_logger::builder()
            .is_test(true)
            .filter_level(LevelFilter::Debug)
            .init();
    });
}
