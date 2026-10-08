"""Independent G15 broadcast position and G2296 -> ITRF2014 reference.

The RINEX record is copied from NYA100NOR_S_20241280000_01D_GN.rnx.gz.
The broadcast propagation reuses the separately checked Python Kepler reference.
Frame parameters are read from NGA/EPSG 10608 and IERS ITRF2020 Table 2;
this script never imports or calls the Rust frame transformation.
"""

import datetime as dt
import hashlib
import json
from pathlib import Path

from nav_legacy import position

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/nav_gps_g15_g2296_2024128.rnx"


def reference():
    raw = FIXTURE.read_bytes()
    lines = raw.decode("ascii").splitlines()
    first = next(i for i, line in enumerate(lines) if line.startswith("G15 2024 05 07 02 00 00"))
    assert len(lines) == first + 8
    fields = [
        float(line[4 + 19 * column:23 + 19 * column])
        for line in lines[first + 1:first + 8]
        for column in range(4)
        if line[4 + 19 * column:23 + 19 * column].strip()
    ]
    assert fields[8] == 180000.0  # ToE, GPST seconds of week
    native_km = position(fields, "G", 300.0)

    # G2296 -> ITRF2020: seven zero parameters by NGA design (EPSG 10608).
    # ITRF2020 -> ITRF2014: IERS ITRF2020 Table 2, epoch 2015.0.
    # T in mm, Tdot in mm/Julian year; D = -0.42 ppb, rotations/rates = 0.
    epoch_utc = dt.datetime(2024, 5, 7, 2, 4, 42)
    years = (epoch_utc - dt.datetime(2015, 1, 1)).total_seconds() / 31557600.0
    translation_mm = [-1.4, -0.9 - 0.1 * years, 1.4 + 0.2 * years]
    target_km = [
        (1.0 - 0.42e-9) * xyz + t_mm * 1e-6
        for xyz, t_mm in zip(native_km, translation_mm)
    ]
    direct_km = [10000.0, 20000.0, 15000.0]
    direct_target_km = [
        (1.0 - 0.42e-9) * xyz + t_mm * 1e-6
        for xyz, t_mm in zip(direct_km, translation_mm)
    ]
    return {
        "fixture_sha256": hashlib.sha256(raw).hexdigest(),
        "source_file": "data/NAV/V3/NYA100NOR_S_20241280000_01D_GN.rnx.gz",
        "sv": "G15",
        "coordinate_epoch": "2024-05-07T02:05:00 GPST",
        "years_since_2015": years,
        "native_position_km": native_km,
        "itrf2014_position_km": target_km,
        "direct_itrf2020_position_km": direct_km,
        "direct_itrf2014_position_km": direct_target_km,
        "translation_mm": translation_mm,
        "scale_ppb": -0.42,
        "itrf2020_parameter_source": "https://itrf.ign.fr/en/solutions/itrf2020",
        "g2296_parameter_source": "https://epsg.io/10608",
    }


if __name__ == "__main__":
    print(json.dumps(reference(), indent=2))
