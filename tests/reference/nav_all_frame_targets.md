# Dated frame requests

The catalogue accepts eight concrete `FrameId` requests. Availability depends on source realization, epoch, edge direction, and target. The tests cover four dated reverse zero-offset approximations, their window boundaries, and the E03 NAV sample. `Err` is a frame conversion rejection and must not be counted as a physical target result.

| Reverse edge | Tested interval | Method |
| --- | --- | --- |
| ITRF2014 → QZSS JGS/ITRF2014 | 2021–2023 documented period | Marked approximation |
| ITRF2020 → QZSS JGS/ITRF2020 | 2024 catalogue period | Marked approximation |
| ITRF2020 → Galileo GTRF23v01 | 2024-05 catalogue period | Marked approximation |
| ITRF2014 → BeiDou BDCS2019v01 | 2022-06 catalogue period | Marked approximation |

The zero numerical offsets express limited alignment evidence; they do not establish a strict satellite-position bound. A native identity has no frame-operation offset. A cross-frame result exposes ordered `edge_ids`, `edge_info`, source realization, method, and cautions. The 2024 E03 sample can reach several concrete targets through dated paths, while JGS/ITRF2014 and BDCS2019v01 requests fail with `OutsideCatalogWindow`.

```sh
cargo test --offline --features nav --test nav_all_frame_targets
cargo run --offline --features nav --example nav_frame -- tests/fixtures/nav_mixed_2024131_first_epoch.rnx E03 '2024-05-10T03:00:00 GPST' --target jgs2020
```

In VS Code, **Debug Test** `representative_nav_requests_cover_evidenced_paths_and_explain_missing_ones`; inspect `target`, `point.realization`, `edge_info`, and `FrameError` at `find_catalog_path`.
