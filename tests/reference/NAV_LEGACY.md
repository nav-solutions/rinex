# Traditional broadcast reference

The five records in `tests/fixtures/nav_legacy_kms_2022159.rnx` are copied
verbatim, including the RINEX 4.00 header, from the public
`nav-solutions/data` submodule file
`NAV/V4/KMS300DNK_R_20221591000_01H_MN.rnx.gz` at commit
`209bfbd7016bd654f256238768a9e030ec5ab299` (MPL-2.0).
Original compressed SHA-256:
`2bae4217cb71ad4a2b9c0067bd1c5b56915e42d2007a94e91eb408468cc4763f`.
Extracted fixture SHA-256:
`579b5da043a98f8027f7c191a5b2f239f033da252b14fe7dcb00622761062425`.
The records are G02 LNAV, E08 INAV, E08 FNAV, C10 D1 (IGSO), and C20 D1
(MEO). IGS MGEX constellation metadata identifies the 2022 C10/C20 orbit
classes: <https://www.igs.org/mgex/constellations/>.

`nav_legacy.py` reads the original 19-character RINEX slots directly. It
implements the position and clock equations independently of the Rust crate,
following RTKLIB `eph2pos` in `src/ephemeris.c` at commit
`180043ee24b6d2b168f98b64be15f69d50046b1a`:
<https://github.com/tomojitakasu/RTKLIB/blob/180043ee24b6d2b168f98b64be15f69d50046b1a/src/ephemeris.c>.
The Python reference is an adapted equation implementation, not a run of the
RTKLIB binary. RTKLIB's repository/license and the RINEX 4.00 specification
are public; the reference script itself is part of this MPL-2.0 repository.
The Galileo week field is aligned to the GPS week as specified by RINEX 4.00:
<https://files.igs.org/pub/data/format/rinex_4.00.pdf>.

Run from the rinex root:

```sh
python3 tests/reference/nav_legacy.py > /tmp/nav_legacy_expected.json
diff -u tests/reference/nav_legacy_expected.json /tmp/nav_legacy_expected.json
cargo test --features nav --test nav_legacy -- --nocapture
```

The reference evaluates each fixed ephemeris at ToE plus -300, 0 and +300 s
in its message system time. Position is native broadcast ECEF (GPS WGS-84,
Galileo GTRF, BeiDou CGCS2000), in km. Velocity is the native ECEF time
derivative in km/s, computed by a centered difference of the independent
position reference with 0.05 s half-step; no record is reselected during the
difference. Clock is broadcast polynomial at ToC plus Kepler relativistic
correction, in seconds, without signal group delay. No frame conversion,
antenna offset, Sagnac correction or signal-specific delay is applied.

Before comparison, the limits are 1e-6 km for position, 1e-7 km/s for
velocity, and 1e-10 s for clock. They allow RINEX decimal serialization and
the numerical difference error while still resolving the former Y-velocity
derivative sign error. The analytic circular-orbit test independently checks
the zero-harmonic and week-rollover behavior. These checks are algorithmic
regressions, not an assessment against precise orbit products.
