# N09d BeiDou-2 C10 D1 IGSO BDCS(2019v01) → ITRF2014 marked position path

This unit uses the real C10 D1 IGSO record in
`tests/fixtures/nav_legacy_kms_2022159.rnx` at 2022-06-08 07:00:00 BDT
(ToE 284400 s). The existing `NativeFrame::Cgcs2000` labels the broad
historical BeiDou frame/ellipsoid family. The specific source path is
`BeidouBroadcast` with `Realization(Bdcs2019v01)`; the broad label alone
does not establish a concrete CGCS2000 operational realization.

## Source identity and conversion limit

The [CSNO Open Service Performance Standard](https://www.beidou.gov.cn/xt/gfxz/201812/P020181227529449178798.pdf)
defines broadcast positions in BDCS, aligned to the latest ITRF with annual
updates. The [2019 CSNO ICD](https://www.beidou.gov.cn/xt/gfxz/201912/P020200429814796324960.pdf)
also defines BDCS. [IGS 2022 workshop material](https://files.igs.org/pub/resource/pubs/workshop/2022/TourdelIGS4_05_Hu.pdf)
calls BDCS(2019v01), aligned to ITRF2014, the current solution. The
[IGS constellation table](https://www.igs.org/mgex/constellations/) identifies
C10 as a BeiDou-2 IGSO. Assigning the 2019v01 version to this individual
2022-06-08 broadcast is an **inference**, not a day-specific CSNO
certificate. The library therefore requires NAV record, orbit reference,
and evaluation instants inside `[2022-06-01, 2022-07-01)` UTC. This narrow
library window is not an official full validity period.

The path applies an explicit zero-offset BDCS(2019v01) → ITRF2014
**approximation**. The 2019 workshop station parameters are not applied as
frozen 2022 parameters because a dated rate model and full convention for
that operation were not established. No satellite-position error bound is
claimed. The result is `UnboundedApproximate`, includes `CAUTION`, and has
no target-frame velocity. `NumericalOnly`, any strict
`max_frame_operation_error_m`, and `require_velocity` reject. A direct point
caller must assert `Realization(FrameId::Bdcs2019v01)`; the generic
`BeidouBroadcast` identity remains unresolved. C20 D1 MEO is covered in a
[separate dated unit](nav_bds_meo_frame_approx.md); other BeiDou orbit/message
classes beyond the separately documented C05 D2 GEO unit and dates outside
this sample window remain unresolved.

## Traceable input and independent expected positions

The [existing fixture provenance](NAV_LEGACY.md) gives the public
`nav-solutions/data` commit, original compressed SHA-256
`2bae4217cb71ad4a2b9c0067bd1c5b56915e42d2007a94e91eb408468cc4763f`
and fixture SHA-256
`579b5da043a98f8027f7c191a5b2f239f033da252b14fe7dcb00622761062425`.
`nav_bds_igso_frame.py` reads the 19-character C10 D1 orbit slots directly
and uses the existing independent Python BeiDou Kepler implementation in
`nav_legacy.py`; the target JSON explicitly copies the native position for
the declared zero-offset approximation. It does not invoke Rust. Script
SHA-256 `4f663d4e8d0854093eeee0b3c611a00f526e8e054f242a9673be3d2522847cf4`;
JSON SHA-256 `02e941d54e9848a3f8b203500715ce4a21ded8e1afb795a8c51ebea18cd488f6`.
At ToE both declared native and target XYZ are
`[-5417.088102384822, 40682.625743301054, 8728.669264949665] km`.
The two other rows are ToE ±300 s. Per-component tolerance is `1e-6 km`.
Agreement checks orbit arithmetic and the declared identity operation,
not physical BDCS-to-ITRF frame accuracy.

From the rinex repository root:

```sh
python3 tests/reference/nav_bds_igso_frame.py > /private/tmp/nav_bds_igso_frame_expected_check.json
diff -u tests/reference/nav_bds_igso_frame_expected.json /private/tmp/nav_bds_igso_frame_expected_check.json
cargo test --offline --features nav,log --test nav_bds_igso_frame -- --nocapture
cargo run --offline --features nav --example nav_frame -- tests/fixtures/nav_legacy_kms_2022159.rnx C10 '2022-06-08T07:00:00 BDT'
```

The CLI should report `msgtype: D1`, `Bdcs2019v01`, `Itrf2014`, one
`rinex:BDCS2019v01-ITRF2014:zero-offset-approx` edge,
`UnboundedApproximate`, `CAUTION`, and unavailable target velocity. In VS
Code, use Rust Analyzer **Debug Test** on
`real_c10_d1_igso_reaches_marked_itrf2014_at_its_own_epoch` in
`tests/nav_bds_igso_frame.rs`. Break in `NavCandidate::spatial_state_at`
at the BeiDou realization branch and `FrameTransformer::to_frame` at the
BDCS path. Inspect `key.msgtype`, `record_epoch`, `orbit_reference`,
requested `epoch`, `source_evidence`, `edge_ids`, and `position_km`.
Predict before running: the target XYZ equals the native XYZ numerically
because the declared operation is zero-offset, while the quality remains
approximate. C20 follows its separately documented MEO path. Terminal checks
do not establish VS Code breakpoint operation or physical frame accuracy.
