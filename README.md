RINEX 
=====

[![Rust](https://github.com/nav-solutions/rinex/actions/workflows/rust.yml/badge.svg)](https://github.com/nav-solutions/rinex/actions/workflows/rust.yml)
[![Rust](https://github.com/nav-solutions/rinex/actions/workflows/daily.yml/badge.svg)](https://github.com/nav-solutions/rinex/actions/workflows/daily.yml)
[![crates.io](https://docs.rs/rinex/badge.svg)](https://docs.rs/rinex/)
[![crates.io](https://img.shields.io/crates/d/rinex.svg)](https://crates.io/crates/rinex)

[![MRSV](https://img.shields.io/badge/MSRV-1.83.0-orange?style=for-the-badge)](https://github.com/rust-lang/rust/releases/tag/1.83.0)
[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/nav-solutions/rinex/blob/main/LICENSE)

[RINEX (Receiver Independent EXchange)](https://en.wikipedia.org/wiki/RINEX) parser and formatter.   
The RINEX format is fully open source and is specified to answer the requirements of navigation and much more.

To contribute to either of our project or join our community, you way
- open an [Issue on Github.com](https://github.com/nav-solutions/rinex/issues) 
- follow our [Discussions on Github.com](https://github.com/nav-solutions/discussions)
- join our [Discord channel](https://discord.gg/EqhEBXBmJh)

## Advantages :rocket: 

- Fast
- Open sources: read and access all the code
- All modern GNSS constellations, codes and signals
  - GPS, Galileo, BeiDou and QZSS
- Time scales: GPST, QZSST, BDT, GST, UTC, TAI
- Efficient seamless compression and decompression
  - modern rewrite of the Hatanaka compression algorithm
- RINEX V4 full support, including
  - new Ionospheric coorections
  - new Time offset corrections
  - precise Earth Orientation updates
- Supports Observation, Navigation, Meteo and Clock RINEX,
other RINEX-like formats have their own parser:
  - [IONEX (Ionosphere Maps)](https://github.com/nav-solutions/ionex)
  - [DORIS (special observations)](https://github.com/nav-solutions/doris)
- Many pre-processing algorithms including Filter Designer
- Several file operations: merging, splitting, time binning (batch)
- Several conversion methods (serdes operations)
  - BINEX protocol (on `binex` feature)
  - U-Blox protocol (on `ublox` feature)
  - RTCM protocol (on `rtcm` feature)

## Warnings :warning:

- Navigation is currently not feasible with Glonass, SBAS and IRNSS
- File production might lack some features, mostly because we're currently focused on data processing

## Citation and referencing

If you need to reference this work, please use the following model:

`Nav-solutions (2025), RINEX: analysis and processing (MPLv2), https://github.com/nav-solutions`

Formats & revisions
===================

The parser supports RINEX V4.0, that includes RINEX V4 Navigation files.   
We support the latest revisions for both IONEX and Clock RINEX.  
We support the latest (rev D) SP3 format.  

RINEX formats & applications
============================

| Type                       | Parser            | Writer              |  CLI                 |      Content         | Record Iteration     | Timescale  |
|----------------------------|-------------------|---------------------|----------------------|----------------------|----------------------| -----------|
| Navigation  (NAV)          | :heavy_check_mark:| :construction:      |  :heavy_check_mark: :chart_with_upwards_trend:  | Ephemerides, Ionosphere models | Epoch | SV System time broadcasting this message |
| Observation (OBS)          | :heavy_check_mark:| :heavy_check_mark: | :heavy_check_mark:  :chart_with_upwards_trend: | Phase, Pseudo Range, Doppler, SSI | Epoch | GNSS (any) |
|  CRINEX  (Compressed OBS)  | :heavy_check_mark:| RNX2CRX1 :heavy_check_mark: RNX2CRX3 :construction:  | :heavy_check_mark:  :chart_with_upwards_trend:  |  Phase, Pseudo Range, Doppler, SSI | Epoch | GNSS (any) |
|  Meteorological data (MET) | :heavy_check_mark:| :heavy_check_mark:  | :heavy_check_mark: :chart_with_upwards_trend:  | Meteo sensors data (Temperature, Moisture..) | Epoch | UTC | 
|  Clocks (CLK)              | :heavy_check_mark:| :construction:      | :heavy_check_mark: :chart_with_upwards_trend:  | Precise SV and Reference Clock states |  Epoch | GNSS (any) |
|  Antenna (ATX)             | :heavy_check_mark:| :construction:      | :construction:   | Precise RX/SV Antenna calibration | `antex::Antenna` | :heavy_minus_sign: |
|  Ionosphere Maps  (IONEX)  | :heavy_check_mark:|  :construction:     | :heavy_check_mark:  :chart_with_upwards_trend: | Ionosphere Electron density | Epoch | UTC |
|  DORIS RINEX               | :heavy_check_mark:|  :construction:     | :heavy_check_mark:   | Temperature, Moisture, Pseudo Range and Phase observations | Epoch | TAI |

Contributions
=============

Contributions are welcomed, do not hesitate to open new issues
and submit Pull Requests through Github.

If you want to take part in active developments, check out our [contribution guidelines and hints](CONTRIBUTING.md) to navigate this library quicker.
