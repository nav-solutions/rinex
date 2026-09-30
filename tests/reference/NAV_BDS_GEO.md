# BeiDou GEO reference

`tests/fixtures/nav_bds_geo_c05_2022159.rnx` contains the original RINEX
4.00 header and one unmodified C05 D2 record from the public
`nav-solutions/data` submodule, commit
`209bfbd7016bd654f256238768a9e030ec5ab299`, file
`NAV/V4/KMS300DNK_R_20221591000_01H_MN.rnx.gz` (MPL-2.0).
Compressed source SHA-256:
`2bae4217cb71ad4a2b9c0067bd1c5b56915e42d2007a94e91eb408468cc4763f`.
Extracted fixture SHA-256:
`6bfb1401b5319b58a736fd45463a7d9d2de2f688a236efa43d77431a9da34ed4`.
RINEX 4.00/4.02 defines D2 as the BeiDou GEO legacy message, independently
of a PRN range: <https://files.igs.org/pub/data/format/rinex_4.02.pdf>,
section 8.3.12, Table A23. The fixture C05 is listed as BDS-2 GEO by IGS
MGEX: <https://www.igs.org/mgex/constellations/>.

`nav_bds_geo.py` reads the 19-character RINEX slots directly, solves Kepler's
equation, and applies the GEO rotation from RTKLIB `eph2pos`,
`src/ephemeris.c`, commit `180043ee24b6d2b168f98b64be15f69d50046b1a`:
<https://github.com/tomojitakasu/RTKLIB/blob/180043ee24b6d2b168f98b64be15f69d50046b1a/src/ephemeris.c>.
It is an independently written equation adaptation, not an execution of
RTKLIB. It uses the published BeiDou constants `mu=3.986004418e14 m^3/s^2`,
`omega=7.292115e-5 rad/s`, and tilt `+5 degrees`. The `+5` agrees with
RTKLIB's `SIN_5 = sin(-5 degrees)` after expanding its component equations.

From the rinex root:

```sh
python3 tests/reference/nav_bds_geo.py > /tmp/nav_bds_geo_expected.json
diff -u tests/reference/nav_bds_geo_expected.json /tmp/nav_bds_geo_expected.json
cargo test --features nav --test nav_bds_geo -- --nocapture
```

The expected JSON holds the fixed C05 D2 ephemeris at ToE -300, 0, +300,
and +1800 seconds in BDT. Position is broadcast CGCS2000 ECEF in km.
Velocity is the derivative in that same frame in km/s; the reference uses
a 0.05 second centered position difference on the **same record**.
Before comparison, tolerances were set to `1e-6 km` per position component
and `1e-7 km/s` per velocity component, allowing RINEX decimal serialization
and finite-difference error while resolving a wrong 5-degree rotation or
Earth-rate derivative. The test also uses a second centered difference of
the Rust position to check its analytic velocity, which alone would only
show self-consistency. A synthetic zero-harmonic, week-rollover case has a
separate analytic position expectation.

The API does not transform CGCS2000 to another frame. Its clock output
excludes group delay; this reference validates position and velocity,
not GEO clock accuracy against an independent clock product. No antenna
offset, signal delay, Sagnac correction or precise-orbit comparison is applied.
