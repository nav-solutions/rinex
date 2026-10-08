# GPS G2296 frame conversion reference: G15, 2024-05-07

This first supported cross-frame path starts with a real GPS LNAV record,
propagates it through the existing NAV state API, and converts the Earth-fixed
position from broadcast WGS 84 (G2296) to ITRF2014 **at the same coordinate
epoch**. It does not use a station position, an ANISE frame, or a kernel.

## Input and independent result

- Public repository source: `data/NAV/V3/NYA100NOR_S_20241280000_01D_GN.rnx.gz`,
  SHA-256 `c423f14595aef4385834b413572068da7aa2dacb02e0e5a9e0876ca897308568`.
- The header and one G15 2024-05-07 02:00:00 LNAV record are copied byte for byte
  into `tests/fixtures/nav_gps_g15_g2296_2024128.rnx`, SHA-256
  `c36899069ec3488fcf53b3a2bc001bf62ab8b87401fb83864ab999dc123af6dd`.
- Evaluate at 2024-05-07 02:05:00 GPST (02:04:42 UTC), ToE + 300 s. The
  separately implemented Python broadcast Kepler reference gives native XYZ
  `[19224.62119621031, -2435.348503636455, 17751.84785453113]` km.
- Independently applying the two frame operations gives ITRF2014 XYZ
  `[19224.62118673597, -2435.3485044483346, 17751.847850344806]` km.
  The Rust test uses 1e-8 km per component for native propagation and
  1e-10 km for the transformed coordinates. These are computation tolerances,
  not physical frame accuracy bounds.

`python3 tests/reference/nav_frame_transform.py` regenerates the JSON reference
without calling the Rust conversion. It reuses `nav_legacy.position` for the
already independently tested GPS broadcast propagation. The frame calculation
is separate Python arithmetic using the published parameter direction and
units. The integration test also checks the native position before comparing
the transformed result.

## Parameter chain and applicability

1. The U.S. GPS [NANU 2024014](https://www.navcen.uscg.gov/gps-constellation)
   reports the G2296 operational update completed by 2024-03-04. The G15
   record is after that date. [NGA's WGS 84 statement](https://earth-info.nga.mil/?action=wgs84&dir=wgs84)
   defines the WGS 84 to ITRF seven parameters as zero by design; the concrete
   [EPSG:10608 operation](https://epsg.io/10608) records G2296 to ITRF2020
   with seven zeros. This edge carries no claimed satellite-position upper
   bound; its 0.01 m operation accuracy at 2024.0 is a different metric.
2. The [IERS ITRF2020 Table 2](https://itrf.ign.fr/en/solutions/itrf2020)
   gives **ITRF2014 minus ITRF2020** at 2015.0: translation
   `[-1.4, -0.9, +1.4]` mm, translation rate `[0.0, -0.1, +0.2]` mm per
   Julian year, scale `-0.42` ppb, scale rate zero, rotations and their rates
   zero. Evaluate at the point's coordinate epoch, then use
   `p_2014 = T(t) + (1+s) p_2020` in SI units once at the boundary.

The fixed catalogue accepts operations touching G2296 only from
2024-03-04 through 2024-12-31 UTC. The ITRF2020 ↔ ITRF2014 operation alone
has a separate 2015-01-01 through 2026-12-31 UTC catalogue window. These are
conservative **implementation windows**, not claims that the frames ceased to
exist afterward.
Other GPS broadcast dates may return bare WGS84 as native with unknown
realization, but cannot enter this cross-frame chain. NavIC's WGS84 family name
does not resolve to GPS G2296. Caller-constructed points may assert a concrete
source realization; results mark that basis as caller asserted. For a NAV
record, the record epoch, orbit reference, and state epoch must all fall in the
G2296 window before the source realization is resolved.
The result exposes the NAV realization evidence separately from the
coordinate operation source. Caller-asserted points have no such NAV evidence.
Input arrays must be Earth-fixed Cartesian kilometres (and optional kilometres
per second). A bare floating-point array does not reveal whether a caller
accidentally supplied metres; the constructor rejects non-finite numbers, but
the caller must assert the physical units and frame correctly.

The IERS parameter uncertainties, the EPSG operation accuracy, and orbit
differences do not establish a strict bound for the frame operation on this satellite position.
`max_frame_operation_error_m` therefore fails for every cross-frame request in this
catalogue. Cross-frame velocity is also withheld pending independent velocity
validation, even when the native NAV state has velocity and the ITRF table
publishes some rates. `require_velocity` fails for this path. Identity requests
can return the caller's original velocity.

This purely terrestrial path does not invoke ANISE or require a frame kernel.

For a caller-owned point, the same converter can be called without a NAV
record or velocity:

```rust
use rinex::navigation::rinex::spatial_state::{
    FrameId, FrameRequest, FrameTransformer, SourceFrameIdentity,
    SpatialPoint, TransformOptions,
};
use rinex::prelude::Epoch;
use std::str::FromStr;

let point = SpatialPoint::new(
    [10000.0, 20000.0, 15000.0],
    Epoch::from_str("2024-05-07T02:05:00 GPST")?,
    SourceFrameIdentity::Realization(FrameId::Itrf2020),
    None,
)?;
let result = FrameTransformer.to_frame(
    &point,
    FrameRequest::Realization(FrameId::Itrf2014),
    TransformOptions::default(),
)?;
assert_eq!(result.target_realization, rinex::navigation::rinex::spatial_state::FrameRealization::Known(FrameId::Itrf2014));
```

The source realization in this call is the caller's assertion, reported as
`CallerAsserted`. The integration test checks this exact input against frozen
independent target XYZ.

## Run and inspect

From the repository root:

```sh
python3 tests/reference/nav_frame_transform.py > /tmp/nav_frame_transform_current.json
diff -u tests/reference/nav_frame_transform_expected.json /tmp/nav_frame_transform_current.json
cargo test --features nav --test nav_frame_transform
cargo run --features nav --example nav_frame -- tests/fixtures/nav_gps_g15_g2296_2024128.rnx G15 '2024-05-07T02:05:00 GPST'
```

In VS Code, use **Debug Test** on
`real_g15_reaches_explicit_itrf2014_with_independent_position` in
`tests/nav_frame_transform.rs`. Useful breakpoints are
`NavCandidate::spatial_state_at`, `FrameTransformer::to_frame`,
`itrf2020_parameters`, and `itrf2020_to_2014`. Inspect `source_id`,
`target_id`, `years`, `translation_mm`, `scale`, and `position_km`.
