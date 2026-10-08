# GLONASS FDMA R01 reference

From the repository root, run:

```sh
python3 tests/reference/glonass_fdma_r01.py > /tmp/glonass_fdma_r01.json
diff -u tests/reference/glonass_fdma_r01.json /tmp/glonass_fdma_r01.json
cargo test --features nav --test nav_glonass_fdma -- --nocapture
```

The input is the R01 FDMA record in `data/NAV/V4/rinex402_examples_MN.rnx`, SHA-256 `ba4b549010bc97eaf61c6102a09052668d3b29c423688f6bc5b4b30f7e433442`. It is a public repository fixture, not a synthetic orbit. The independent Python reference reads the fixed-width RINEX fields directly and does not invoke this Rust library. It uses the PZ-90 rotating-Earth GLONASS equations and 60 s fourth-order Runge-Kutta steps described in [ESA Navipedia](https://gssc.esa.int/navipedia/index.php/GLONASS_Satellite_Coordinates_Computation), cross-checked against `deq` and `glorbit` in [RTKLIB `ephemeris.c` at full commit `180043ee24b6d2b168f98b64be15f69d50046b1a`](https://github.com/tomojitakasu/RTKLIB/blob/180043ee24b6d2b168f98b64be15f69d50046b1a/src/ephemeris.c). The local Python reference, rather than the RTKLIB executable, produced the checked-in numbers. Its SHA-256 at N04 validation was `ec392f7332dd71201f2e75ac8230ebcaf7a8cb597ca69d9d108e76af5193b077`; the script in this tree is the reproducible reference.

The model uses `mu=398600.44 km³/s²`, `ae=6378.136 km`, `J2=1.0826257e-3`, `omega=7.292115e-5 rad/s`, and the three broadcast perturbations in km/s². Position and velocity are respectively km and the derivative in the PZ-90 rotating axes in km/s. The RINEX epoch is UTC; the UTC(SU) proxy error and PZ-90 realization are not resolved by this record. The interpolation is limited to `|t-t_ref|<900 s`, following the ESA/GLONASS 15-minute integration interval; this is an algorithmic validity window, not a guarantee of metre-level physical accuracy. Clock results use the stored `-TauN` and `+GammaN` relative to the record epoch; `TauC` and signal-dependent delays are outside this result.

JSON states round each component to 12 decimal places. The Rust test permits `1e-8 km` per position component and `1e-10 km/s` per velocity component: these exceed serialization and floating-point reorder noise while remaining far below plausible orbit-model errors. The observed largest differences were `1.819e-12 km` and `4.379e-13 km/s`. At `+899.5 s`, the 60/30 and 30/15 s position differences were `5.752e-7 km` and `3.598e-8 km`. These numerical checks do not establish absolute orbit accuracy against precise ephemerides.
