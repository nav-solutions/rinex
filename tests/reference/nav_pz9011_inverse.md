# ITRF2014 → PZ-90.11 inverse operation

The installed directed edge is the mathematical inverse of the library's
PZ-90.11 → ITRF2014 **linearized** operation. It uses the published 2010.0
translation and coordinate-frame rotation row, frozen at the coordinate epoch.
The [ICG parameter table](https://www.unoosa.org/documents/pdf/icg/2019/resources/PZ-90.11_v.1.2_04.11.2018.pdf)
publishes the forward row; the inverse is derived from it here, not a separate
published satellite transformation. The library applies both directions only
from 2014-01-15 through 2024-12-31 UTC, its conservative approximation window.
The result is `MarkedApproximation`, without a strict satellite frame-operation
error bound or cross-frame velocity. The current catalogue and other target
paths are described in [`nav_all_frame_targets.md`](nav_all_frame_targets.md).

For input metres `y`, translation `t`, and the coordinate-frame rotation vector
`r`, the forward linearized operation is `y = (I - cross(r)) x + t`.
The reverse implementation evaluates
`x = (I + cross(r) + r rᵀ) (y - t) / (1 + rᵀr)`.
All rotation parameters are converted from milliarcseconds to radians, and
coordinates from kilometres to metres before applying the operation.

`nav_pz9011_inverse.py` independently solves the three forward equations by
Gaussian elimination. Its input is the frozen ITRF2014 XYZ from the real R01
RINEX reference in `nav_glonass_frame_approx_expected.json`; the source fixture
SHA-256 is `b456dd8d83756a5b6108495c5a7b334d599edcbadd9ef944daa2b9cd6caaec52`.
The recovered PZ XYZ is
`[14228.774942571004, 2534.385126965379, 21029.483788965554]` km,
within `1e-9` km per axis of the independently propagated native R01 reference.
This agreement checks operation arithmetic and direction, not physical frame
accuracy at the satellite.

From the repository root:

```sh
python3 tests/reference/nav_pz9011_inverse.py
cargo test --offline --features nav --test nav_pz9011_inverse --test nav_glonass_frame --test nav_directed_frame_paths
cargo run --offline --features nav --example nav_frame -- tests/fixtures/nav_mixed_2024131_first_epoch.rnx E03 '2024-05-10T03:00:00 GPST' --target pz9011
```

The CLI selects E03 INAV and prints the ordered path GTRF23v01 → ITRF2020 →
ITRF2014 → PZ-90.11, `MarkedApproximation`, and a caution for both approximate
edges. It does not create a result for rejected NAV records. To inspect the
inverse in a debugger, enter
`inverse_of_published_r01_reference_recovers_independent_native_xyz` in
`tests/nav_pz9011_inverse.rs`. Break at `FrameTransformer::to_frame_inner` near
`find_catalog_path`, then at `itrf2014_to_pz9011_approx`. Inspect
`point.position_km`, `path`, `r`, `u`, `cross`, and `result.position_km`.
At 2025-01-01, the caller-asserted point receives `OutsideCatalogWindow`.
