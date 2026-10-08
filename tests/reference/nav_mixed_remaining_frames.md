# E03, J04, C02, and I03 in the DLR NAV sample

The [NAV-only source manifest](../fixtures/nav_mixed_2024131_source.json) maps every retained record to its original DLR line and byte hash. The 2024-05-10 03:00:00 GPST query selects E03 INAV at original source line 870. [`nav_galileo_e03_mixed_frame.py`](nav_galileo_e03_mixed_frame.py) independently reads that raw record and evaluates its Kepler position, yielding `[13527.734303005513, 20608.988501740743, -16393.79374112982]` km. The GTRF23v01 → ITRF2020 → G2296 path is marked approximate; the zero-offset first edge does not establish physical satellite-position accuracy.

J04 has no selected record because its retained LNAV is unhealthy and its CNAV is unsupported by this propagator. C02 D2 propagates in native BeiDou axes, but its 2024 concrete BDCS realization is unestablished. I03 LNAV propagates in native NavIC WGS-84-family axes, without an evidenced target relation. They return `UnknownSourceRealization` and `UnsupportedSource(NavicBroadcastWgs84)` respectively when a target frame is requested. Their native XYZ remains available for diagnostics. The dated JGS2020 path is independently tested with an asserted point; it does not turn J04 into a selectable broadcast record.

```sh
python3 tests/reference/nav_galileo_e03_mixed_frame.py
cargo test --offline --features nav --test nav_mixed_remaining_frames --test nav_mixed_first_epoch
cargo run --offline --features nav --example nav_frame -- tests/fixtures/nav_mixed_2024131_first_epoch.rnx E03 '2024-05-10T03:00:00 GPST' --target wgs84
```

Use **Debug Test** on `e03_reaches_g2296_with_independent_raw_orbit_reference` in `tests/nav_mixed_remaining_frames.rs`; inspect `native.state.position_km`, `point.realization`, `edge_ids`, and `result.position_status()`.
