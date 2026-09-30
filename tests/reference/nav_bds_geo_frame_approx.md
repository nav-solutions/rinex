# N09d BeiDou-2 C05 D2 GEO BDCS(2019v01) → ITRF2014 marked position path

This unit uses the real C05 D2 GEO record in
`tests/fixtures/nav_bds_geo_c05_2022159.rnx` at 2022-06-08 09:00:00 BDT
(ToE 291600 s). The existing `NativeFrame::Cgcs2000` is a broad
BeiDou frame/ellipsoid label. The specific source is `BeidouBroadcast`
with `Realization(Bdcs2019v01)`; the broad label alone cannot establish
a concrete operational realization.

## Source identity and conversion limit

The [IGS MGEX constellation table](https://www.igs.org/mgex/constellations/)
identifies PRN C05 as BDS-2 GEO-5 in this period. The
[CSNO Open Service Performance Standard](https://www.beidou.gov.cn/xt/gfxz/201812/P020181227529449178798.pdf)
explicitly assigns D2 to BDS-2 GEO B1I/B3I and defines broadcast
positions in BDCS, aligned to ITRF with annual updates. The
[IGS 2022 workshop material](https://files.igs.org/pub/resource/pubs/workshop/2022/TourdelIGS4_05_Hu.pdf)
calls BDCS(2019v01) the current solution and aligns it to ITRF2014.
Assigning 2019v01 to this individual 2022-06-08 C05 broadcast is an
**inference**, not a day-specific CSNO certificate. The library requires
NAV record, orbit reference, and evaluation instants inside
`[2022-06-01, 2022-07-01)` UTC. This narrow sample window is not an
official full validity period.

The path uses an explicit zero-offset BDCS(2019v01) → ITRF2014
**approximation**. It does not freeze 2019 station parameters in 2022
without dated rates and full convention. No satellite-position error
bound is claimed. The result is `UnboundedApproximate`, includes
`CAUTION`, and has no target-frame velocity. `NumericalOnly`, any strict
`max_frame_operation_error_m`, and `require_velocity` reject. A direct point
caller must assert `Realization(FrameId::Bdcs2019v01)`; generic
`BeidouBroadcast` remains unresolved. This unit opens only real PRN C05
D2, not every BeiDou GEO PRN.

## Traceable input and independent expected positions

The [existing GEO fixture provenance](NAV_BDS_GEO.md) identifies the
public `nav-solutions/data` source and license. Original compressed
SHA-256:
`2bae4217cb71ad4a2b9c0067bd1c5b56915e42d2007a94e91eb408468cc4763f`;
fixture SHA-256:
`6bfb1401b5319b58a736fd45463a7d9d2de2f688a236efa43d77431a9da34ed4`.
`nav_bds_geo_frame.py` verifies the raw C05 record epoch and orbit
reference, then uses the existing independent Python GEO Kepler and
5-degree rotation calculation in `nav_bds_geo.py`. The target JSON
explicitly copies the native position for the declared zero-offset
operation. It does not invoke Rust. Script SHA-256:
`da8dfcf5c59599070a086d393848accdc83247303bc6bdc53172137d06989e17`;
JSON SHA-256:
`4f60a24312bec05385bf8c61bb54a35beaa69ab1757d05a6588e9f53cf118578`.
At ToE both declared native and target XYZ are
`[21819.405038660814, 36026.551504041825, 454.58606461052455] km`.
The two other rows are ToE ±300 s. Per-component tolerance is `1e-6 km`.
Agreement checks the GEO orbit arithmetic and declared identity
operation, not physical BDCS-to-ITRF frame accuracy.

From the rinex repository root:

```sh
python3 tests/reference/nav_bds_geo_frame.py > /private/tmp/nav_bds_geo_frame_expected_check.json
diff -u tests/reference/nav_bds_geo_frame_expected.json /private/tmp/nav_bds_geo_frame_expected_check.json
cargo test --offline --features nav,log --test nav_bds_geo_frame -- --nocapture
cargo run --offline --features nav --example nav_frame -- tests/fixtures/nav_bds_geo_c05_2022159.rnx C05 '2022-06-08T09:00:00 BDT'
```

The CLI should report `msgtype: D2`, `Bdcs2019v01`, `Itrf2014`, one
`rinex:BDCS2019v01-ITRF2014:zero-offset-approx` edge,
`UnboundedApproximate`, `CAUTION`, and unavailable target velocity. In VS
Code, use Rust Analyzer **Debug Test** on
`real_c05_d2_geo_reaches_marked_itrf2014_at_its_own_epoch` in
`tests/nav_bds_geo_frame.rs`. Break in `NavCandidate::spatial_state_at`
at the BeiDou realization branch and `FrameTransformer::to_frame` at the
BDCS path. Inspect `key.sv`, `key.msgtype`, `record_epoch`,
`orbit_reference`, requested `epoch`, `source_evidence`, `edge_ids`, and
`position_km`. Predict before running: target XYZ equals native XYZ
numerically, while the quality remains approximate. A generic
`BeidouBroadcast` direct point must fail with unknown realization.
CLI and terminal tests do not establish breakpoint operation or physical
frame accuracy.
