"""Independent inverse of the published 2010.0 PZ-90.11 -> ITRF2014 row.

Input ITRF2014 XYZ comes from the independently propagated R01 reference.
Solve the three forward equations by Gaussian elimination, without calling Rust.
"""

import json
from math import pi
from pathlib import Path


REFERENCE = Path(__file__).with_name("nav_glonass_frame_approx_expected.json")


def main() -> None:
    data = json.loads(REFERENCE.read_text())
    mas = pi / (180.0 * 3600.0 * 1000.0)
    rx, ry, rz = [v * mas for v in (0.035, -0.087, 0.036)]
    translation = (-0.0053, -0.0040, -0.0032)
    target = data["approx_itrf2014_position_km"]
    rows = [
        [1.0, rz, -ry, 1000.0 * target[0] - translation[0]],
        [-rz, 1.0, rx, 1000.0 * target[1] - translation[1]],
        [ry, -rx, 1.0, 1000.0 * target[2] - translation[2]],
    ]
    for pivot in range(3):
        scale = rows[pivot][pivot]
        rows[pivot] = [v / scale for v in rows[pivot]]
        for index in range(3):
            if index != pivot:
                factor = rows[index][pivot]
                rows[index] = [a - factor * b for a, b in zip(rows[index], rows[pivot])]
    recovered = [row[3] / 1000.0 for row in rows]
    assert all(abs(a - b) < 1e-9 for a, b in zip(recovered, data["native_position_km"]))
    print(json.dumps({
        "reference_fixture_sha256": data["fixture_sha256"],
        "input_itrf2014_km": target,
        "inverse_pz9011_km": recovered,
        "original_native_pz9011_km": data["native_position_km"],
        "parameter_reference_epoch": "2010.0",
    }, indent=2))


if __name__ == "__main__":
    main()
