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

## Warnings :warning:

- Navigation is currently not feasible with Glonass, SBAS and IRNSS
- File production might lack some features, mostly because we're currently focused on data processing

## Citation and referencing

If you need to reference this work, please use the following model:

`Nav-solutions (2025), RINEX: analysis and processing (MPLv2), https://github.com/nav-solutions`

## Library features

All RINEX formats [described in the following table](#RINEX_formats_&_applications) are supported natively, we do not
hide a specific format under compilation options. The parser is smart enough to adapt to the file
revision, you don't need specific options to work with RINEX V2 or RINEX V4, and you may work with both at the same
time conveniently.

We offer one compilation option
per format, to provide more detail and "enhance" the capabilities for that format.
For example:
- `obs` relates to the Observation RINEX format and provides
special iterators and processing feature for this file format.
- `nav`: is the heaviest amongst all options, because it relies on heavy external libraries like `nalgebra` 
and `anise`. 

Note that this library requires std library at all times, it is not planed to make it no-std compatible.

We offer many serialization (and deserialization) options:

- `serde` for standard serdes, usually to JSON
- `ublox`: to serialize RINEX structures to UBX messages
and construct RINEX structures from UBX messages
- `binex`: same thing for BINEX protocol
- `rtcm`: same thing for RTCM protocol
- `gnss-protos`: to serialize RINEX structures to raw GNSS navigation messages,
for example GPS messages, or collecting a RINEX structure from GPS messages.
For example, this could be the high level entrypoint to a GNSS simulator.

`GNSS-QC` (Quality check) relates to complex geodesic processing workflows, usually starting
from RINEX files, by means of this library. To support demanding `GNSS-QC` operations, we provide two options:

- The `qc` option is the entry point, it provides means to manipulate thos files.
For example, merging two files into one.
- The `processing` features builds on top `qc` and is expected to provide all requirements
to complex GNSS-QC workflows.

Formats & revisions
===================

The parser supports RINEX V4.0, that includes RINEX V4 Navigation files.   
All revisions are supported by default and without compilation options: the parser automatically adapts.

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
