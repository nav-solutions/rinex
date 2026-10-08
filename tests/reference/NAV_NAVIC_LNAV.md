# NavIC LNAV reference

The RINEX 4.02 normative I02 LNAV layout is checked against the two I02
records in `data/NAV/V4/rinex402_examples_MN.rnx` (IGS/RTCM Table A30).
The actual small sample in `tests/fixtures/nav_navic_i02_2023071.rnx`
copies the original RINEX 4.00 header and one I02 LNAV record verbatim from
the public `nav-solutions/data` submodule file
`NAV/V4/BRD400DLR_S_20230710000_01D_MN.rnx.gz` at commit
`209bfbd7016bd654f256238768a9e030ec5ab299` (MPL-2.0). That file
identifies DLR/GSOC, O. Montenbruck and P. Steigenberger and
<https://doi.org/10.57677/BRD400DLR>. Source gzip SHA-256:
`bb07453e63eaba0f4b1a2b20b089655b045b328ec9f90fc87b54a2b519ec94ca`.
Extracted fixture SHA-256:
`ae632c2debb026be9acaf207549a903bda9d89466c0ed1535c0e7cb2b2309ecd`.
The record has continuous week 2253, ToE = ToC = 2023-03-12 00:00:00 in
the crate's GPST proxy, IODEC 0, and health 0 (both L5 and S healthy).
[IGS MGEX metadata](https://connect.igs.org/mgex/constellations/) identifies
I02 as IRNSS-1B in an inclined geosynchronous orbit.

[IGS RINEX 4.02](https://files.igs.org/pub/data/format/rinex_4.02.pdf)
sections 4.1.6 and Table A30 define the NavIC time notation, GPS-aligned
continuous week, record layout, and 2-bit health meaning. The
[ISRO NavIC SPS ICD 1.1](https://www.isro.gov.in/media_isro/pdf/SateliteNavigation/irnss_sps_icd_version1.1-2017.pdf)
sections 5.7, 5.8 and 6.2.1 and appendices A/B define IRNSST, WGS-84,
health, clock and orbit equations. Appendix B gives μ = 3.986005 × 10¹⁴
m³/s² and Earth rate = 7.2921151467 × 10⁻⁵ rad/s. The clock output adds
the relativistic term and excludes the signal-specific TGD and other
measurement corrections. Positions and velocities are in native WGS-84
ECEF km and km/s. No frame transform is performed.

IRNSST and GPST share the week origin, but their realizations can differ
slightly. The time library has no IRNSST variant, so the existing parser
and this reference use GPST as a proxy. This is an explicit approximation,
not a claim that the two physical timescales are exactly equal. The ICD
lists nominal IODEC update rates, including two-hour sets, but Table A30
has no fit field and the ICD does not state a single ToE-centered fit
interval for all records. Selection retains the pre-existing `|t − ToE| <
7200 s` policy and now applies the same bound to ToC. That bound is a
library acceptance policy; it is not an ICD certified validity guarantee.
With no requested signal, only health code 0 is selected. A signal-specific
decision and TGD correction require another API.

`nav_navic_lnav.py` reads the 19-character RINEX slots directly and
evaluates the ICD equations without importing the Rust crate. It obtains
reference velocity by centered position difference with a 0.05 s half-step,
retaining the same ephemeris. The JSON covers ToE −1800, ToE, and ToE
+1800 s; the negative offset crosses the week boundary. Bounds chosen
before comparison: 1e-6 km per position component, 1e-7 km/s per velocity
component, and 1e-10 s for clock. This checks the algorithm and serial
field mapping, not physical accuracy against a precise orbit/clock product.

Run from the rinex root:

```sh
python3 tests/reference/nav_navic_lnav.py > /tmp/nav_navic_lnav_expected.json
diff -u tests/reference/nav_navic_lnav_expected.json /tmp/nav_navic_lnav_expected.json
cargo test --features nav --test nav_navic_lnav -- --nocapture
```
