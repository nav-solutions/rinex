# GAGAN S27 native state and frame rejection

The [RINEX 4.02 specification](https://files.igs.org/pub/data/format/rinex_4.02.pdf) maps S27 to SBAS PRN127. The [Airports Authority of India service table](https://aim-india.aai.aero/eAIP_Archive/19-05-2022/eAIP/IN-ENR%204.3-en-GB.html) identifies it as GAGAN GSAT-8. These sources do not give a concrete realization or dated ITRF2014 relation for the retained March 2023 record. `spatial_state_at` therefore exposes its native polynomial position and velocity, while target frame requests return `UnsupportedSource(SbasBroadcast)`.

The real [S27 fixture](../fixtures/nav_sbas_s27_2023071.rnx), source, and [independent Python polynomial](nav_sbas_s27.py) are described in [NAV_SBAS.md](NAV_SBAS.md). Five frozen rows cover ToC ±300 s; at +120 s native XYZ is `[24160.29789, 34538.69603, 30.32833]` km. The test compares the native six-vector and verifies the frame error. This does not establish physical alignment or precise satellite-position accuracy.

```sh
cargo test --offline --features nav --test nav_sbas_nominal_frame
cargo run --offline --features nav --example nav_frame -- tests/fixtures/nav_sbas_s27_2023071.rnx S27 '2023-03-12T01:17:44 GPST' --target itrf2014
```

In VS Code, **Debug Test** `s27_native_reference_is_preserved_and_unverified_frame_is_rejected`; inspect `native.state.position_km`, `native.state.velocity_km_s`, `native.state.source()`, and the `FrameError`.
