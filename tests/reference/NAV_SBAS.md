# SBAS GEO broadcast state reference

`tests/fixtures/nav_sbas_s27_2023071.rnx` contains the original RINEX 4.00 header and one S27 SBAS EPH record, copied verbatim from the public `nav-solutions/data` submodule at commit `209bfbd7016bd654f256238768a9e030ec5ab299`, `NAV/V4/BRD400DLR_S_20230710000_01D_MN.rnx.gz`. The source identifies DLR/GSOC, O. Montenbruck and P. Steigenberger, and DOI <https://doi.org/10.57677/BRD400DLR>. The data submodule is MPL-2.0. Source gzip SHA-256: `bb07453e63eaba0f4b1a2b20b089655b045b328ec9f90fc87b54a2b519ec94ca`. Small fixture SHA-256: `93c7b062ebb651d17c02cc4cebb023b4717c6b56cec6b6a24a63fdc8b718fbee`.

[IGS RINEX 4.02](https://files.igs.org/pub/data/format/rinex_4.02.pdf), sections 5.4.7 and Table A28, defines the same SBAS record layout: GPS-time clock epoch, `aGf0` (s), `aGf1` (s/s), transmission time `t_tm` (GPS seconds of week), and each axis's position (km), velocity (km/s), acceleration (km/s²). `t_tm` is neither GPS week number nor a quadratic clock term. The orbit and clock reference in this record is the clock-line epoch; no separate ToE field exists in RINEX Table A28. The returned native frame is labeled `SbasBroadcast` because the record alone does not state a versioned realization of the service's terrestrial frame. The native polynomial applies no frame transform.

For each axis, the independently evaluated model is `p(t)=p0+v0*dt+a*dt²/2`, `v(t)=v0+a*dt`, with `dt=t−Toc` in seconds. The minimal satellite clock correction is `aGf0+aGf1*dt` (s). The separate Python script reads the fixed-width record directly, without the Rust parser or state code, and writes `nav_sbas_s27_expected.json` for `dt=-300,-120,0,120,300 s`. This checks field mapping, sign and arithmetic against the actual S27 record. The synthetic per-axis test also uses distinct coefficients, including explicit zero, to expose axis-copy mistakes. It does not assess physical accuracy against a precise orbit product.

[RTKLIB's `seph2pos`](https://github.com/tomojitakasu/RTKLIB/blob/master/src/ephemeris.c) provides an independent published implementation of the same SBAS position and clock polynomial. We did not run its binary for this comparison.

Selection uses `|dt| < 360 s`, a conservative bound matching [RTKLIB's SBAS maximum ToE difference](https://github.com/tomojitakasu/RTKLIB/blob/master/src/rtklib.h); this is a library policy, not a claim of a universal RINEX validity interval. Health zero is accepted; nonzero MT17 status or URA index 15 is rejected; unavailable or missing health is unknown. This state API does not implement the SBAS differential correction system, integrity service, or observation processing.

Run from the rinex root:

```sh
python3 tests/reference/nav_sbas_s27.py > /tmp/nav_sbas_s27_expected.json
diff -u tests/reference/nav_sbas_s27_expected.json /tmp/nav_sbas_s27_expected.json
cargo test --features nav --test nav_sbas -- --nocapture
```
