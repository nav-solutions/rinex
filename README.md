RINEX 
=====

[![Rust](https://github.com/nav-solutions/rinex/actions/workflows/rust.yml/badge.svg)](https://github.com/nav-solutions/rinex/actions/workflows/rust.yml)
[![Rust](https://github.com/nav-solutions/rinex/actions/workflows/daily.yml/badge.svg)](https://github.com/nav-solutions/rinex/actions/workflows/daily.yml)
[![crates.io](https://docs.rs/rinex/badge.svg)](https://docs.rs/rinex/)
[![crates.io](https://img.shields.io/crates/d/rinex.svg)](https://crates.io/crates/rinex)
[![discord server](https://img.shields.io/discord/1342922474110586910?logo=discord)](https://discord.gg/EqhEBXBmJh)

[![MRSV](https://img.shields.io/badge/MSRV-1.89.0-orange?style=for-the-badge)](https://github.com/rust-lang/rust/releases/tag/1.89.0)
[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/nav-solutions/rinex/blob/main/LICENSE)

[RINEX (Receiver Independent EXchange)](https://en.wikipedia.org/wiki/RINEX) parser and formatter.   
The RINEX format is fully open source and is specified to answer the requirements of navigation and much more.

To contribute to either of our project or join our community, you way
- open an [Issue on Github.com](https://github.com/nav-solutions/rinex/issues) 
- follow our [Discussions on Github.com](https://github.com/nav-solutions/discussions)
- join our [Discord channel](https://discord.gg/EqhEBXBmJh)

## Advantages :rocket: 

- Fast and powerful parser
- Open sources: read and access all the code
- Seamless Gzip decompression (on `flate2` feature)
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
- Several conversion methods (SERDES operations):
  - to and from JSON with `serde`
  - RTCM protocol (on `rtcm` feature)
  - BINEX messages (with `binex` feature)
  - U-Blox (UBX) messages (with `ublox` feature)
  - to binary GNSS protocols (currently limited to GPS protocol) with `protos`

## Warnings :warning:

- Supported GLONASS FDMA, SBAS, and NavIC broadcast messages can produce native satellite states with the `nav` feature; full positioning and precise orbit accuracy are not established
- File production might lack some features, mostly because we're currently focused on data processing

## NAV parse diagnostics

`Rinex::parse`, `Rinex::from_file`, and `Rinex::from_gzip_file` keep valid NAV records before and after a record with an invalid nonblank field in a known ephemeris layout. They reject the entire invalid record. Inspect `rinex.nav_parse_report().diagnostics()` and `rejected_records()` after reading; each field diagnostic includes the physical file line, slot, field name, original field text, and reason, plus SV, epoch, and message when parseable. `unsupported_records()` counts messages or time pairs without a supported layout separately. Non-NAV results have an empty report. The report describes the original parse and is not written to RINEX output.

Use `parse_strict`, `from_file_strict`, or `from_gzip_file_strict` to return an error at the first rejected NAV record. I/O failures, lost record structure, and incomplete ephemeris rows fail in either mode. Explicit zero fields are retained; blank fields remain absent. These rules do not assert that every present broadcast field has been scientifically validated.

## GPS LNAV native state (`nav` feature)

`nav_select_gps_lnav(sv, t, UnknownHealthPolicy::Reject)` reports the candidate ephemerides retained in the `NavKey` record map for that satellite, including why a record was rejected. Call `chosen().and_then(|candidate| candidate.native_state_at(t).ok())` for the selected record's position in km, velocity in km/s, and satellite clock correction in seconds. The position and velocity use GPS broadcast WGS-84 axes; the NAV message does not name a WGS-84 realization. The clock omits signal group delay. No terrestrial-frame conversion or antenna correction is applied. Unknown health can be allowed explicitly; unhealthy records are always rejected. The 2-hour half-window around each of ToE and ToC is exclusive; it is a library selection policy, not a clock accuracy guarantee.

| Decoded NAV record | Selection and native position/velocity |
| --- | --- |
| RINEX 2/3/4 GPS LNAV without subtype | Supported; real V2, V3, V4 records are exercised in `tests/nav_gps_lnav.rs` |
| GPS CNAV and other messages or constellations | Reported as `UnsupportedMessage` by this selector; further propagators are separate work |

## GLONASS FDMA native state (`nav` feature)

`nav_select_glonass_fdma(sv, t, UnknownHealthPolicy::Reject)` reports candidate ephemerides retained in the `NavKey` record map for the satellite and selects a propagatable GLONASS FDMA or legacy LNAV record. `chosen().unwrap().fdma_state_at(t)` gives position (km), rotating-axis velocity (km/s), and satellite clock correction (s) in native broadcast PZ-90 axes. The record does not specify a PZ-90 realization. The clock uses `-TauN + GammaN * (t - Tb)` and excludes TauC and signal delays; the third clock slot is frame time, not quadratic drift. The integration window is `|t - Tb| < 900 s`, exclusive. This selector rejects other GLONASS message types, missing acceleration, unhealthy or invalid data, and candidates whose propagation fails. It does not convert to WGS-84 or ITRF.

Real V2/V3 legacy LNAV and V4 FDMA selection are exercised in `tests/nav_glonass_fdma.rs`. The V4 R01 reference states come from the independent script `tests/reference/glonass_fdma_r01.py`; source, equations, and numerical limits are recorded in `tests/reference/GLONASS_FDMA.md`. Run `cargo test --features nav --test nav_glonass_fdma` from the repository root. The numerical comparison verifies this implementation against the recorded integration algorithm, not precise orbit accuracy.

## Additional native NAV states (`nav` feature)

`nav_select_ephemeris(sv, t, UnknownHealthPolicy::Reject)` ranks records retained in the `NavKey` map only after checking message support, orbit and clock windows, health, required fields, and whether the selected propagator can produce a finite state at `t`. The result reports each retained candidate and its rejection reason. Records with the same `NavKey` overwrite earlier entries during parsing, so the report is not a count of every raw input block. Call `kepler_state_at(t)` for the Kepler families below, `sbas_state_at(t)` for SBAS, or `fdma_state_at(t)` for GLONASS. The states return native broadcast terrestrial axes, position in km, rotating-axis velocity in km/s, and satellite clock correction in seconds without signal group delay. They do not perform a terrestrial-frame conversion.

| NAV record | Native frame and state | Reference test |
| --- | --- | --- |
| GPS LNAV, RINEX 2/3/4 | WGS-84 broadcast axes, Kepler | `nav_gps_lnav`, `nav_legacy` |
| GLONASS LNAV (2/3), FDMA (4) | PZ-90 broadcast axes, RK4 | `nav_glonass_fdma` |
| SBAS LNAV (2/3), SBAS (4) | SBAS broadcast axes, position/velocity polynomial | `nav_sbas` (real V4) |
| QZSS LNAV (3/4) | JGS broadcast axes, Kepler | `nav_qzss_lnav` (real V4) |
| NavIC LNAV (3/4) | WGS-84 broadcast axes, Kepler; realization unknown | `nav_navic_lnav` (real V4) |
| Galileo INAV/FNAV (4) | GTRF broadcast axes, Kepler | `nav_legacy` |
| BeiDou D1 non-GEO, D2 GEO (4) | CGCS2000 broadcast axes, Kepler; GEO rotation for D2 | `nav_legacy`, `nav_bds_geo` |
| BeiDou V2/V3 LNAV, D1 GEO, D2 non-GEO, other messages | `UnsupportedMessage` | `nav_broadcast_selection`, `nav_bds_geo` |

The Kepler selectors apply each message's existing exclusive orbit half-window independently to ToE and ToC: GPS/NavIC 7200 s, QZSS 3600 s, Galileo 10800 s, and BeiDou 21600 s. For Galileo and BeiDou this is a library clock acceptance policy, not a separately validated clock fit interval. The other selectors use the windows documented in the matching `tests/reference/NAV_*.md` files. Those notes identify the real record, independent calculation, units, and limits. The window and numerical comparisons check the declared algorithm; they do not establish physical orbit accuracy or a particular frame realization. RINEX 4 Galileo INAV/FNAV parsing follows the spare slot after the week field, and SBAS writing uses its `t_tm` transmission-time field.

## GPS G2296 to ITRF position (`nav` feature)

For GPS LNAV in the supported 2024 catalogue window, `NavCandidate::spatial_state_at(t)` retains the NAV identity and native WGS-84 position. `state.to_frame(FrameRequest::Realization(FrameId::Itrf2014))` applies the dated G2296 → ITRF2020 → ITRF2014 position path. `FrameTransformer::to_frame` accepts a caller-asserted Earth-fixed point in km through `SpatialPoint::new`; the coordinate epoch is required. Results include the resolved source and target realizations, operation IDs, and provenance. Cross-frame velocity is unavailable, and an asserted strict position bound is rejected. Other constellations do not inherit the GPS G2296 realization. See [the G15 reference and limits](tests/reference/nav_frame_transform.md); run `cargo test --features nav --test nav_frame_transform` or `cargo run --features nav --example nav_frame -- tests/fixtures/nav_gps_g15_g2296_2024128.rnx G15 '2024-05-07T02:05:00 GPST'`.

Other ITRF2014 positions use narrowly scoped paths. Inspect `position_status()`, `source_realization`, `source_evidence`, ordered `edge_info`, and cautions alongside XYZ. `MarkedApproximation` has documented path evidence but no strict satellite-position bound; it withholds target-frame velocity and can be rejected with `warnings_as_errors`. Without a documented path, conversion returns the specific `FrameError`.

| NAV source and accepted period | ITRF2014 path | Limits and reference |
| --- | --- | --- |
| GLONASS FDMA/LNAV, 2014-01-15 to 2024-12-31 UTC | PZ-90.11 fixed-parameter approximation | [R01](tests/reference/nav_glonass_frame_approx.md); reverse operation is also marked approximate, with no strict bound |
| QZSS J02 LNAV, 2021-02-16 to 2023-11-08 UTC | JGS aligned-period zero-offset approximation | [J02](tests/reference/nav_qzss_frame_approx.md); no strict bound |
| Galileo INAV/FNAV, 2024-05 UTC | GTRF23v01 approximate alignment to ITRF2020, then dated ITRF2014 transform | [INAV](tests/reference/nav_galileo_frame_approx.md), [FNAV](tests/reference/nav_galileo_fnav_frame_approx.md); 2022 records remain unresolved |
| BeiDou C10 D1 IGSO, C20 D1 MEO, C05 D2 GEO, 2022-06 UTC | BDCS(2019v01) zero-offset approximation | [IGSO](tests/reference/nav_bds_igso_frame_approx.md), [MEO](tests/reference/nav_bds_meo_frame_approx.md), [GEO](tests/reference/nav_bds_geo_frame_approx.md); date applicability is inferred |
| NavIC I02 LNAV | No ITRF path; native WGS-84-family state remains available | [I02](tests/reference/nav_navic_nominal_frame.md); concrete source realization unknown |
| GAGAN S27 SBAS | No ITRF path; native SBAS state remains available | [S27](tests/reference/nav_sbas_s27_nominal_frame.md); concrete source realization unknown |

All ranges above are library restrictions, not guarantees of physical frame accuracy. Outside an evidenced path, conversion returns `Err(FrameError)` while the selected state retains native XYZ, source identity, and provenance. Rejected NAV records have no propagated state. `nav_frame` prints the native state before requesting a target frame and shows either the ordered path or the error.

The [frame request reference](tests/reference/nav_all_frame_targets.md) covers all eight `FrameId` labels and their evidenced dated paths, including reverse approximations for PZ-90.11, QZSS JGS, Galileo GTRF23v01 and BeiDou BDCS2019v01. `nav_frame --target` accepts `g2296`, `itrf2020`, `itrf2014`, `pz9011`, `jgs2014`, `jgs2020`, `gtrf23v01`, and `bdcs2019v01`; `wgs84` remains the generic request. Inspect ordered `edge_info` and every `CAUTION` before using target XYZ. An `Err` preserves the native state; it does not provide coordinates in the requested frame.

### Upstream API review and migration

These APIs changed during the NAV work and require maintainer compatibility review before release:

| Earlier use | Current path and limitation |
| --- | --- |
| `Ephemeris::kepler2position(sv, t)` returning ANISE `Orbit` | Select a matching NAV record and call `native_state_at`, `fdma_state_at`, `sbas_state_at`, or `kepler_state_at`. The returned broadcast XYZ is not an ANISE `Orbit`; `kepler2position_velocity` remains a raw, unlabeled calculation. |
| `Rinex::sv_orbit(sv, t)` | Use `nav_select_ephemeris(sv, t, policy)` and propagate the chosen candidate. Selection and supported message coverage differ from the old raw lookup; no equivalent `Orbit` return is provided. |
| `Rinex::nav_azimuth_elevation_range(sv, t, rx_orbit, almanac)` | There is no drop-in replacement. The caller must establish compatible terrestrial frames and epochs for satellite and receiver, then perform geometry with a suitable library. Unbounded approximate ITRF results are not verified ANISE frame inputs. |
| External `Rinex { ... }` struct literal | Use `Rinex::new(header, record)` and set public `comments` and `production` as needed. The added private parse-report field prevents external struct literal construction. |
| `Record::parse` on NAV input | A rejected known-layout NAV record now returns an error. Use `Rinex::parse` or file constructors for lenient reading with `nav_parse_report()` diagnostics; use their `*_strict` variants for fail-fast behavior. Direct `Record::parse` has no equivalent diagnostics accessor. |

These migrations do not restore the old ANISE `Orbit` semantics. `nav_ephemeris_selection` remains a raw ephemeris lookup. The frame converter also now returns `Err(FrameError)` where earlier revisions returned a copied, nominally relabeled XYZ; `PositionStatus::NominalAssumption` and its result metadata were removed. Frame applicability and warning status must be inspected before downstream geometry.

A caller can preserve a native diagnostic state after a frame error:

```rust
let native = chosen.spatial_state_at(t)?;
match native.state.to_frame(request) {
    Ok(result) => { /* inspect result.position_status() and result.cautions() */ }
    Err(reason) => {
        let native_xyz_km = native.state.position_km;
        let native_source = native.state.source();
        eprintln!("frame error: {reason}; native source: {native_source:?}; native XYZ: {native_xyz_km:?}");
    }
}
```

The G02 V4 test fixture `tests/fixtures/nav_gps_g02_lnav_2022159.rnx` (SHA-256 `8dd9cf1925d8cfa7c7d4fddb79509427986495e8b08d418aec2951934e354f62`) is an unchanged excerpt of `data/NAV/V4/KMS300DNK_R_20221591000_01H_MN.rnx.gz` at data submodule commit `209bfbd7016bd654f256238768a9e030ec5ab299` (MPL-2.0); the original compressed SHA-256 is `2bae4217cb71ad4a2b9c0067bd1c5b56915e42d2007a94e91eb408468cc4763f`. `tests/reference/nav_gps_lnav.py` independently reads the fixed RINEX slots and evaluates the broadcast equations, adapted from RTKLIB `eph2pos` at commit `180043ee24b6d2b168f98b64be15f69d50046b1a`. From the repository root, run `python3 tests/reference/nav_gps_lnav.py` to inspect its JSON on stdout, then `cargo test --features nav --test nav_gps_lnav`. The position, velocity, and clock tolerances in the test assess this algorithm against the independent calculation; they do not establish precise-orbit accuracy.

## Citation and referencing

If you need to reference this work, please use the following model:

`Nav-solutions (2026), RINEX: analysis and processing (MPLv2), https://github.com/nav-solutions`

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

Our `log` feature unlocks debug traces. Please avoid using the `trace` level, as it is dedicated to debugging our
file decompressor and is _very_ verbose. In a complex processing pipeline, you can adjust the verbosity for each
library, for example, this command line would define a default `trace` level, but increase that level to `debug` for the RINEX library
only:

```bash
export RUST_LOG=trace,rinex=debug
```

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

| Type                       | Parser            | Writer              |      Content                                  | Record Indexing                                                                  | Timescale  |
|----------------------------|-------------------|---------------------|-----------------------------------------------|----------------------------------------------------------------------------------| -----------|
| Navigation  (NAV)          | :heavy_check_mark:| :heavy_check_mark:  | Ephemerides, Ionosphere models                | [NavKey](https://docs.rs/rinex/latest/rinex/navigation/struct.NavKey.html)       | SV System time broadcasting this message |
| Observation (OBS)          | :heavy_check_mark:| :heavy_check_mark:  | Phase, Pseudo Range, Doppler, SSI             | [ObsKey](https://docs.rs/rinex/latest/rinex/observation/struct.ObsKey.html)      | GNSS (any) |
|  CRINEX  (Compressed OBS)  | :heavy_check_mark:| :heavy_check_mark:  | Phase, Pseudo Range, Doppler, SSI             | [ObsKey](https://docs.rs/rinex/latest/rinex/observation/struct.ObsKey.html)      | GNSS (any) |
|  Meteorological data (MET) | :heavy_check_mark:| :heavy_check_mark:  | Meteo sensors data (Temperature, Moisture..)  | [MeteoKey](https://docs.rs/rinex/latest/rinex/meteo/struct.MeteoKey.html)        | UTC | 
|  Clocks (CLK)              | :heavy_check_mark:| :construction:      | Precise temporal states                       | [ClockKey](https://docs.rs/rinex/latest/rinex/clock/record/struct.ClockKey.html) | GNSS (any) |
|  Antenna (ATX)             | :heavy_check_mark:| :construction:      | Precise RX/SV Antenna calibration | `antex::Antenna` | :heavy_minus_sign: |
|  Ionosphere Maps  (IONEX)  | [Moved to dedicated parser](https://github.com/nav-solutions/ionex) |  :heavy_check_mark:     | Ionosphere Electron density | [Record Key](https://docs.rs/ionex/latest/ionex/key/struct.Key.html) | UTC |
|  DORIS RINEX               | [Moved to dedicated parser](https://github.com/nav-solutions/doris) |  :heavy_check_mark:     | Temperature, Moisture, Pseudo Range and Phase observations | [Record Key](https://docs.rs/doris-rs/latest/doris_rs/record/struct.Key.html) | TAI / "DORIS" timescale |

Contributions
=============

Contributions are welcomed, we still have a lot to accomplish, any help is always appreciated.   
[We wrote these few lines](CONTRIBUTING.md) to help you understand the inner workings.    
Join us on [Discord](https://discord.gg/EqhEBXBmJh) to discuss ongoing and future developments.
