"""Independent RINEX 4 broadcast reference, following RTKLIB eph2pos.

Reads the five unmodified KMS300DNK records in tests/fixtures. This script
does not import rinex code or its orbit descriptor. The first 17 continuation
slots have the traditional broadcast Kepler layout for these four messages.
"""
import datetime as dt
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "tests/fixtures/nav_legacy_kms_2022159.rnx"
MESSAGES = ("G02 LNAV", "E08 INAV", "E08 FNAV", "C10 D1", "C20 D1")
CONSTANTS = {
    "G": (3.9860050e14, 7.2921151467e-5),
    "E": (3.986004418e14, 7.2921151467e-5),
    "C": (3.986004418e14, 7.292115e-5),
}


def records():
    lines = SOURCE.read_text().splitlines()
    result = {}
    for i, line in enumerate(lines):
        if not line.startswith("> EPH "):
            continue
        ident = line[6:]
        if ident not in MESSAGES:
            continue
        first = lines[i + 1]
        toc = dt.datetime.strptime(first[4:23].strip(), "%Y %m %d %H %M %S")
        clock = [float(first[23 + 19 * j:42 + 19 * j]) for j in range(3)]
        slots = []
        for row in lines[i + 2:i + 9]:
            slots.extend(float(row[4 + 19 * j:23 + 19 * j])
                         if row[4 + 19 * j:23 + 19 * j].strip() else None
                         for j in range(4))
        result[ident] = (toc, clock, slots)
    assert set(result) == set(MESSAGES)
    return result


def position(fields, constellation, tk):
    mu, earth_rate = CONSTANTS[constellation]
    crs, dn, m0, cuc, eccentricity, cus, sqrt_a = fields[1:8]
    toe, cic, omega0, cis, i0, crc, omega, omega_dot, i_dot = fields[8:17]
    a = sqrt_a * sqrt_a
    mean_anomaly = m0 + (math.sqrt(mu / a**3) + dn) * tk
    eccentric_anomaly = mean_anomaly
    for _ in range(30):
        change = (eccentric_anomaly - eccentricity * math.sin(eccentric_anomaly)
                  - mean_anomaly) / (1 - eccentricity * math.cos(eccentric_anomaly))
        eccentric_anomaly -= change
        if abs(change) < 1e-14:
            break
    else:
        raise ValueError("Kepler did not converge")
    true_anomaly = math.atan2(math.sqrt(1 - eccentricity**2)
                              * math.sin(eccentric_anomaly),
                              math.cos(eccentric_anomaly) - eccentricity)
    phi = true_anomaly + omega
    harmonic_sin, harmonic_cos = math.sin(2 * phi), math.cos(2 * phi)
    u = phi + cus * harmonic_sin + cuc * harmonic_cos
    r = a * (1 - eccentricity * math.cos(eccentric_anomaly)) \
        + crs * harmonic_sin + crc * harmonic_cos
    inclination = i0 + i_dot * tk + cis * harmonic_sin + cic * harmonic_cos
    node = omega0 + (omega_dot - earth_rate) * tk - earth_rate * toe
    x, y = r * math.cos(u), r * math.sin(u)
    return [
        (x * math.cos(node) - y * math.cos(inclination) * math.sin(node)) / 1000,
        (x * math.sin(node) + y * math.cos(inclination) * math.cos(node)) / 1000,
        y * math.sin(inclination) / 1000,
    ]


def reference(ident, toc, clock, fields, offset):
    constellation = ident[0]
    toe = fields[8]
    toc_sow = ((toc.weekday() + 1) % 7) * 86400 + toc.hour * 3600 \
        + toc.minute * 60 + toc.second
    # The NAV week and ToE are in the message system time; RINEX Galileo
    # weeks use the GPS week alignment. These records do not cross a leap.
    dt_clock = toe - toc_sow + offset
    h = 0.05
    p = position(fields, constellation, offset)
    before = position(fields, constellation, offset - h)
    after = position(fields, constellation, offset + h)
    velocity = [(a - b) / (2 * h) for a, b in zip(after, before)]
    a0, a1, a2 = clock
    e = fields[5]
    sqrt_a = fields[7]
    mu = CONSTANTS[constellation][0]
    m = fields[3] + (math.sqrt(mu / (sqrt_a * sqrt_a)**3) + fields[2]) * offset
    anomaly = m
    for _ in range(30):
        change = (anomaly - e * math.sin(anomaly) - m) / (1 - e * math.cos(anomaly))
        anomaly -= change
        if abs(change) < 1e-14:
            break
    relativity = -2 * math.sqrt(mu) / 299792458.0**2 * e * sqrt_a * math.sin(anomaly)
    return {
        "offset_s": offset,
        "position_km": p,
        "velocity_km_s": velocity,
        "clock_s_without_group_delay": a0 + a1 * dt_clock + a2 * dt_clock**2 + relativity,
    }


def main():
    data = records()
    result = {}
    for ident in MESSAGES:
        toc, clock, fields = data[ident]
        result[ident] = [reference(ident, toc, clock, fields, offset)
                         for offset in (-300, 0, 300)]
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
