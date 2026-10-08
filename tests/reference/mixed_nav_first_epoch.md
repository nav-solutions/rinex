# DLR NAV microfixture at 2024-05-10 03:00:00 GPST

The 108-line [`nav_mixed_2024131_first_epoch.rnx`](../fixtures/nav_mixed_2024131_first_epoch.rnx) retains the original DLR RINEX 4 header and 11 byte-identical EPH blocks. The [NAV-only source manifest](../fixtures/nav_mixed_2024131_source.json) records the BRD400DLR product DOI, public BKG archive, original NAV SHA-256, original one-based line spans and block hashes, and the microfixture SHA-256. The unchanged header includes `LEAP SECONDS` and the DLR DOI. Selection uses G04 competing LNAV/CNAV records, R02 FDMA, E03 INAV, C02 D2, I03 LNAV, and J04 LNAV/CNAV. This is a deliberately small behavior sample, not a completeness claim about the daily product.

The source is [DLR/GSOC's BRD400DLR product](https://igs.org/mgex/mgex-product-descriptions/), credited to O. Montenbruck and P. Steigenberger. [IGS MGEX](https://www.igs.org/mgex/) states that its data and products are freely available for public use and requests citation when used in a publication. The repository preserves the source header and identifies the provider and DOI. The tests check selected record identity and numerical algorithms, not physical satellite-position accuracy.

From the repository root:

```sh
cargo test --offline --features nav --test nav_mixed_first_epoch
cargo run --offline --features nav --example nav_frame -- tests/fixtures/nav_mixed_2024131_first_epoch.rnx G04 '2024-05-10T03:00:00 GPST' --target itrf2014
python3 tests/support/extract_mixed_nav_first_epoch.py /path/to/BRD400DLR_S_20241310000_01D_MN.rnx
```

The last command requires the full DLR NAV source with the manifest's exact hash and rebuilds the tracked microfixture from its original bytes. For a no-path example, change `G04` to `C02` and the target to `itrf2020`: `nav_frame` prints native XYZ with the source label, then `frame_error=UnknownSourceRealization` and exits nonzero. The native XYZ must not be read as ITRF2020 XYZ.

In VS Code, use **Debug Test** on `representative_real_records_keep_selection_propagation_and_frame_errors_distinct` in `tests/nav_mixed_first_epoch.rs`. Break at `NavCandidate::spatial_state_at` and `FrameTransformer::to_frame_inner`; inspect `chosen.key`, `native.state.position_km`, `point.realization`, and the returned `FrameError`. VS Code breakpoint operation remains for the user to verify.
