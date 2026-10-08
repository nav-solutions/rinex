# N09d BeiDou-3 C20 D1 MEO BDCS(2019v01) → ITRF2014 marked position path

This unit uses the real C20 D1 MEO record in
`tests/fixtures/nav_legacy_kms_2022159.rnx` at 2022-06-08 09:00:00 BDT
(ToE 291600 s). The existing `NativeFrame::Cgcs2000` labels a broad
BeiDou frame/ellipsoid family. The specific source path is
`BeidouBroadcast` with `Realization(Bdcs2019v01)`; that broad label alone
does not establish a concrete operational realization.

## Source identity and conversion limit

The [IGS MGEX constellation table](https://www.igs.org/mgex/constellations/)
identifies PRN C20 as BDS-3 MEO-2 (SVN C202) in this period. The
[CSNO Open Service Performance Standard](https://www.beidou.gov.cn/xt/gfxz/201812/P020181227529449178798.pdf)
explicitly lists D1 for BDS-3 MEO B1I/B3I and defines broadcast positions
in BDCS, which is aligned to ITRF and updated annually. The
[IGS 2022 workshop material](https://files.igs.org/pub/resource/pubs/workshop/2022/TourdelIGS4_05_Hu.pdf)
calls BDCS(2019v01) the current solution and aligns it to ITRF2014.
Assigning 2019v01 to this individual 2022-06-08 C20 broadcast is an
**inference**, not a day-specific CSNO certificate. The library requires
NAV record, orbit reference, and evaluation instants inside
`[2022-06-01, 2022-07-01)` UTC. This is a conservative sample window,
not an official full validity period.

The path uses the existing explicit zero-offset BDCS(2019v01) → ITRF2014
**approximation**. It does not freeze the 2019 workshop station parameters
in 2022 without dated rates and full convention. No satellite-position
error bound is claimed. The result is `UnboundedApproximate`, includes
`CAUTION`, and has no target-frame velocity. `NumericalOnly`, any strict
`max_frame_operation_error_m`, and `require_velocity` reject. A direct point
caller must assert `Realization(FrameId::Bdcs2019v01)`; generic
`BeidouBroadcast` remains unresolved. This unit opens only real PRN C20
D1, not every BDS-3 MEO PRN. C05 D2 GEO has a
[separate dated unit](nav_bds_geo_frame_approx.md); other dates remain
unresolved.

## Traceable input and independent expected positions

The [existing fixture provenance](NAV_LEGACY.md) identifies the public
`nav-solutions/data` source and license. Original compressed SHA-256:
`2bae4217cb71ad4a2b9c0067bd1c5b56915e42d2007a94e91eb408468cc4763f`;
fixture SHA-256:
`579b5da043a98f8027f7c191a5b2f239f033da252b14fe7dcb00622761062425`.
`nav_bds_meo_frame.py` reads the 19-character C20 D1 orbit slots directly
and uses the established independent Python BeiDou Kepler calculation in
`nav_legacy.py`; target JSON explicitly copies the native position for
the declared zero-offset operation. It does not invoke Rust. Script
SHA-256 `3c553426bf440f8d2f4b69a0fe30c588276cf2c973adf6e60cf6cabc217cd266`;
JSON SHA-256 `832dd1ec0041b87e16f3d94bb8b56e9b11566dd4973dfee094317edd21854e17`.
At ToE both declared native and target XYZ are
`[12978.091071180865, 20600.509363038196, -13586.238150668227] km`.
The two other rows are ToE ±300 s. Per-component tolerance is `1e-6 km`.
Agreement checks orbit arithmetic and the declared identity operation,
not physical BDCS-to-ITRF frame accuracy.

From the rinex repository root:

```sh
python3 tests/reference/nav_bds_meo_frame.py > /private/tmp/nav_bds_meo_frame_expected_check.json
diff -u tests/reference/nav_bds_meo_frame_expected.json /private/tmp/nav_bds_meo_frame_expected_check.json
cargo test --offline --features nav,log --test nav_bds_meo_frame -- --nocapture
cargo run --offline --features nav --example nav_frame -- tests/fixtures/nav_legacy_kms_2022159.rnx C20 '2022-06-08T09:00:00 BDT'
```

The CLI should report `msgtype: D1`, `Bdcs2019v01`, `Itrf2014`, one
`rinex:BDCS2019v01-ITRF2014:zero-offset-approx` edge,
`UnboundedApproximate`, `CAUTION`, and unavailable target velocity. In VS
Code, use Rust Analyzer **Debug Test** on
`real_c20_d1_meo_reaches_marked_itrf2014_at_its_own_epoch` in
`tests/nav_bds_meo_frame.rs`. Break in `NavCandidate::spatial_state_at`
at the BeiDou realization branch and `FrameTransformer::to_frame` at the
BDCS path. Inspect `key.sv`, `key.msgtype`, `record_epoch`,
`orbit_reference`, requested `epoch`, `source_evidence`, `edge_ids`, and
`position_km`. Predict before running: target XYZ equals native XYZ
numerically, while the quality remains approximate. A generic
`BeidouBroadcast` direct point must fail with unknown realization. CLI
and terminal tests do not establish breakpoint operation or physical
frame accuracy.
