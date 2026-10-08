# N09d QZSS J02 JGS → ITRF2014 marked position approximation

This unit covers one RINEX 4.00 `J02 LNAV` QZSS PNT broadcast state at its
own coordinate epoch. The source is the QZSS PNT JGS frame during its
ITRF2014-aligned period. The conversion assumes zero correction to ITRF2014,
but returns `UnboundedApproximate` and a visible `CAUTION`; this is not an
exact identity or a demonstrated satellite-position accuracy bound. The native
state is unchanged, and cross-frame target velocity is unavailable.

## Source, window, and model

The [QZSS Cabinet Office coordinate history](https://qzss.go.jp/en/technical/dod/pnt/coordinate-system.html)
states that PNT broadcast ephemerides use JGS and lists the applied ITRF2014
period as 2021-02-15 through 2023-11-09. The [2021 completion notice](https://qzss.go.jp/en/overview/notices/qzss_210215.html)
reports the ITRF2014 alignment completed on February 15 JST. The
[2023 completion notice](https://qzss.go.jp/en/overview/notices/qzss_231110.html)
reports a switch to ITRF2020 during November 10 JST. The library deliberately
uses the narrower **2021-02-16 00:00:00 through 2023-11-09 00:00:00 UTC,
exclusive at the end** window, starting after the 2021 change date and
stopping before the 2023 change work. NAV
record, orbit reference, and evaluation instants must all be inside it.
This is a library policy, not an official validity interval for every LNAV
record. Existing ephemeris fit and health selection remain separate.

The official coordinate history says the PNT monitor-station frame offset
relative to ITRF should be within 2 cm (95%). That statistic supplies a
reason to offer a **zero-offset approximation** in the stated period; it does
not supply translation/rotation parameters, a strict upper bound, or a
satellite-domain frame error estimate. This edge therefore returns the same
numeric XYZ in km, while reporting distinct source and target realizations,
the zero-offset edge ID, `UnboundedApproximate`, and `CAUTION`. Requests for
`NumericalOnly`, a strict `max_frame_operation_error_m`, or cross-frame velocity fail.
`SpatialPoint::new` callers must assert
`SourceFrameIdentity::Realization(FrameId::QzssJgsItrf2014Aligned)`; a generic
`QzssBroadcastJgs` point has unresolved realization.

## Real input and independent arithmetic

The unchanged fixture `tests/fixtures/nav_qzss_j02_2023071.rnx` is from
`data/NAV/V4/BRD400DLR_S_20230710000_01D_MN.rnx.gz`; source/provenance and
license are described in [NAV_QZSS_LNAV.md](NAV_QZSS_LNAV.md). Fixture SHA-256:
`05ab6e9da648a1e0307b155bd04befde148a26cc85c97e0b3960a5b0f101de14`.
The independent raw-RINEX propagation script
`tests/reference/nav_qzss_lnav.py` has SHA-256
`13e987783be488d53ebc729731d5c73cb312758cbe0249ed0284222ff4ca8930`;
its checked-in JSON has SHA-256
`548a0302017def03ba2cb89f8cc7d9f8d72c6e990fecfc8d19b790ba9fbda43e`.
The script does not call Rust code. It computes J02 at ToE −1800 s, ToE,
and ToE +1800 s, including a QZSS week crossing. Under the declared zero
correction, the **independent target XYZ** at ToE = 2023-03-12 00:00:00 QZSST
is `[-27925.241669130697, 25147.710913540428, 24120.917646932096] km`.
The other two targets are the corresponding JSON `position_km` rows.
Per-component comparison tolerance is `1e-6 km`, set by the existing
independent broadcast-propagation check. This verifies the implemented
arithmetic and labeling, not a physical zero frame offset.

From the rinex repository root:

```sh
python3 tests/reference/nav_qzss_lnav.py > /private/tmp/nav_qzss_lnav_expected_n09d.json
diff -u tests/reference/nav_qzss_lnav_expected.json /private/tmp/nav_qzss_lnav_expected_n09d.json
cargo test --offline --features nav,log --test nav_qzss_frame -- --nocapture
cargo run --offline --features nav --example nav_frame -- tests/fixtures/nav_qzss_j02_2023071.rnx J02 '2023-03-12T00:00:00 QZSST'
```

The CLI should show `QzssJgsItrf2014Aligned`, `Itrf2014`,
`UnboundedApproximate`, the zero-offset edge ID, `velocity_km_s=None`,
and `CAUTION`. For VS Code, use Rust Analyzer's **Debug Test** on
`real_j02_reaches_marked_itrf2014_approximation_at_its_own_epoch` in
`tests/nav_qzss_frame.rs`. Break at `NavCandidate::spatial_state_at` where
`realization` is chosen, then at `FrameTransformer::to_frame` where
`source_id`, `target_id`, and `position_km` are set. Inspect the three input
instants, source evidence, method and edge info. Terminal tests do not prove
VS Code breakpoint operation or physical frame accuracy.
