# N09d Galileo E08 INAV GTRF23v01 → ITRF2014 marked position path

This unit covers a real RINEX 4.00 `E08 INAV` record at 2024-05-07
00:30:00 GST. The source identity is GTRF23v01 under the narrow catalogue
inference below. The first operation uses **zero correction** from GTRF23v01
to ITRF2020 as an unbounded approximation; the second applies the published
ITRF2020 → ITRF2014 Table 2 transform at each point's own coordinate epoch.
The overall method is `UnboundedApproximate`, visibly marked `CAUTION`.
This is neither an exact identity nor a satellite-position accuracy bound.

## Source identity, date, and limits

The [Galileo Open Service definition](https://www.gsc-europa.eu/sites/default/files/sites/all/files/Galileo-OS-SDD_v1.2.pdf)
places broadcast instantaneous satellite positions in GTRF. The
[ESA/ESOC GGSP presentation](https://navigation-office.esa.int/attachments/32835744/1/GTRF_IGS_Stop_6.pdf),
dated 2023-05-23, states that GTRF23v01 is aligned to ITRF2020 and
**applicable since 2023-05-05**. The [ICG eighteenth-meeting report](https://documents.un.org/doc/undoc/gen/v24/073/00/pdf/v2407300.pdf),
covering October 2024, reports ESA planned a GTRF update in the following
months. Taken together, these support an **inference**, not an explicit
day-stamped ESA statement, that GTRF23v01 remained applicable for this
2024-05-07 sample. The library accepts only 2024-05-01 00:00:00 through
2024-06-01 00:00:00 UTC (exclusive end). This is a deliberately narrow
library policy, not the official full validity interval. NAV record, orbit
reference, and evaluation instants must all be inside it. The older
2022-06-08 E08 remains physically unresolved; the current API returns `UnknownSourceRealization` after native propagation. FNAV has a separate regression fixture.

ESA gives a GTRF-to-ITRF alignment requirement of 3 cm (2σ) for reference
stations. It publishes no seven-parameter GTRF23v01 → ITRF2020 correction
in the cited presentation. The zero-offset first edge is therefore an
explicit practical approximation without a strict satellite-position bound.
The [IERS/IGN ITRF2020 Table 2](https://itrf.ign.fr/en/solutions/itrf2020)
gives ITRF2020 → ITRF2014 translations in mm, scale in ppb, and rates at
2015.0; its published parameters are the second, numerical edge. The
approximate first edge governs the whole path: `NumericalOnly`, any strict
`max_frame_operation_error_m`, and `require_velocity` all fail. Target velocity is
unavailable. A direct point caller must assert
`Realization(FrameId::GalileoGtrf23v01)`; a generic
`GalileoBroadcastGtrf` point remains unresolved.

## Traceable real input and independent numbers

The unchanged microfixture `tests/fixtures/nav_galileo_e08_inav_2024128.rnx`
contains the original nine-line RINEX 4.00 header and the first E08 INAV
record at source file line 74090, copied byte-for-byte from DLR's public
`BRD400DLR_S_20241280000_01D_MN.rnx.gz` product via the
[IGS/BKG daily archive](https://igs.bkg.bund.de/root_ftp/IGS/BRDC/2024/128/BRD400DLR_S_20241280000_01D_MN.rnx.gz).
The [IGS product description](https://igs.org/mgex/mgex-product-descriptions/)
identifies BRD400DLR as DLR's RINEX 4 merged broadcast data product and gives
its dataset DOI; [IGS MGEX states its data and products are freely available for public use and requests citation for publications](https://www.igs.org/mgex/).
The source header itself carries that DOI. Compressed source
SHA-256: `ec027e4a167d3d08c17acfc3001fdf0cbbc723a8416fb5684b1d95b66291926f`;
fixture SHA-256:
`88373b4e0255ee8d885f7798a95628c9e33ab3d09a818a000ba75ffd53b7a8f5`.
The fixture retains original RINEX fixed-width padding.

The independent `tests/reference/nav_galileo_frame.py` reads raw RINEX
19-character slots, reuses the existing Python implementation of the
published Galileo Kepler equations from `nav_legacy.py`, and separately
evaluates ITRF Table 2 at each UTC instant. It never calls Rust. Script
SHA-256: `4fb390fbf63678d9d3523fbd7492434efb9b86fbb6e007a5dba8e1e63f542cef`;
expected JSON SHA-256:
`dd351cf06cf97c036db92cdfdbeadeee76f71125d6bae7da8178bf047982272f`.
At ToE, its native XYZ is
`[12178.135715449172, -13528.834212660882, 23344.83270688306] km`;
the declared approximate path yields ITRF2014 XYZ
`[12178.135708934355, -13528.83420881348, 23344.832700347644] km`.
The other two rows use ToE ±300 s. Per-component comparison tolerance is
`1e-6 km`, inherited from the independent broadcast propagation reference.
Agreement verifies the implemented arithmetic and labels, not physical
GTRF-to-ITRF zero error. The native-to-target change is nonzero because the
ITRF2020 → ITRF2014 edge is numerical.

From the rinex repository root:

```sh
python3 tests/reference/nav_galileo_frame.py > /private/tmp/nav_galileo_frame_expected_n09d.json
diff -u tests/reference/nav_galileo_frame_expected.json /private/tmp/nav_galileo_frame_expected_n09d.json
cargo test --offline --features nav,log --test nav_galileo_frame -- --nocapture
cargo run --offline --features nav --example nav_frame -- tests/fixtures/nav_galileo_e08_inav_2024128.rnx E08 '2024-05-07T00:30:00 GST'
```

The CLI should show `GalileoGtrf23v01`, `Itrf2014`, both ordered edge IDs,
`UnboundedApproximate`, `CAUTION`, and `velocity_km_s=None`. In VS
Code, use Rust Analyzer **Debug Test** on
`real_e08_inav_reaches_marked_itrf2014_at_its_own_epoch` in
`tests/nav_galileo_frame.rs`. Break in `NavCandidate::spatial_state_at` at
`realization`, then `FrameTransformer::to_frame` at the Galileo path. Inspect
`record_epoch`, `orbit_reference`, requested `epoch`, `source_evidence`,
`source_id`, `target_id`, `edge_info`, and `position_km`. Terminal success
does not verify VS Code breakpoint operation or scientific frame accuracy.
