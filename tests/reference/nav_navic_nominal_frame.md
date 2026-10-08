# NavIC I02 native state and frame rejection

The [ISRO NavIC SPS ICD 1.1](https://www.isro.gov.in/media_isro/pdf/Missions/irnss_sps_icd_version1.1-2017.pdf) supplies WGS-84-family orbit equations but does not identify the concrete WGS-84 realization or a dated ITRF2014 relation for this broadcast record. `NavCandidate::spatial_state_at` returns native position and velocity. `to_frame(Wgs84)` and `to_frame(Itrf2014)` return `UnsupportedSource(NavicBroadcastWgs84)` while the original native state remains available. The library represents IRNSST with a GPST proxy; the physical time-scale offset is not validated here.

The real [I02 fixture](../fixtures/nav_navic_i02_2023071.rnx) and [independent Python reference](nav_navic_lnav.py) are described in [NAV_NAVIC_LNAV.md](NAV_NAVIC_LNAV.md). At ToE its native XYZ is `[20972.353636599695, 34616.32508750093, -12067.52585104587]` km. The test compares three native epochs with the frozen JSON reference and checks the frame errors; it does not validate physical orbit or cross-frame accuracy.

```sh
cargo test --offline --features nav --test nav_navic_nominal_frame
cargo run --offline --features nav --example nav_frame -- tests/fixtures/nav_navic_i02_2023071.rnx I02 '2023-03-12T00:00:00 GPST' --target itrf2014
```

In VS Code, **Debug Test** `i02_native_reference_is_preserved_and_unverified_frame_is_rejected`; inspect `native.state.position_km`, `native.state.source()`, and the `FrameError` returned by `to_frame_inner`.
