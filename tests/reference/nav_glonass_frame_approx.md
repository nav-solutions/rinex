# GLONASS R01: marked PZ-90.11 → ITRF2014 position approximation

This single directed edge uses published PZ-90.11 → ITRF2014 parameters at
**2010.0**, held fixed at the input coordinate epoch. It is an
`UnboundedApproximate` position, with a visible `CAUTION` and no target-frame
velocity. The conversion does not certify satellite orbit or frame accuracy.

## Sources and applicability

- The [ICG 2023 GNSS frame table](https://www.unoosa.org/documents/pdf/icg/2023/ICG-17/icg17_wgd_02_03.pdf)
  identifies GLONASS broadcast axes as PZ-90.11. The [ICG 2016 GLONASS
  report](https://www.unoosa.org/pdf/icg/2016/icg11/wgd/13wgd.pdf) gives
  January 15, 2014 as the introduction for navigation support. The library
  conservatively resolves a NAV record to PZ-90.11 only when its record,
  orbit-reference and evaluation instants are in 2014-01-15 through
  2024-12-31 UTC. The end date is library policy, not a published frame end.
- The [ICG PZ-90.11 parameter table](https://www.unoosa.org/documents/pdf/icg/2019/resources/PZ-90.11_v.1.2_04.11.2018.pdf)
  lists the directed PZ-90.11 → ITRF2014 row at 2010.0: translations
  `[-0.0053, -0.0040, -0.0032]` m, coordinate-frame rotations
  `[+0.035, -0.087, +0.036]` mas, and scale zero to the displayed precision.
  With `r` in radians, the implemented convention is
  `X′ = X + rz·Y − ry·Z + dx`,
  `Y′ = Y − rz·X + rx·Z + dy`,
  `Z′ = Z + ry·X − rx·Y + dz`.
  No rates or strict later-epoch/satellite-position error bound are supplied.
  A later forward path through ITRF2020 and WGS84 G2296 is documented in
  `nav_glonass_r02_mixed_frame.md`; it retains this first edge's CAUTION.
- A direct `SpatialPoint` must assert `Realization(Pz90_11)`; the bare
  `GlonassBroadcastPz90` family remains unresolved. `numerical_only`,
  `warnings_as_errors`, a requested position error bound, and target velocity
  each reject the approximate edge. These controls do not change GPS's
  existing numerical paths.

## Traceable real input and independent expectation

Five R01 record lines plus header come from the fixed `data` submodule's
`NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz` (SHA-256
`e4a1448062cbd27c01c1d4d05908836874deab1b7a2297f8bc6f0f8243c738c4`).
The reduced `tests/fixtures/nav_glonass_r01_pz9011_2020176.rnx` has SHA-256
`b456dd8d83756a5b6108495c5a7b334d599edcbadd9ef944daa2b9cd6caaec52`.
R01 was broadcast at 2020-06-24 23:45:00 UTC and is evaluated 300 seconds
later. `nav_glonass_frame_approx.py` reads the fixed-width slots, independently
propagates the GLONASS J2 state with 60-second RK4 steps, and applies the
published frame row. It does not call Rust. Frozen output is
`nav_glonass_frame_approx_expected.json`: native XYZ
`[14228.774942571004, 2534.385126965379, 21029.483788965554]` km;
approximate target XYZ
`[14228.77494658332, 2534.3851240503727, 21029.483779333983]` km.
The numerical shift is about 10.8 mm, which is not a physical accuracy claim.

From the repository root:

```sh
python3 tests/reference/nav_glonass_frame_approx.py > /tmp/nav_glonass_frame_current.json
diff -u tests/reference/nav_glonass_frame_approx_expected.json /tmp/nav_glonass_frame_current.json
cargo test --features nav --test nav_glonass_frame
cargo run --features nav --example nav_frame -- tests/fixtures/nav_glonass_r01_pz9011_2020176.rnx R01 '2020-06-24T23:50:00 UTC'
```

The example prints `UnboundedApproximate`, the chosen edge, `CAUTION`, and
`velocity_km_s=None`. For VS Code, use **Debug Test** on
`real_r01_produces_marked_itrf2014_approximation` in
`tests/nav_glonass_frame.rs`; inspect `realization` in
`NavCandidate::spatial_state_at`, `source_id` in `FrameTransformer::to_frame`,
and `rx`, `ry`, `rz` in `pz9011_to_itrf2014_approx`.
