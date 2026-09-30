# QZSS LNAV reference

`tests/fixtures/nav_qzss_j02_2023071.rnx` contains the original RINEX 4.00
header and one J02 LNAV record copied verbatim from the public
`nav-solutions/data` submodule's
`NAV/V4/BRD400DLR_S_20230710000_01D_MN.rnx.gz` at commit
`209bfbd7016bd654f256238768a9e030ec5ab299` (MPL-2.0). The source
identifies DLR/GSOC, O. Montenbruck and P. Steigenberger and
<https://doi.org/10.57677/BRD400DLR>. Source gzip SHA-256:
`bb07453e63eaba0f4b1a2b20b089655b045b328ec9f90fc87b54a2b519ec94ca`.
Extracted fixture SHA-256:
`05ab6e9da648a1e0307b155bd04befde148a26cc85c97e0b3960a5b0f101de14`.
The record has QZSS week 2253, ToE = ToC = 2023-03-12 00:00:00 QZSST,
zero health and fit flag, and nonzero eccentricity and harmonic corrections.
[IGS MGEX constellation metadata](https://www.igs.org/mgex/constellations/)
identifies J02 as QZS-2; [the QZSS satellite list](https://qzss.go.jp/en/technical/satellites/)
classifies QZS-2 as QZO.

[RINEX 4.02 table A19](https://files.igs.org/pub/data/format/rinex_4.02.pdf)
defines the J/LNAV field layout and GPS-aligned continuous week number.
[IS-QZSS-PNT-006](https://qzss.go.jp/en/technical/download/pdf/ps-is-qzss/is-qzss-pnt-006.pdf)
sections 5.1, 5.3, 5.5 and 5.6.1 define QZSST and the orbit/clock equations,
with μ = 3.986005 × 10¹⁴ m³/s² and Earth rate = 7.2921151467 × 10⁻⁵ rad/s.
Its table 4.1.1-2 gives a two-hour total validity period about each of ToE
and ToC: the selected state requires both |t − ToE| < 3600 s and
|t − ToC| < 3600 s. The [QZSS coordinate notice](https://qzss.go.jp/en/technical/dod/pnt/coordinate-system.html)
identifies the broadcast terrestrial frame as JGS, not WGS-84; the native
state remains in JGS. This native state does not convert to ITRF2014.

`nav_qzss_lnav.py` reads the 19-character RINEX fields directly and evaluates
the ICD equations without importing Rust code. It uses a centered position
difference with 0.05 s half-step for reference velocity, always retaining the
same ephemeris. The fixed JSON checks ToE ±1800 s and ToE; the negative offset
crosses the QZSS week boundary. `nav_qzss_lnav.rs` also has an explicitly
synthetic circular equatorial record with analytic position/velocity, plus
health, fit flag, message and validity failures. Before comparison, bounds
were set to 1e-6 km per position component, 1e-7 km/s per velocity component,
and 1e-10 s for the broadcast clock without signal group delay. These are
algorithmic checks, not validation against precise orbit and clock products.

The inherited orbit descriptor calls QZSS's reserved/ephemeris-status slot
`l2Codes`; the J02 raw value is 2. The selection path does not interpret it
as GPS L2 codes or as invalid data. Health selection has no signal argument:
only an all-zero QZSS health code is accepted by default. A signal-specific
clock correction, group delay and per-signal health decision remain outside
this state API.

From the rinex repository root:

```sh
python3 tests/reference/nav_qzss_lnav.py > /tmp/nav_qzss_lnav_expected.json
diff -u tests/reference/nav_qzss_lnav_expected.json /tmp/nav_qzss_lnav_expected.json
cargo test --features nav --test nav_qzss_lnav -- --nocapture
```
