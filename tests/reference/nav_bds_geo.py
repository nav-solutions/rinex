"""Independent, fixed-record BeiDou D2 GEO position reference.

The GEO transformation follows RTKLIB eph2pos (see NAV_BDS_GEO.md).
This script reads only the RINEX fixture, never the Rust implementation.
"""
import json
import math
from pathlib import Path

SOURCE = Path(__file__).resolve().parents[1] / "fixtures/nav_bds_geo_c05_2022159.rnx"
MU = 3.986004418e14
EARTH_RATE = 7.292115e-5


def fields():
    lines = SOURCE.read_text().splitlines()
    i = lines.index("> EPH C05 D2")
    slots = []
    for line in lines[i + 2:i + 9]:
        slots.extend(
            float(line[4 + 19 * j:23 + 19 * j])
            if line[4 + 19 * j:23 + 19 * j].strip() else None
            for j in range(4)
        )
    assert len(slots) == 28
    return slots


def position(f, tk):
    crs, dn, m0, cuc, ecc, cus, sqrt_a = f[1:8]
    toe, cic, node0, cis, inc0, crc, arg_perigee, node_rate, inc_rate = f[8:17]
    a = sqrt_a * sqrt_a
    mean = m0 + (math.sqrt(MU / a**3) + dn) * tk
    anomaly = mean
    for _ in range(30):
        delta = (anomaly - ecc * math.sin(anomaly) - mean) / (1 - ecc * math.cos(anomaly))
        anomaly -= delta
        if abs(delta) < 1e-14:
            break
    else:
        raise ValueError("Kepler did not converge")
    true = math.atan2(math.sqrt(1 - ecc * ecc) * math.sin(anomaly),
                      math.cos(anomaly) - ecc)
    phi = true + arg_perigee
    twice_sin, twice_cos = math.sin(2 * phi), math.cos(2 * phi)
    u = phi + cus * twice_sin + cuc * twice_cos
    r = a * (1 - ecc * math.cos(anomaly)) + crs * twice_sin + crc * twice_cos
    inc = inc0 + inc_rate * tk + cis * twice_sin + cic * twice_cos
    node = node0 + node_rate * tk - EARTH_RATE * toe
    x, y = r * math.cos(u), r * math.sin(u)
    xg = x * math.cos(node) - y * math.cos(inc) * math.sin(node)
    yg = x * math.sin(node) + y * math.cos(inc) * math.cos(node)
    zg = y * math.sin(inc)
    # RTKLIB's SIN_5 is sin(-5 deg); expanded here as Rx(+5 deg).
    tilt = math.radians(5)
    tilted_y = yg * math.cos(tilt) - zg * math.sin(tilt)
    tilted_z = yg * math.sin(tilt) + zg * math.cos(tilt)
    theta = EARTH_RATE * tk
    return [(xg * math.cos(theta) + tilted_y * math.sin(theta)) / 1000,
            (-xg * math.sin(theta) + tilted_y * math.cos(theta)) / 1000,
            tilted_z / 1000]


def main():
    f = fields()
    result = []
    h = 0.05
    for offset in (-300, 0, 300, 1800):
        p = position(f, offset)
        before, after = position(f, offset - h), position(f, offset + h)
        v = [(a - b) / (2 * h) for a, b in zip(after, before)]
        result.append({"offset_s": offset, "position_km": p, "velocity_km_s": v})
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
