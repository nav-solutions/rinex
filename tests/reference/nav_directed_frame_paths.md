# Directed frame paths and explicit errors

`FrameTransformer::to_frame` returns `Ok(FrameResult)` only for native identity or a dated, catalogued path. A selected NAV record with finite native XYZ can still return `Err(FrameError)` for a requested frame. `UnknownSourceRealization`, `UnsupportedSource`, `OutsideCatalogWindow`, and `NoPath` remain distinct. The caller may keep `native.state.position_km` for diagnostics alongside `native.state.source()` and the error; those coordinates retain the native frame.

The [DLR NAV microfixture](mixed_nav_first_epoch.md) exercises C02 (unknown 2024 BDCS realization), I03 (no supported NavIC relation), E03 (dated GTRF23v01 path and an older JGS window rejection), and G04 health/data gates. E03 to ITRF2020 uses one marked approximate edge; `NumericalOnly`, `warnings_as_errors`, a strict frame-operation bound, and target velocity requests each reject for their own reason. Caller-asserted points keep the `SourceBasis::CallerAsserted` caution on evidenced paths. No unknown source gains a concrete target through a copied XYZ.

```sh
cargo test --offline --features nav --test nav_directed_frame_paths
cargo run --offline --features nav --example nav_frame -- tests/fixtures/nav_mixed_2024131_first_epoch.rnx E03 '2024-05-10T03:00:00 GPST' --target itrf2020
cargo run --offline --features nav --example nav_frame -- tests/fixtures/nav_mixed_2024131_first_epoch.rnx C02 '2024-05-10T03:00:00 GPST' --target itrf2020
```

Use **Debug Test** on `missing_evidence_returns_error_without_relabeling_native_xyz` in `tests/nav_directed_frame_paths.rs`. Break at `NavCandidate::spatial_state_at` and `FrameTransformer::to_frame_inner`; inspect `native.state.position_km`, `point.realization`, `source_id`, and the `Err` variant.
