"""Independent R02 propagation and PZ-90.11 -> ITRF2014 -> ITRF2020 -> G2296.

Reads fixed-width RINEX fields, implements the GLONASS J2 RK4 model in Python,
then applies published frame parameters. Does not call the Rust library.
"""

from datetime import datetime, timedelta, timezone
from hashlib import sha256
import json
from math import pi
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/nav_mixed_2024131_first_epoch.rnx"
FIXTURE_SHA256 = "dce340cf859c06b7785596f379f1bbf39f8042042629309adc9fbdc2f322c69e"
RECORD_PREFIX = "R02 2024 05 10 02 45 00"
QUERY_GPST_LABEL = datetime(2024, 5, 10, 3, 0, 0, tzinfo=timezone.utc)


def main() -> None:
    raw = FIXTURE.read_bytes()
    assert sha256(raw).hexdigest() == FIXTURE_SHA256
    lines = raw.decode("ascii").splitlines()
    header = lines[:next(i for i, line in enumerate(lines) if "END OF HEADER" in line) + 1]
    leap_lines = [line for line in header if "LEAP SECONDS" in line]
    assert len(leap_lines) == 1
    gpst_minus_utc = int(leap_lines[0][:6])
    assert gpst_minus_utc == 18
    query_utc = QUERY_GPST_LABEL - timedelta(seconds=gpst_minus_utc)
    matches = [i for i, line in enumerate(lines) if line.startswith(RECORD_PREFIX)]
    assert len(matches) == 1
    row = matches[0]
    assert lines[row - 1].startswith("> EPH R02 FDMA")
    record = lines[row:row + 5]
    toc = datetime(*map(int, record[0][:23].split()[1:]), tzinfo=timezone.utc)
    dt_s = (query_utc - toc).total_seconds()
    assert dt_s == 882.0
    rows = [
        [float(record[i][4 + 19 * j:23 + 19 * j].replace("D", "E")) for j in range(4)]
        for i in (1, 2, 3)
    ]
    state = [item[0] for item in rows] + [item[1] for item in rows]
    acceleration = [item[2] for item in rows]

    def derivative(q: list[float]) -> list[float]:
        x, y, z, vx, vy, vz = q
        r2 = x*x + y*y + z*z
        r3 = r2 * r2**0.5
        mu, radius, j2, omega = 398600.44, 6378.136, 1.0826257e-3, 7.292115e-5
        j = 1.5 * j2 * mu * radius**2 / (r2 * r3)
        central = -mu / r3 - j * (1 - 5*z*z/r2)
        return [vx, vy, vz,
                (central + omega**2)*x + 2*omega*vy + acceleration[0],
                (central + omega**2)*y - 2*omega*vx + acceleration[1],
                (central - 2*j)*z + acceleration[2]]

    remaining = dt_s
    while remaining > 0:
        h = min(60.0, remaining)
        k1 = derivative(state)
        k2 = derivative([q + h*k/2 for q, k in zip(state, k1)])
        k3 = derivative([q + h*k/2 for q, k in zip(state, k2)])
        k4 = derivative([q + h*k for q, k in zip(state, k3)])
        state = [q + h*(a + 2*b + 2*c + d)/6
                 for q, a, b, c, d in zip(state, k1, k2, k3, k4)]
        remaining -= h
    native_km = state[:3]

    # ICG-13, PZ-90.11 to ITRF2014 at 2010.0, frozen at query epoch.
    metres = [1000*p for p in native_km]
    mas = pi / (180.0 * 3600.0 * 1000.0)
    rx, ry, rz = [value * mas for value in (0.035, -0.087, 0.036)]
    rotation = ((0, rz, -ry), (-rz, 0, rx), (ry, -rx, 0))
    translation_m = (-0.0053, -0.0040, -0.0032)
    itrf2014_km = [
        (metres[i] + sum(rotation[i][j]*metres[j] for j in range(3)) + translation_m[i])/1000
        for i in range(3)
    ]

    # IERS ITRF2020 Table 2 gives ITRF2014 minus ITRF2020 at 2015.0.
    years = (query_utc - datetime(2015, 1, 1, tzinfo=timezone.utc)).total_seconds() / 31557600.0
    translation_mm = (-1.4, -0.9 - 0.1*years, 1.4 + 0.2*years)
    scale = 1.0 - 0.42e-9
    itrf2020_km = [(itrf2014_km[i] - translation_mm[i]*1e-6)/scale for i in range(3)]

    # EPSG:10608 seven zero parameters: inverse ITRF2020 -> G2296 at 2024 epoch.
    g2296_km = itrf2020_km.copy()
    print(json.dumps({
        "fixture_sha256": FIXTURE_SHA256,
        "sv": "R02",
        "record_epoch_utc": toc.isoformat().replace("+00:00", "Z"),
        "query_epoch_gpst": "2024-05-10T03:00:00 GPST",
        "query_epoch_utc": query_utc.isoformat().replace("+00:00", "Z"),
        "propagation_seconds": dt_s,
        "native_position_km": native_km,
        "native_velocity_km_s": state[3:],
        "approx_itrf2014_position_km": itrf2014_km,
        "approx_itrf2020_position_km": itrf2020_km,
        "approx_g2296_position_km": g2296_km,
        "itrf_years_since_2015": years,
        "itrf_translation_mm": translation_mm,
        "itrf_scale_ppb": -0.42,
        "pz_parameter_reference_epoch": "2010.0",
        "status": "MarkedApproximation; no strict satellite-position or epoch error bound",
    }, indent=2))


if __name__ == "__main__":
    main()
