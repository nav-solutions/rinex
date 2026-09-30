# F2 GLONASS source unit: R02 at 2024-05-10 03:00 GPST

This unit extends the existing directed PZ-90.11 → ITRF2014 approximate edge through the already installed numerical ITRF2014 → ITRF2020 and ITRF2020 → WGS84 G2296 edges. It changes positions only. The NAV record, source realization, native position, and each edge remain visible. It does not claim a physical satellite-position accuracy bound.

## Evidence and applicability

- The original mixed NAV has SHA-256 `9cffb1b1f2978c46e8352d78f603dd089ecd7f32312c1a6c4ae585b5d0e538b6`; the byte-preserving NAV microfixture has SHA-256 `dce340cf859c06b7785596f379f1bbf39f8042042629309adc9fbdc2f322c69e`. See `tests/fixtures/nav_mixed_2024131_source.json` for the retained source line spans. Selection chooses R02 FDMA ToC/ToE 2024-05-10 02:45:00 UTC. The query 03:00:00 GPST is 02:59:42 UTC using the retained NAV header's `LEAP SECONDS` value of 18 and the GPST/UTC relation, so propagation is +882 seconds, inside the exclusive 900-second selector window.
- The [Russian PZ-90.11 reference table hosted by UNOOSA](https://www.unoosa.org/documents/pdf/icg/2019/resources/PZ-90.11_v.1.2_04.11.2018.pdf), page 2, gives the PZ-90.11 → ITRF2014 2010.0 row: translation `[-0.0053, -0.0040, -0.0032]` m; coordinate-frame rotation `[+0.035, -0.087, +0.036]` mas; scale zero to the published precision. Existing `nav_glonass_frame_approx.md` documents the parameter convention and independent R01 check. The official PDF direct URL returned 404 during this implementation; its indexed official excerpt agrees with the existing repository record. The direct source still needs rechecking when available.
- PZ-90.11 NAV source assignment requires record time, orbit reference, and evaluation time within the library's 2014-01-15 through 2024-12-31 UTC window. This is a library restriction; the source material identifies the GLONASS PZ-90.11 era but does not certify a 2024 satellite-position transform from the 2010.0 station fit. The first edge freezes that row at 2024 and remains `UnboundedApproximate`.
- [ITRF2020 Table 2](https://itrf.ign.fr/en/solutions/itrf2020) states ITRF2014 minus ITRF2020 at 2015.0: translation `[-1.4, -0.9, +1.4]` mm, rates `[0.0, -0.1, +0.2]` mm/Julian year, scale `-0.42` ppb, zero published rotations and rotation rates. At the point epoch, the inverse operation is `p_2020 = (p_2014 - T(t)) / (1+s)`. Its library window is 2015-01-01 through 2026-12-31 UTC.
- [EPSG:10608](https://epsg.io/10608) publishes seven zero parameters for WGS84 G2296 → ITRF2020. The inverse ITRF2020 → G2296 therefore copies the Cartesian XYZ numerically in the library's 2024-03-04 through 2024-12-31 UTC window. EPSG's 0.01 m operation accuracy at epoch 2024.0 and ITRF parameter uncertainties are not strict satellite-position upper bounds.

For the PZ edge, with coordinates converted from km to m and rotations converted from mas to radians, the coordinate-frame convention is `X' = X + rz·Y − ry·Z + dx`, `Y' = Y − rz·X + rx·Z + dy`, `Z' = Z + ry·X − rx·Y + dz`. The result converts back to km once. The complete path's status is `MarkedApproximation`, including when its final edge is numerical. `NumericalOnly`, `warnings_as_errors`, `max_frame_operation_error_m`, and `require_velocity` each reject this path for their respective reasons. Cross-frame velocity stays absent.

The independent Python reference reads the R02 fields, integrates the GLONASS J2 model with RK4 steps up to 60 seconds, and applies the published parameters without calling Rust. At the query epoch it yields native XYZ `[-13288.263885025124, 8043.228789681527, -20203.634130303984]` km and approximate G2296 XYZ `[-13288.263901624035, 8043.228789786232, -20203.634141020608]` km. The numerical test tolerance is `1e-9` km per transformed component; it checks arithmetic and sign, not physical frame accuracy.

The reduced NAV sample retains only the records needed for the focused selection, native propagation, and frame-path checks. No all-satellite result count is inferred from it.

## Run and debug

From the repository root:

```sh
python3 tests/reference/nav_glonass_r02_mixed_frame.py > /tmp/nav_glonass_r02_current.json
diff -u tests/reference/nav_glonass_r02_mixed_frame_expected.json /tmp/nav_glonass_r02_current.json
cargo test --offline --features nav --test nav_glonass_r02_mixed_frame --test nav_glonass_frame --test nav_mixed_first_epoch
cargo run --offline --features nav --example nav_frame -- \
  tests/fixtures/nav_mixed_2024131_first_epoch.rnx R02 \
  '2024-05-10T03:00:00 GPST' --target wgs84
```

In VS Code, use **Debug Test** on `r02_reaches_g2296_with_three_ordered_edges_and_weakest_status` in `tests/nav_glonass_r02_mixed_frame.rs`. Break at `FrameTransformer::to_frame_inner` where `source_id` and `target_id` are resolved, then at `pz9011_to_itrf2014_approx` and `itrf2014_to_2020`. Inspect `point.position_km`, `itrf2014_km`, `position_km`, `edge_ids`, and `result.position_status()`. Terminal checks cannot establish VS Code breakpoint availability; that remains for the user to confirm.
