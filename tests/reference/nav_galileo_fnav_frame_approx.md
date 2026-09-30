# N09d Galileo E08 F/NAV GTRF23v01 → ITRF2014 marked position path

This unit uses a real RINEX 4.00 `E08 FNAV` record at 2024-05-07
00:40:00 GST (ToE 175200 s). Its orbit fields differ from the earlier 00:30
I/NAV fixture. The selected message must remain F/NAV; a matching satellite
and date do not justify silently substituting I/NAV.

## Source identity and conversion limit

The [Galileo OS SDD 1.3](https://www.gsc-europa.eu/sites/default/files/sites/all/files/Galileo-OS-SDD_v1.3.pdf)
identifies F/NAV as the E5a-I message and states that instantaneous satellite
positions derived from OS SIS navigation data are in GTRF. The
[ESA/ESOC GGSP presentation](https://navigation-office.esa.int/attachments/32835744/1/GTRF_IGS_Stop_6.pdf)
dates GTRF23v01, aligned to ITRF2020, as applicable from 2023-05-05. The
[October 2024 ICG report](https://documents.un.org/doc/undoc/gen/v24/073/00/pdf/v2407300.pdf)
still describes ESA's next GTRF update as planned. As for the accepted I/NAV
unit, assigning GTRF23v01 to this May 2024 F/NAV record is an **inference**,
not an ESA day-specific certificate. The library checks the NAV record,
orbit reference, and evaluation instants inside `[2024-05-01, 2024-06-01)`
UTC. That is a narrow library policy, not the full official validity period.

The GTRF23v01 → ITRF2020 step uses an explicit zero-offset approximation
without a strict satellite-position error bound. The second step uses the
published [ITRF2020 Table 2](https://itrf.ign.fr/en/solutions/itrf2020)
translations, scale and rates to ITRF2014 at the point's own epoch. The
whole path is `UnboundedApproximate`, carries `CAUTION`, and returns no
target velocity. `NumericalOnly`, any strict `max_frame_operation_error_m`, and
`require_velocity` reject. A direct point caller must assert
`Realization(FrameId::GalileoGtrf23v01)`; a generic GTRF label is unresolved.
The older 2022 E08 messages still have unknown realization.

## Traceable input and independent expected positions

The microfixture `tests/fixtures/nav_galileo_e08_fnav_2024128.rnx` is the
unaltered nine-line header plus the single F/NAV record beginning at line
75052 of DLR's public
[`BRD400DLR_S_20241280000_01D_MN.rnx.gz`](https://igs.bkg.bund.de/root_ftp/IGS/BRDC/2024/128/BRD400DLR_S_20241280000_01D_MN.rnx.gz).
The [IGS product description](https://igs.org/mgex/mgex-product-descriptions/)
identifies the DLR RINEX 4 product; [IGS MGEX](https://www.igs.org/mgex/)
states that its data and products are freely available for public use and requests citation for publications. The source header includes its DOI. Compressed source SHA-256:
`ec027e4a167d3d08c17acfc3001fdf0cbbc723a8416fb5684b1d95b66291926f`;
fixture SHA-256:
`d0570dcc33768d805d57b7a8bf4b928be81164b3188d84dcd4e4b762491a81f5`.
RINEX fixed-width padding is retained.

`tests/reference/nav_galileo_fnav_frame.py` reads the F/NAV 19-character
slots directly, uses the established independent Python Galileo Kepler
calculation in `nav_legacy.py`, then reuses the declared frame arithmetic
from `nav_galileo_frame.py`. It does not invoke Rust. Script SHA-256:
`8018fda933f7a67dfcbbd9380a7295465931d329d137eed9ece633a7a08e2899`;
expected JSON SHA-256:
`feef4f9a56c5349c79aa3e43891d3a87e84d9fc99d6f65e54961fe5fb46c2075`.
At ToE, native XYZ is
`[12690.603521848381, -12206.141111492765, 23797.46857761044] km`;
the declared target XYZ is
`[12690.603515118328, -12206.141108200894, 23797.46857088492] km`.
The two other rows are ToE ±300 s. The per-component comparison tolerance
is `1e-6 km`, matching the existing independent broadcast reference. Numeric
agreement checks the stated arithmetic and message route, not the physical
error of the zero-offset approximation.

From the rinex repository root:

```sh
python3 tests/reference/nav_galileo_fnav_frame.py > /private/tmp/nav_galileo_fnav_frame_expected_check.json
diff -u tests/reference/nav_galileo_fnav_frame_expected.json /private/tmp/nav_galileo_fnav_frame_expected_check.json
cargo test --offline --features nav,log --test nav_galileo_fnav_frame -- --nocapture
cargo run --offline --features nav --example nav_frame -- tests/fixtures/nav_galileo_e08_fnav_2024128.rnx E08 '2024-05-07T00:40:00 GST'
```

The CLI should report `msgtype: FNAV`, `GalileoGtrf23v01`, `Itrf2014`,
the two ordered edge IDs, `UnboundedApproximate`, `CAUTION`, and unavailable
target velocity. In VS Code, use Rust Analyzer **Debug Test** on
`real_e08_fnav_reaches_marked_itrf2014_at_its_own_epoch` in
`tests/nav_galileo_fnav_frame.rs`. Break in `NavCandidate::spatial_state_at`
at the realization branch and `FrameTransformer::to_frame` at the Galileo
path. Inspect `key.msgtype`, `record_epoch`, `orbit_reference`, requested
`epoch`, `source_evidence`, `edge_ids`, and `position_km`. Terminal checks do
not establish breakpoint operation or scientific frame accuracy.
