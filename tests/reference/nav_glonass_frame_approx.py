"""Independent arithmetic check for the marked PZ-90.11 -> ITRF2014 approximation.

Reads the R01 broadcast state directly from RINEX slots, propagates it to
record epoch +300 s, and never calls the Rust propagator or converter. The
2010.0 frame parameters are applied unchanged at that 2020 point epoch by
explicit approximation policy.
"""

from datetime import datetime, timedelta, timezone
from hashlib import sha256
import json
from math import pi
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/nav_glonass_r01_pz9011_2020176.rnx"
FIXTURE_SHA256 = "b456dd8d83756a5b6108495c5a7b334d599edcbadd9ef944daa2b9cd6caaec52"


def main() -> None:
    raw = FIXTURE.read_bytes()
    assert sha256(raw).hexdigest() == FIXTURE_SHA256
    lines = raw.decode("ascii").splitlines()
    row = next(i for i, line in enumerate(lines) if line.startswith("R01 "))
    record = lines[row : row + 5]
    date = [int(part) for part in record[0][:23].split()[1:]]
    record_epoch = datetime(*date, tzinfo=timezone.utc)
    rows = [
        [float(record[i][4 + 19 * j : 23 + 19 * j].replace("D", "E")) for j in range(4)]
        for i in (1, 2, 3)
    ]
    state = [row[0] for row in rows] + [row[1] for row in rows]
    acceleration = [row[2] for row in rows]

    # Independent RK4 evaluation of the GLONASS ICD J2 rotating-frame model.
    def derivative(q: list[float]) -> list[float]:
        x, y, z, vx, vy, vz = q
        r2 = x * x + y * y + z * z
        r3 = r2 * r2**0.5
        mu, radius, j2, omega = 398600.44, 6378.136, 1.0826257e-3, 7.292115e-5
        j = 1.5 * j2 * mu * radius**2 / (r2 * r3)
        central = -mu / r3 - j * (1 - 5 * z * z / r2)
        return [
            vx, vy, vz,
            (central + omega**2) * x + 2 * omega * vy + acceleration[0],
            (central + omega**2) * y - 2 * omega * vx + acceleration[1],
            (central - 2 * j) * z + acceleration[2],
        ]

    for _ in range(5):  # record epoch + 300 s, with 60 s maximum steps
        h = 60.0
        k1 = derivative(state)
        k2 = derivative([q + h * k / 2 for q, k in zip(state, k1)])
        k3 = derivative([q + h * k / 2 for q, k in zip(state, k2)])
        k4 = derivative([q + h * k for q, k in zip(state, k3)])
        state = [q + h * (a + 2 * b + 2 * c + d) / 6 for q, a, b, c, d in zip(state, k1, k2, k3, k4)]
    native_km = state[:3]
    epoch = (record_epoch + timedelta(seconds=300)).isoformat().replace("+00:00", "Z")

    # ICG-13 wgd_24.pdf, pp. 5 and 8: coordinate-frame rotation (not
    # position-vector rotation), metres, milliarcseconds, scale 10^-6.
    metres = [1000.0 * p for p in native_km]
    mas = pi / (180.0 * 3600.0 * 1000.0)
    rx, ry, rz = [value * mas for value in (0.035, -0.087, 0.036)]
    rotation = ((0.0, rz, -ry), (-rz, 0.0, rx), (ry, -rx, 0.0))
    translation_m = (-0.0053, -0.0040, -0.0032)
    target_km = [
        (metres[i] + sum(rotation[i][j] * metres[j] for j in range(3)) + translation_m[i])
        / 1000.0
        for i in range(3)
    ]
    result = {
        "fixture_sha256": FIXTURE_SHA256,
        "sv": "R01",
        "record_epoch_utc": record_epoch.isoformat().replace("+00:00", "Z"),
        "epoch_utc": epoch,
        "native_position_km": native_km,
        "native_velocity_km_s": state[3:],
        "approx_itrf2014_position_km": target_km,
        "parameter_reference_epoch": "2010.0",
        "method": "unbounded approximate static use of 2010.0 station-fit parameters",
    }
    print(json.dumps(result, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
