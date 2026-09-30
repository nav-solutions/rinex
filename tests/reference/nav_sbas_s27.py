"""Independent SBAS MT9 polynomial reference for the S27 RINEX fixture.

Input fields are read by RINEX's 19-column layout, without using rinex code.
Run from repository root: python3 tests/reference/nav_sbas_s27.py
"""
import json
from pathlib import Path

path = Path("tests/fixtures/nav_sbas_s27_2023071.rnx")
lines = path.read_text().splitlines()
start = lines.index("> EPH S27 SBAS")
clock, xline, yline, zline = lines[start + 1 : start + 5]
field = lambda line, prefix, slot: float(line[prefix + 19 * slot : prefix + 19 * (slot + 1)])
a0, a1, t_tm = (field(clock, 23, i) for i in range(3))
axes = [[field(line, 4, i) for i in range(3)] for line in (xline, yline, zline)]
rows = []
for dt in (-300, -120, 0, 120, 300):
    rows.append({
        "dt_s": dt,
        "position_km": [p + v * dt + a * dt * dt / 2 for p, v, a in axes],
        "velocity_km_s": [v + a * dt for p, v, a in axes],
        "clock_correction_s": a0 + a1 * dt,
    })
print(json.dumps({"t_tm_s": t_tm, "rows": rows}, indent=2))
